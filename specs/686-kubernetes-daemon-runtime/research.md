# Research: Kubernetes session daemon runtime (#686)

Each decision names what was chosen, why, and what was rejected. The image source (K1, K2, K3) and
the clone source (K8) are deliberately self-contained: FR-004 and FR-007 run on orchestrator
defaults pending the owner's decision cards (ledger Decisions 1-3), and a different answer replaces
only those sections and the modules they name (`micold-core/src/daemon_image/`,
`micold-core/src/kube/clone.rs`, the `RepoClone` op), not the pod, tunnel or lifecycle design.

## K0. Talking to the cluster: the `kube` crate in the client, a port in core

**Decision**: the client speaks to the Kubernetes API with `kube` 4.2 (`client`, `config`,
`rustls-tls`, `ring`, `ws` features; Apache-2.0, kube-rs is a CNCF sandbox project, MSRV 1.89 <
workspace 1.97) and `k8s-openapi` 0.28 (feature `v1_32`, the oldest supported API level; Apache-2.0).
Both are dependencies of `micold-client` only. `micold-core` gains a port, `kube::Cluster` (async
trait-free: a trait returning boxed futures, like `ssh::SshRunner`), whose operations take and return
plain data (`serde_json::Value` manifests, `PodStatusView`, `ApiFailure`); the real implementation,
`micold-client/src/kube/live.rs` `LiveCluster`, is chosen once in `shell/capabilities.rs`, and a
`FakeCluster` in core drives every sequence test. `ring` matches the workspace's existing rustls
backend choice (`ureq`, feature 533): no C/NASM toolchain on Windows.

**Why**: `kube` reads kubeconfig (merged `KUBECONFIG` list, contexts, exec credential plugins, client
certificates, OIDC) exactly as `kubectl` does, and its `ws` feature gives the API's port-forward as an
in-process `AsyncRead + AsyncWrite` stream, so no local listener exists at all (FR-006 is stronger
than "loopback only"). The user needs no `kubectl` installed, which the spec's Assumptions do not
promise.

`sha2` 0.10 (RustCrypto, MIT/Apache-2.0, pure Rust, no default features needed) is added to
`micold-core` for the daemon key (K5) and the image-inputs hash (K2).

**Rejected**: shelling out to `kubectl` (an unstated runtime requirement; `kubectl port-forward`
opens a loopback listener any local process can use; parsing its human output for failure reasons is
fragile); a hand-written REST client over `ureq` (re-implements exec plugins, auth providers,
SPDY/WebSocket port-forward); `kube` in `micold-core` (pulls hyper, tower and rustls into the
render-free crate the daemon also compiles; the core keeps only rules).

## K1. Image source model: one shared `DaemonImageSource`, introduced here for #806

