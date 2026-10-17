# Research: Kubernetes session daemon runtime (#686)

Each decision names what was chosen, why, and what was rejected.

**Revision 2 (2026-10-17)**: the owner replaced the image design (ledger decisions 8-10): the pod
runs a basic image with an SSH server, the client copies its own daemon in over SSH and installs the
AI CLIs on start, with one mechanism shared with the local container runtime, built by #806 and
consumed here. K1-K4 and K12 are rewritten; K0, K6, K7 and K10 are amended; the build, tag and push
decisions of revision 1 (old K1-K3), the in-pod loopback listener without a token (old K4) and the
kind registry (old K12) are gone. The clone source (K8) is unchanged.

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
in-process `AsyncRead + AsyncWrite` stream, which the loopback relay of K2 hands to the user's `ssh`.
The user needs no `kubectl` installed, which the spec's Assumptions do not promise.

No hash crate is added: the daemon key (K5) uses the core's existing `protocol::hashing::sha256_hex`.

**Rejected**: shelling out to `kubectl` (an unstated runtime requirement; `kubectl port-forward`
opens a loopback listener any local process can use; parsing its human output for failure reasons is
fragile); a hand-written REST client over `ureq` (re-implements exec plugins, auth providers,
SPDY/WebSocket port-forward); `kube` in `micold-core` (pulls hyper, tower and rustls into the
render-free crate the daemon also compiles; the core keeps only rules).

## K1. The SSH image, daemon copy and AI CLI install: #806's shared mechanism, consumed here

**Decision**: #686 does not build images, push them, or carry its own copy of the daemon-provisioning
code. It consumes one mechanism, built by the #806 run (owner, 2026-10-17), that both the local
container runtime and this runtime use: (a) a basic **SSH image** with a published reference the
client pins; (b) a **per-daemon key pair** and a host key; (c) **provisioning over SSH**, spec 489's
probe, copy and bridge (`ssh::provision`, FR-005) generalised to a target reached on a loopback port
with a pinned host key and with daemon environment passed through; (d) the **AI CLI install on
start** into the volume, idempotent, streamed, cancellable and classified. What #686 needs from it is
written down as [contracts/shared-ssh-image.md](./contracts/shared-ssh-image.md); #806's plan owns the
module and its names (working name `micold-core/src/ssh_image/` in this feature's documents). Every
#686 task that calls it is marked **blocked on #806** and sits in the last milestones. The contract
also requires the copy and the install to be safe when two clients run them at once on one volume
(FR-023): each writes to a temporary path and renames into place, under a lock file on the volume, so
the later client waits within the phase bound and then finds the result in place. #686 reaches the
key generation through a small port (`KeyGen`, data model) so the access secret is built and tested
before #806 lands.

Until #806 lands, the Kubernetes start sequence runs up to an open tunnel to the pod's SSH server
(phases `Config` to `Tunnel`, K6) and is proven over `FakeCluster`; the Add form does not offer the
Kubernetes runtime (FR-024, no half-wired UI) until the milestone that connects.

**Why**: the owner chose the mechanism and its sharing; one copy means the local container and the
pod cannot drift, and the daemon is always the client's own build (FR-017 by construction, as 489).

**Rejected**: building the mechanism here first (the owner moved it to #806); a Kubernetes-only copy
step through `pods/exec` (a second mechanism, and `exec` RBAC is broader than `portforward`); keeping
revision 1's build → push → pull by digest (owner decision 9).

## K2. Reaching the pod's SSH server: port-forward behind a loopback relay

**Decision**: the pod's SSH server listens on the pod's loopback, `127.0.0.1:2222` (non-root cannot
bind 22). spec 489 drives the user's own `ssh` binary (`ssh::argv`), which needs a TCP endpoint, so
the client runs a **loopback relay** per connected daemon (`micold-client/src/kube/relay.rs`): a
listener on `127.0.0.1:0`; each accepted connection opens a fresh `Cluster::port_forward(pod, 2222)`
stream and copies bytes both ways. The SSH target handed to the shared provisioning is host
`127.0.0.1`, the relay's port, the image's user, the daemon's client key, and the pinned host key
under a per-daemon `HostKeyAlias` in a per-daemon known-hosts file (K3), so `StrictHostKeyChecking=yes`
and `UpdateHostKeys=no` hold as in 489. The relay lives as long as the daemon's actor holds it and
closes on disconnect, stop or remove. Before #806, the `Tunnel` phase proves the path by reading
the SSH server's identification line (`SSH-2.0-`) through the relay; the login with the pinned host
key happens in `Copy`, where a mismatch is classified as `HostKeyMismatch`. `Transport::Tunnel` (revision 1, T010/T017) is not needed: the
protocol runs over 489's `Transport::Ssh`.

**Why**: reuses 489's mechanism unchanged in shape (the owner's instruction); a loopback listener is
inside FR-006 ("nothing beyond loopback"), and a local process that connects to it meets the SSH
server's key-only login, not the daemon.

**Rejected**: `ProxyCommand` running the app's own executable as a relay (a second process that must
re-read kubeconfig and re-run exec plugins); an in-process SSH client crate (a second SSH stack beside
489's); `kubectl port-forward` (K0).

## K3. SSH access: per-daemon access secret, per-client keys, pinned host key (FR-008, FR-009)

**Decision**: one secret per daemon, `<base>-ssh-access`, labelled ours (K5), holds the pod's host
key pair (`ssh_host_ed25519_key`, `.pub`) and `authorized_keys`, one line per client allowed to log
in, each tagged with a client id comment. The client that first starts the daemon generates the host
key pair and its own key pair with the shared mechanism's key generation; a later client (another
computer, FR-023 / 489 FR-021) adds its public key to `authorized_keys` by an update guarded by the
secret's `resourceVersion` (a `409` re-reads and retries), and reads the host public key from the
secret through the cluster API, whose trust is the user's kubeconfig. Each client keeps its private
key and the pinned host key in owner-only files under the app's data directory (`owner_only.rs`),
keyed by the daemon key; never in settings, the cluster or logs. The image's SSH server reads
`authorized_keys` straight from the mounted secret (so a key added later is seen without restarting
the pod, after the kubelet's volume refresh; the `Tunnel` phase waits for a login within its bound),
and copies the host key to `/tmp` with mode 0600 at startup (`sshd` refuses a group-readable host
key). Remove deletes the secret and this client's key files; when the cluster is unreachable, the
confirmation lists the access secret, and with it this client's still-authorized key, among the
resources that remain (FR-015).

**Why**: the private key never leaves the client (FR-008); the host key is pinned, so a replaced pod
or secret is a refused mismatch, not a silent trust (edge case); several clients share one pod
without recreating it.

**Rejected**: one key shared by every client (it would have to travel between computers); a
password or token in the pod spec (readable by `get pods`); trust on first use of the host key (the
cluster API already gives an authenticated copy).

## K4. Revision 1's in-pod loopback listener without a token: removed

**Decision**: the daemon is reached through 489's bridge over SSH, so it never listens for the
port-forward. The `listener_auth` rule (`protocol/auth.rs`) and `adopt_auth_token`'s empty-token path
in `micold-daemon/src/server.rs`, done in M1a (T008, T016) but unmerged, are reverted; feature 027's
rule (an empty token path is fatal) stands unchanged.

**Why**: a relaxed authentication path with no caller is a liability (Constitution: no dead code; a
security check loosened for nothing).

## K5. Resource identity, ownership and races (FR-003, FR-016, FR-023)

**Decision**: every resource is named deterministically from the daemon key
`key = sha256_hex(context ‖ 0 ‖ namespace ‖ 0 ‖ daemon name)[..8]` (`protocol::hashing`):
`micold-<slug(name)>-<key>` (pod), `…-data` (PVC), `…-ssh-access` (K3), `…-git-credentials`,
`…-git-ssh`, `…-gitconfig`, `…-signin-<cli>`, `…-pull` (secrets). `slug` is DNS-1123, at most 40 characters. Every resource
carries labels `app.kubernetes.io/managed-by=micold-ai-ide` and `micold.dev/daemon=<key>` and the
annotation `micold.dev/daemon-name=<name>`. A resource is **ours** only when both labels match.
Create is `POST`; `409 AlreadyExists` → `GET`: ours → adopt (the race loser reuses the winner's pod,
FR-023), not ours → `Conflict { kind, name }` (edge case "Pod created by someone else"). The client
never lists or deletes by label selector alone; deletes name the resource and pass a precondition on
its UID read in the same step. An adopted pod carries the annotation `micold.dev/pod-settings`, the
canonical JSON of the settings that shape it (image, resources, node selector, tolerations, allowed
credentials); when it differs from the saved settings, start stops and asks the user to recreate the
pod or keep it (FR-005), naming the settings that differ.

**Why**: names derived from the three identifying fields make two clients with the same daemon meet on
the same objects with no coordination (spec 489 FR-021), and the API's create-uniqueness is the lock.

**Rejected**: generated names with a label lookup (two racing clients both create); a Deployment or
StatefulSet (spec Assumption: bare pod; a controller would recreate a pod the user stopped).

## K6. Start sequence, phase bounds and failure classification (FR-012, FR-013, US4)

**Decision**: start is a fixed sequence of phases in `micold-core/src/kube/start.rs`, a pure state
machine over `Cluster` results, each with its bound: (1) `Config` load kubeconfig, context, exec
plugin — 120 s; (2) `Preflight` namespace `GET` and `SelfSubjectAccessReview` for `create pods`,
`create persistentvolumeclaims`, `create secrets`, `update secrets`, `create pods/portforward`,
`delete pods` — 120 s, so RBAC denial is reported before anything is created (story 4.1); (3) `Volume`
ensure PVC — 120 s; (4) `Secrets` reconcile the access secret (K3) and the allowed ones — 120 s;
(5) `Pod` create or adopt — 120 s; (6) `Schedule` until `PodScheduled=True` — 120 s; (7) `Pull` until
the container is no longer `ContainerCreating` with a pull in progress — 15 min; (8) `Tunnel` the
relay (K2) up and the SSH server's identification line read through it — 120 s (the login with the
pinned host key is in `Copy`). Then the shared mechanism
(K1, **blocked on #806**): (9) `Copy` probe, copy if absent, start — 120 s; (10) `Install` the AI
CLIs — 15 min, live, cancellable; (11) `Connect` the 489 bridge and handshake — 120 s. Failure
classes (`KubeFailure`) come from pure classifiers over API status codes and pod/PVC conditions and
events: `Forbidden { verb, resource }` (403 or a denied review), `ImagePull { image, message }`
(`ErrImagePull`, `ImagePullBackOff`, `InvalidImageName` — reported on first sight, not after the
bound), `Unschedulable { message }` (`PodScheduled=False, reason=Unschedulable`), `ClaimNotBound
{ message }` (PVC `ProvisioningFailed` event, or the scheduler's "unbound immediate
PersistentVolumeClaims"), `ContextMissing`, `NamespaceMissing`, `Unreachable`, `ExecPlugin`,
`Conflict`, `ImageContract { image, missing }` (the container exits or the SSH server never answers
on 2222), `HostKeyMismatch`, `PhaseTimeout { phase }`, `Cancelled`, and the shared mechanism's own
failures (copy, start, install; their texts are #806's) wrapped as `Provision(..)`. A failed start
deletes what this attempt created (pod, secrets; the PVC only if this attempt created it), by name
and UID (FR-013); a client key added to an adopted access secret in this attempt is removed again.

**Why**: a phase list with one bound each is the spec's FR-012 text made data; each classifier is a
table test.

**Rejected**: one overall timeout (an image pull or a CLI install legitimately takes minutes while RBAC
denial must not wait for it); watching with informers (a `GET` poll every second within a phase is enough for one pod
and much simpler to fake).

## K7. Pod shape and secrets (FR-006, FR-008, FR-009, US3)

**Decision**: one container running the SSH image (#806's pinned reference, or the user's), command
left to the image, `restartPolicy: Always`, `automountServiceAccountToken: false` (else the
service-account token would be a secret the user did not allow), `enableServiceLinks: false`,
pod `securityContext { runAsNonRoot, runAsUser: 10001, runAsGroup: 10001, fsGroup: 10001,
seccompProfile: RuntimeDefault }`, container `securityContext { readOnlyRootFilesystem: true,
allowPrivilegeEscalation: false, capabilities.drop: [ALL] }`; volumes: the PVC at `/data` (the image
user's home `/data/home`, the copied daemon under it per 489's `remote_dir`, the AI CLIs, `/data/repos`),
an `emptyDir` at `/tmp`, the access secret (K3) read-only at `/run/micold/ssh` (`defaultMode: 0440`);
`resources.requests`/`resources.limits` from `DaemonResources` (each key only when set, FR-005);
`nodeSelector` and `tolerations` as given (the daemon's architecture is found by 489's probe, so no
arch selector is added). The SSH server's port is never declared as a `containerPort`. The pod's env
reaches the SSH server, not the daemon (sshd starts sessions with a clean environment), so the daemon's
environment — `MICOLD_IDLE_STOP=off` (the pod's lifetime is the user's Stop), `MICOLD_REPOS_ROOT=/data/repos`
(K8), and each allowed credential's variables — is passed by the shared provisioning's start step
(contract shared-ssh-image §3), built by `kube/credentials.rs`. Allowed secrets are mounted
read-only at fixed paths: `/run/micold/creds/git-credentials` (git `credential.helper store --file`
via `GIT_CONFIG_*` env), `/run/micold/creds/ssh/{id,known_hosts}` (`GIT_SSH_COMMAND` with
`IdentitiesOnly=yes`, `StrictHostKeyChecking=yes`, the user's known-hosts lines for the remote's host
only), `/data/home/.gitconfig` (subPath), each sign-in at its provider's `sandbox_auth_file` path
under `HOME` (subPath). Image-pull secret in `imagePullSecrets` only when allowed (a private user
image). Withdrawn secrets (ours, not in the allow-list) are deleted in the `Secrets` phase. No
Service, Ingress, `hostPort`, `hostNetwork` or `containerPort` is ever written; a manifest test
asserts it.

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
`Dial` arm (relay up, then 489's bridge over it). Before each re-dial the actor `GET`s the pod: `404` or a `deletionTimestamp` → gone
("the pod was deleted"), `status.reason = Evicted` → evicted (with its message), `phase = Unknown` or
the node `NotFound` → node lost; on gone, the actor emits a new `DaemonEvent::Gone(reason)`, the state
becomes `Stopped` with that reason, sessions are marked `lost to its daemon`, and the actor stops
without recreating anything. A transient API error keeps `Reconnecting`. Sessions survive the client
(FR-010) because the daemon runs detached in the pod, as 489's does on a host, and outlives the SSH
connection; reconnection reads the catalog and history as 489
does. Another client's Stop or Remove reaches this client by the same check (FR-023).

## K11. Stop, Remove and the unreachable case (FR-014, FR-015)

**Decision**: Stop deletes the pod by name and UID (grace 10 s) and waits until it is gone (bounded
120 s); the PVC and secrets stay. Remove deletes pod, secrets and PVC (ours only), then forgets the
daemon. When the cluster is unreachable the confirmation lists the deterministic resource names (K5)
and offers "Forget locally"; nothing in the cluster is touched. The confirmations reuse 491's
bound-worktree counts and the running-session count from the catalog.

## K12. CI: a kind cluster, the SSH image loaded into it (FR-021)

**Decision**: a new Linux CI job `kube-cluster` (beside `ssh-remote`, same `classify` docs-only skip)
installs `kind` and `kubectl` at pinned versions with checksums, creates a kind cluster, makes the SSH
image available to it (pulled by the cluster from its published reference, or, for a stand-in test
image built from #806's image plus the stand-in AI CLI's package, `kind load docker-image`), and runs
`cargo test -p micold-client --features kube-real-runtime kube_real_ -- --test-threads=1`. The
stand-in AI CLI is installed on start like any CLI, from a package source reachable from the pod
(a local directory on the volume or a file server in the job, per #806's install contract). RBAC
denial uses a kubeconfig context for a ServiceAccount whose Role lacks `create pods` (and one lacking
`pods/portforward`), from `kubectl create token`; image-pull failure names an image reference that does
not exist; unschedulable uses a node selector no node has; install failure points the stand-in CLI at
an unreachable source. Locally: `mise run test-kube` (needs docker, kind, kubectl; creates and deletes
its own cluster `micold-kube-test`). Off by default (cargo feature). The suite runs after #806 lands
(**blocked on #806**).

**Why**: no registry is part of the product any more, so none is part of the test.

**Rejected**: a `registry:2` in the job (revision 1; nothing pushes now); k3d or minikube (kind is the
smallest to run on a GitHub runner and is what Kubernetes' own CI uses); a long-lived shared cluster
(cost, credentials in CI, cross-run interference).

## K13. Platform scope of the real-cluster suite

**Decision**: the real-cluster suite runs on Linux only (kind needs a Linux container runtime; GitHub
macOS runners have none and Windows runners lack Linux containers). Cross-platform parity of the
client-side rules (kubeconfig candidates, separators, exec-plugin resolution, home directory) is
proven by pure, platform-parameterised core tests that run on all three CI platforms. The reason is
recorded in `ci.yml` beside the job, as 489 did for `ssh-remote`.