**Decision**: #806 is open and unimplemented (no `specs/806-*`, container daemons still use
`sandbox::ImageSource` with `Registry | ImportedFile | LocalBuild`). This feature introduces the
shared model #806 will adopt, in a new module `micold-core/src/daemon_image/`:
`DaemonImageSource::Build { dockerfile: Option<PathBuf>, providers: BTreeSet<AiCli> }` (no
Dockerfile = the app's own template) and `DaemonImageSource::Pull { reference: ImageRef }`, plus
`ResourceBudget` reused unchanged from `sandbox` (#806's resources fields). The Kubernetes settings add
`registry: Option<RegistryRepo>` beside it (FR-004: the build choice needs a registry). The Settings
form for the source and resources is one shared form section (`ui/settings/daemon_image.rs`, new)
consumed by the Kubernetes form now and the container form when #806 lands.

**Why**: the owner asked for one shared form (spec Overview). Building it here, isolated, means #806
reuses it rather than reconciling two models; if #806 lands first with its own type, the Kubernetes
settings hold that type instead and only `kube/image.rs` changes.

**Rejected**: reusing `sandbox::ImageSource` (its default is the published release image FR-004
forbids, and `ImportedFile` means nothing to a cluster); waiting for #806 (blocks this feature on an
unscheduled one; the owner's comment asked for alignment, not ordering).

## K2. Build inputs, tag and rebuild rule (#806's rule, deferred to the plan by clarify)

**Decision**:
- **Daemon binary**: a Linux build of this client's daemon for the chosen node architecture
  (`KubeSettings.arch`, `amd64` default, or `arm64`), selected by the 489 artefact rule
  (`ssh::provision::ArtefactChoice`): the client's own binary on a Linux client of that
  architecture, else the bundled `<install>/remote-daemons/linux-<arch>/micold-daemon` (489 R3).
- **Containerfile**: with no Dockerfile named, `daemon_image::containerfile::render(providers)`
  renders the app's template, derived from `packaging/sandbox/Containerfile` (same base, same pinned
  CLI versions read from its `ARG` lines at compile time with `include_str!`), installing only the
  selected providers' CLIs, running as UID 10001, with `HOME=/data/home`, `XDG_DATA_HOME=/data/state`,
  `TMPDIR=/tmp`. With a Dockerfile named, its directory is the build context and the daemon binary is
  passed as the named build context `micold` (`--build-context micold=<tmp>`, BuildKit; docker ≥ 23,
  podman ≥ 4.1); the user's Dockerfile copies it with `COPY --from=micold micold-daemon …`.
- **Tag**: `<registry>/micold-daemon:<CARGO_PKG_VERSION>-<inputs>` where `<inputs>` is the first 12
  hex of SHA-256 over (daemon binary bytes, rendered or user Containerfile bytes, sorted provider
  list, `linux/<arch>`). Never a moving tag (`sandbox::image::MOVING_TAGS` check reused).
- **Rebuild rule**: no rebuild on edit (story 5.2: settings apply on next start). On start, if the
  local runtime has no image with that tag, build; then always push (a push of present layers
  transfers nothing) and read the pushed digest. A changed provider set, Dockerfile, architecture or
  client version changes the tag, so the next start rebuilds and pushes; nothing else does.
- **Pod reference**: the image by digest (`<registry>/micold-daemon@sha256:…`), so the pod runs
  exactly what was pushed even if the tag is re-pushed by another client.

**Why**: the version in the tag plus the digest reference make FR-017's mismatch impossible on the
build path; the hash makes "selected CLIs changed" a rebuild without any stored state. Both are pure
functions, testable in core.

**Rejected**: rebuild on every start (minutes per start, SC-001); rebuild on edit (an edit while the
cluster is unreachable would fail the form); a stored "last built" record (stale across machines that
share the daemon, spec 489 FR-021); `:latest` or version-only tags (a provider change would reuse a
wrong image).

## K3. Build, push and their bounds

**Decision**: build and push run through the local container runtime CLI already wrapped by
`sandbox/cli.rs` (`ContainerCli`, docker or podman per the sandbox's `RuntimeKind`), gaining
`build`, `push` and `repo_digest`. Output streams line by line into the daemon's diagnostics log
(`DaemonDiagnostics::push_log`) and the row shows "Building image" / "Pushing image" with **Cancel**
(kills the child). The phase fails at 30 minutes from its start (FR-012). Push authenticates with the
runtime's own login; a refusal is classified (`not logged in`, `denied`, `unreachable`) naming the
registry and the runtime's stderr (edge case "Registry push refused"). No runtime installed with the
build choice: start fails with "a container runtime is needed to build; pulling a given image needs
none".

**Rejected**: an in-process image builder (`buildkit` client, kaniko in-cluster): the first needs a
daemon anyway, the second needs cluster privileges the spec's Assumptions do not grant.

## K4. Daemon authentication inside the pod: loopback-only listener, no token

**Decision**: the pod sets `MICOLD_LISTEN_ADDR=127.0.0.1:7727` and `MICOLD_TOKEN_PATH=` (empty). The
daemon gains one rule (`micold-core/src/protocol/auth.rs`, `listener_auth(addr, token_env)`): an
empty token path is accepted **only** with a loopback listen address, and means "no token"; an empty
path with a non-loopback address stays fatal (feature 027's rule unchanged). The API's port-forward
dials the pod's own loopback, so the only way to the daemon is an identity that the cluster grants
`pods/portforward` in that namespace — the same identity that could `exec` into the pod.

**Why**: a token would have to reach the pod as a Kubernetes secret, which FR-009 and SC-004 forbid
(0 secrets beyond those the user allowed) and which adds nothing: anyone who can read that secret can
port-forward. Other pods cannot reach a pod's loopback.

**Rejected**: a token secret (violates FR-009/SC-004); the token as an env var in the pod spec
(readable by anyone who can `get pods`); binding `0.0.0.0` (reachable from every pod in the cluster
when no NetworkPolicy exists).

## K5. Resource identity, ownership and races (FR-003, FR-016, FR-023)

**Decision**: every resource is named deterministically from the daemon key
`key = sha256(context ‖ 0 ‖ namespace ‖ 0 ‖ daemon name)[..8 hex]`:
`micold-<slug(name)>-<key>` (pod), `…-data` (PVC), `…-git-credentials`, `…-git-ssh`, `…-gitconfig`,
`…-signin-<cli>`, `…-pull` (secrets). `slug` is DNS-1123, at most 40 characters. Every resource
carries labels `app.kubernetes.io/managed-by=micold-ai-ide` and `micold.dev/daemon=<key>` and the
annotation `micold.dev/daemon-name=<name>`. A resource is **ours** only when both labels match.
Create is `POST`; `409 AlreadyExists` → `GET`: ours → adopt (the race loser reuses the winner's pod,
FR-023), not ours → `Conflict { kind, name }` (edge case "Pod created by someone else"). The client
never lists or deletes by label selector alone; deletes name the resource and pass a precondition on
its UID read in the same step.

**Why**: names derived from the three identifying fields make two clients with the same daemon meet on
the same objects with no coordination (spec 489 FR-021), and the API's create-uniqueness is the lock.

**Rejected**: generated names with a label lookup (two racing clients both create); a Deployment or
StatefulSet (spec Assumption: bare pod; a controller would recreate a pod the user stopped).

## K6. Start sequence, phase bounds and failure classification (FR-012, FR-013, US4)

**Decision**: start is a fixed sequence of phases in `micold-core/src/kube/start.rs`, a pure state
machine over `Cluster` results, each with its bound: (1) `Config` load kubeconfig, context, exec
plugin — 120 s; (2) `Preflight` namespace `GET` and `SelfSubjectAccessReview` for `create pods`,
`create persistentvolumeclaims`, `create secrets` (only when some are allowed), `create pods/portforward`,
`delete pods` — 120 s, so RBAC denial is reported before anything is created (story 4.1); (3) `Image`
build and push — 30 min, live, cancellable (Pull choice skips it); (4) `Volume` ensure PVC — 120 s;
(5) `Secrets` reconcile — 120 s; (6) `Pod` create or adopt — 120 s; (7) `Schedule` until
`PodScheduled=True` — 120 s; (8) `Pull` until the container is no longer `ContainerCreating` with a
pull in progress — 15 min; (9) `Tunnel` port-forward and handshake — 120 s. Failure classes
(`KubeFailure`) come from pure classifiers over API status codes and pod/PVC conditions and events:
`Forbidden { verb, resource }` (403 or a denied review), `ImagePull { image, message }`
(`ErrImagePull`, `ImagePullBackOff`, `InvalidImageName` — reported on first sight, not after the
bound), `Unschedulable { message }` (`PodScheduled=False, reason=Unschedulable`), `ClaimNotBound
{ message }` (PVC `ProvisioningFailed` event, or the scheduler's "unbound immediate
PersistentVolumeClaims"), `ContextMissing`, `NamespaceMissing`, `Unreachable`, `ExecPlugin`,
`Build`, `Push`, `Conflict`, `PhaseTimeout { phase }`. A failed start deletes what this attempt
created (pod, secrets; the PVC only if this attempt created it), by name and UID (FR-013).

**Why**: a phase list with one bound each is the spec's FR-012 text made data; each classifier is a
table test.

**Rejected**: one overall timeout (an image pull legitimately takes minutes while RBAC denial must not
wait for it); watching with informers (a `GET` poll every second within a phase is enough for one pod
and much simpler to fake).

## K7. Pod shape and secrets (FR-006, FR-008, FR-009, US3)

**Decision**: one container, `restartPolicy: Always`, `automountServiceAccountToken: false` (else the
service-account token would be a secret the user did not allow), `enableServiceLinks: false`,
pod `securityContext { runAsNonRoot, runAsUser: 10001, runAsGroup: 10001, fsGroup: 10001,
seccompProfile: RuntimeDefault }`, container `securityContext { readOnlyRootFilesystem: true,
allowPrivilegeEscalation: false, capabilities.drop: [ALL] }`; volumes: the PVC at `/data`, an
`emptyDir` at `/tmp`; resources from `ResourceBudget`; `nodeSelector` = user's plus
`kubernetes.io/arch=<arch>` on the build choice; `tolerations` as given; env `MICOLD_LISTEN_ADDR`,
`MICOLD_TOKEN_PATH=` (K4), `MICOLD_IDLE_STOP=off` (the pod's lifetime is the user's Stop, not the
30-minute idle rule; else an idle exit would read as "pod gone"). Allowed secrets are mounted
read-only at fixed paths: `/run/micold/creds/git-credentials` (git `credential.helper store --file`
via `GIT_CONFIG_*` env), `/run/micold/creds/ssh/{id,known_hosts}` (`GIT_SSH_COMMAND` with
`IdentitiesOnly=yes`, `StrictHostKeyChecking=yes`, the user's known-hosts lines for the remote's host
only), `/data/home/.gitconfig` (subPath), each sign-in at its provider's `sandbox_auth_file` path
under `HOME` (subPath). Image-pull secret in `imagePullSecrets` only when allowed. Withdrawn secrets
(ours, not in the allow-list) are deleted in phase 5. No Service, Ingress, `hostPort`, `hostNetwork`
or `containerPort` is ever written; a manifest test asserts it.

**Credential kinds** (`KubeCredential`): `GitConfig`, `GitCredentials`, `GitSshKey { key file }`,
`SignIn(AiCli)`, `RegistryPull { auths file }`. Not `SshAgent`: an agent socket cannot reach a pod
through the API, so an SSH remote needs an explicitly chosen key file, read and sent only when
allowed.

**Rejected**: copying credentials onto the volume (a withdrawn credential would survive on disk,
breaking FR-008's "deleted on the next start"); forwarding the SSH agent over a second tunnel (new
protocol surface; out of proportion).

## K8. Clone source and worktree location (FR-007)

**Decision**: binding a worktree of local project P to a Kubernetes daemon resolves P's remote URL
(`origin`, else P's only remote; none → refused "<project> has no git remote; a Kubernetes daemon
clones from it"; several without `origin` → refused naming them), then asks the daemon to ensure a
clone with a new protocol request `RepoClone { url, dest }`, `dest = /data/repos/<slug(repo)>-<hash8(url)>`
(deterministic, so every client of the daemon meets the same clone). The daemon clones into
`/data/repos/.partial-<dest>-<rand>` with `GIT_TERMINAL_PROMPT=0` and renames on success, deleting
the partial directory on failure (edge case: nothing half-cloned), and fetches when `dest` exists.
Failures are classified (`AuthRequired { remote }`, `DiskFull`, `Unreachable { remote }`,
`Other`). The clone is then a remote project on that daemon with 489's identity
`micold-remote://<daemon-id>/data/repos/…` (489 R5), and the worktree is created in it with the
existing `WorktreeCreate` op; rows show the daemon name and in-pod path (story 1.2).
`PROTOCOL_VERSION` 48 → 49 for the new request.

**Why**: the remote is the only source the pod can reach without the client streaming the repository;
489's identity scheme already makes a non-local project safe for every path-keyed map.

**Rejected**: streaming a `git bundle` from the client through the tunnel (carries unpushed commits —
the spec explicitly drops them — and needs a new bulk-transfer path); cloning in an init container
(needs the remote URL in the pod spec and fails before the client can report which credential is
missing).

## K9. Kubeconfig discovery and exec plugins on every OS (FR-002, FR-022)

**Decision**: `micold-core/src/kube/kubeconfig.rs` computes the candidate files: `KUBECONFIG` split
with the platform separator (`;` Windows, `:` otherwise; a pure function taking the platform, like
`sandbox::pathmap::map_for`), else `<home>/.kube/config`, home being `%USERPROFILE%` on Windows and
`$HOME` otherwise. The client reads and merges them in order with `kube::config::Kubeconfig`, and
lists the contexts for the form; none found → the form says "no kubeconfig found; looked in <list>".
Exec plugins: before handing the config to `kube`, each `users[].exec.command` that is not absolute
is resolved to an absolute path against one search path computed the same way however the app was
launched: the process `PATH`, then the login shell's `PATH` (`$SHELL -l -c` printing it between
markers, 5 s timeout, once per run; skipped on Windows, whose desktop-launched `PATH` is already the
user's), then well-known directories (`/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`,
`~/google-cloud-sdk/bin`). Not found or failing → `ExecPlugin { command, error, search_path }`.
The app never stores or logs anything from `users[]` (FR-002); the persisted settings hold the
context and namespace names only.

**Rejected**: trusting the inherited `PATH` (a Dock- or Finder-launched app has `/usr/bin:/bin`
only); asking the user for the plugin path (a field per cloud vendor).

## K10. Reconnect and "pod gone" (FR-010, FR-011, FR-023, US2)

**Decision**: the 489 actor loop and `Backoff` are reused unchanged; the Kubernetes dial is one more
`Dial` arm. Before each re-dial the actor `GET`s the pod: `404` or a `deletionTimestamp` → gone
("the pod was deleted"), `status.reason = Evicted` → evicted (with its message), `phase = Unknown` or
the node `NotFound` → node lost; on gone, the actor emits a new `DaemonEvent::Gone(reason)`, the state
becomes `Stopped` with that reason, sessions are marked `lost to its daemon`, and the actor stops
without recreating anything. A transient API error keeps `Reconnecting`. Sessions survive the client
(FR-010) because the daemon is the pod's process; reconnection reads the catalog and history as 489
does. Another client's Stop or Remove reaches this client by the same check (FR-023).

## K11. Stop, Remove and the unreachable case (FR-014, FR-015)

**Decision**: Stop deletes the pod by name and UID (grace 10 s) and waits until it is gone (bounded
120 s); the PVC and secrets stay. Remove deletes pod, secrets and PVC (ours only), then forgets the
daemon. When the cluster is unreachable the confirmation lists the deterministic resource names (K5)
and offers "Forget locally"; nothing in the cluster is touched. The confirmations reuse 491's
bound-worktree counts and the running-session count from the catalog.

## K12. CI: a kind cluster with a local registry in the job (FR-021)

**Decision**: a new Linux CI job `kube-cluster` (beside `ssh-remote`, same `classify` docs-only skip)
installs `kind` and `kubectl` at pinned versions with checksums, starts a `registry:2` container on
`localhost:5001`, creates a kind cluster whose containerd mirrors `localhost:5001` to that registry
(kind's documented local-registry recipe), and runs `cargo test -p micold-client --features
kube-real-runtime kube_real_ -- --test-threads=1`. The suite uses the **build** choice with a test
Dockerfile (the app's template plus the stand-in AI CLI installed as `claude`), pushing to the job's
registry, so the default image path (build → push → pull by digest) is exercised for real. RBAC denial
uses a kubeconfig context for a ServiceAccount whose Role lacks `create pods` (and one lacking
`pods/portforward`), from `kubectl create token`; image-pull failure names a digest absent from the
registry; unschedulable uses a node selector no node has. Locally: `mise run test-kube` (needs docker,
kind, kubectl; creates and deletes its own cluster `micold-kube-test`). Off by default (cargo feature).

**Why**: a registry in the job is what the product does (push then pull); `kind load docker-image`
would skip the push and digest steps the default path depends on.

**Rejected**: `kind load docker-image` (bypasses FR-004's push); k3d or minikube (kind is the
smallest to run on a GitHub runner and is what Kubernetes' own CI uses); a long-lived shared cluster
(cost, credentials in CI, cross-run interference).

## K13. Platform scope of the real-cluster suite

**Decision**: the real-cluster suite runs on Linux only (kind needs a Linux container runtime; GitHub
macOS runners have none and Windows runners lack Linux containers). Cross-platform parity of the
client-side rules (kubeconfig candidates, separators, exec-plugin resolution, home directory) is
proven by pure, platform-parameterised core tests that run on all three CI platforms. The reason is
recorded in `ci.yml` beside the job, as 489 did for `ssh-remote`.
