# Tasks: Kubernetes session daemon runtime

**Input**: Design documents from `/specs/686-kubernetes-daemon-runtime/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/kube-runtime.md](./contracts/kube-runtime.md),
[contracts/daemon-image.md](./contracts/daemon-image.md), [quickstart.md](./quickstart.md)

**Tests**: Mandatory (Constitution I). In every phase the test tasks come first and must fail before
the implementation tasks after them. Core rules are `#[cfg(test)]` unit tests in the module named;
client sequences run over `FakeCluster` (`micold-core` feature `kube-fake`); the real-cluster suite
is US6.

**Documentation**: each user-visible phase carries its user-guide task (Constitution VII, CI's
user-guide gate).

**Behavior markers**: `[A<n>]` and `[U<n>]` are the behavior ids of [tdd/test-list.md](./tdd/test-list.md).
`/speckit.tdd.run` ticks a task once every behavior it carries is done; a test task must be seen
failing for the right reason before the implementation task with the same ids starts. Each
milestone ends with an acceptance task: its outer-loop behaviors green before it is complete.

**Visual pass**: each milestone that changes the Settings form (M1, M2, M5, M7) runs the
`visual-pass` skill at its gate (autopilot gate rule) on the parts it adds; T097 is the full
quickstart Part B pass at the end.

**Cross-platform**: the platform-dependent rules (kubeconfig separator, home, exec search path) are
pure functions taking the platform, tested on all three CI platforms (Constitution VI, research K13).

**Separable parts** (FR-004/FR-007 run on orchestrator defaults pending the owner): the image source
is Phase 3b, the clone source Phase 3c. A different answer replaces those phases only.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US6) the task serves

Paths: `core/` = `crates/micold-core/src/`, `client/` = `crates/micold-client/src/`,
`client-tests/` = `crates/micold-client/tests/`, `daemon/` = `crates/micold-daemon/src/`.

---

## Phase 1: Setup

**Purpose**: dependencies, features and empty modules.

- [ ] T001 Add workspace dependencies `kube` 4.2 (`default-features = false`; features `client`, `config`, `rustls-tls`, `ring`, `ws`) and `k8s-openapi` 0.28 (feature `v1_32`) in the root `Cargo.toml` and use them from `crates/micold-client/Cargo.toml` only; add `[features] kube-real-runtime = []` to `crates/micold-client/Cargo.toml`. Confirm `cargo tree -p micold-client -i openssl-sys` prints nothing and both crates are Apache-2.0 (research K0); `micold-core` and `micold-daemon` gain no new dependency.
- [ ] T002 Add feature `kube-fake = []` to `crates/micold-core/Cargo.toml` (comment: exposes `kube::fake::FakeCluster` to client tests) and enable it on the `micold-core` entry of `[dev-dependencies]` in `crates/micold-client/Cargo.toml`.
- [ ] T003 [P] Create empty modules `core/kube/mod.rs` and `core/daemon_image/mod.rs` registered in `core/lib.rs`, and `client/kube/mod.rs` registered in `client/lib.rs`.

---

## Phase 2: Foundational (blocks every story)

**Purpose**: the persisted settings, resource identity, the cluster port and its fake, the tunnel
transport, and the in-pod listener rule.

### Tests (write first, must fail)

- [ ] T004 [P] [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] `KubeSettings` validation tests in `core/kube/settings.rs`; each refusal is `InvalidField` naming the field (FR-018): `context` "non-empty; existence in kubeconfig checked at start, not on save"; `namespace` "DNS-1123 label, 1-63 chars"; `registry` "required when `image` is `Build` (FR-004); `host[:port]/path`, no tag, no digest"; `arch` "`Amd64` default, `Arm64`"; `node_selector` keys and values "Kubernetes label syntax"; `tolerations` "`key[=value]:Effect` or `key:Exists:Effect`; `Effect` ∈ NoSchedule, PreferNoSchedule, NoExecute"; `volume_size` "≥ 1 GiB, default 10 GiB"; `storage_class` "none = cluster default"; `allowed` "empty by default".
- [ ] T005 [P] [U9] [U10] [U11] [U12] `DaemonImageSource` and `DaemonResources` tests in `core/daemon_image/mod.rs`: `Build` is the default with `dockerfile: None` and `providers` defaulting to the CLIs the Settings provider list enables; `Pull { reference: ImageRef }` flags a moving tag as `sandbox::image` does; `DaemonResources` defaults `cpu_request: None`, `cpu_limit: Some(sandbox::DEFAULT_MILLI_CPUS)`, `memory_request: None`, `memory_limit: Some(sandbox::DEFAULT_MEMORY)`; "each of the four set or unset on its own; a request above its limit refused naming the request field".
- [ ] T006 [P] [U13] [U14] [U15] [U16] `DaemonRuntime::Kubernetes` tests in `core/daemons.rs`: serde round trip as `{"kind":"kubernetes", ...}` through the hand-written `DaemonRuntime` serde, no `settings_version` bump; an unknown-kind reader keeps it as `Unsupported` verbatim (491); `label()` = "Kubernetes"; `is_startable()` = true; endpoint = `<context> / <namespace> / <pod name>`; a duplicate daemon name → `RegistryError::DuplicateName` naming the first.
- [ ] T007 [P] [U18] [U19] [U20] [U21] Name and ownership tests in `core/kube/names.rs`: `DaemonKey::of(context, namespace, name)` = first 8 hex of `protocol::hashing::sha256_hex` over the three NUL-separated; `slug` is DNS-1123 and at most 40 characters; `ResourceNames` gives `micold-<slug>-<key>` (pod), `…-data`, `…-gitconfig`, `…-git-credentials`, `…-git-ssh`, `…-signin-<cli>`, `…-pull`; labels `app.kubernetes.io/managed-by=micold-ai-ide`, `micold.dev/daemon=<key>`, annotation `micold.dev/daemon-name=<name>`; `is_ours` is true only when both labels match.
- [ ] T008 [P] [U22] [U23] [U24] [U25] `listener_auth` table tests in `core/protocol/auth.rs` (K4): set path → `Token(path)`; empty path + loopback address (`127.0.0.1`, `::1`) → `LoopbackNoToken`; empty path + non-loopback address → `AuthConfigError`; plus a test in `daemon/server.rs` that `adopt_auth_token` with an empty `MICOLD_TOKEN_PATH` and `MICOLD_LISTEN_ADDR=0.0.0.0:7727` stays fatal.
- [ ] T009 [P] [U26] `KubeFailure` text tests in `core/kube/classify.rs`: every variant of the data model (`ContextMissing`, `NoKubeconfig`, `Unreachable`, `ExecPlugin`, `NamespaceMissing`, `Forbidden`, `NoRuntime`, `Build`, `Push`, `ImagePull`, `Unschedulable`, `ClaimNotBound`, `Conflict`, `CredentialSource`, `PhaseTimeout`, `Cancelled`) displays exactly its contract §6 text.
- [ ] T010 [P] [U34] `Transport::Tunnel` test in `core/connect.rs`: a `tokio::io::duplex` half boxed as `Transport::Tunnel(Box<dyn AsyncReadWrite>)` completes the 491 handshake against a daemon-side stub; no kube type appears in `connect.rs`.

### Implementation

- [ ] T011 [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] `KubeSettings` and its newtypes (`KubeContextName`, `Namespace`, `RegistryRepo`, `NodeArch`, `LabelKey`, `LabelValue`, `Toleration`, `Quantity`) with validation and serde in `core/kube/settings.rs` (data model "KubeSettings"; `allowed: BTreeSet<KubeCredential>` with `KubeCredential` declared in `core/kube/credentials.rs` as the five variants of the data model, mapping added in US3).
- [ ] T012 [P] [U9] [U10] [U11] [U12] `DaemonImageSource` and `DaemonResources` in `core/daemon_image/mod.rs`, reusing `sandbox::MilliCpus`, `sandbox::Bytes` and `sandbox::image::ImageRef` (not `sandbox::ResourceBudget`).
- [ ] T013 [U13] [U14] [U15] [U16] `DaemonRuntime::Kubernetes(KubeSettings)`, `label`, `is_startable`, endpoint text in `core/daemons.rs` (depends on T011, T012).
- [ ] T014 [P] [U18] [U19] [U20] [U21] `DaemonKey`, `slug`, `ResourceNames`, `Ownership`, `is_ours` in `core/kube/names.rs`.
- [ ] T015 `Cluster` port in `core/kube/mod.rs` (a trait returning boxed futures, like `ssh::SshRunner`): create (→ `Created { uid }` or `AlreadyExists`), get, delete by name with `preconditions.uid` (`404` = done), namespace get, access review, pod status (`PodStatusView`), events (`EventView`), node get, `port_forward(pod, 7727)` → `Box<dyn AsyncReadWrite>`, `logs(pod, tail)`; `ApiFailure { code, reason, message }`. `FakeCluster` in `core/kube/fake.rs` under `cfg(any(test, feature = "kube-fake"))`: scriptable per-call answers, pod phase progressions, a call log, objects keyed by kind and name, a tunnel built from `tokio::io::duplex`.
- [ ] T016 [P] [U22] [U23] [U24] [U25] `listener_auth` in `core/protocol/auth.rs`; `adopt_auth_token` in `daemon/server.rs` uses it.
- [ ] T017 [P] [U34] `Transport::Tunnel` in `core/connect.rs` and its dial arm.
- [ ] T018 [P] [U26] `KubeFailure` with `Display` in `core/kube/classify.rs` (new file); its classifiers come in M2 (`from_runtime`), M4 (`PodGone`) and M6 (`from_status`, `from_pod`, `from_pvc`).

**Checkpoint**: `mise run test-core` green; a Kubernetes daemon round-trips through `settings.json`.

---

## Phase 3: User Story 1 — Work on a project in a Kubernetes daemon end to end (P1) 🎯 MVP

**Goal**: add a Kubernetes daemon, start it, bind a worktree, run a session in the pod.

**Independent test**: with a cluster, add a daemon with defaults, start it, create a bound worktree,
start a session and type; input and output reach it (US1 scenarios 1–5).

### 3a — Start and connect with a given image (scenarios 1, 4, 5)

#### Tests (write first, must fail)

- [ ] T019 [P] [US1] [U35] [U36] [U37] [U38] [U39] [U40] Manifest tests in `core/kube/manifest.rs` for `pod(..)` and `pvc(..)`: never a `Service` or `Ingress` kind; no `hostPort`, `hostNetwork`, `hostPID`, `privileged`, `containerPort`; `restartPolicy: Always`, `automountServiceAccountToken: false`, `enableServiceLinks: false`; one container with env `MICOLD_LISTEN_ADDR=127.0.0.1:7727`, `MICOLD_TOKEN_PATH=` (empty), `MICOLD_IDLE_STOP=off`, `MICOLD_REPOS_ROOT=/data/repos`, `HOME=/data/home`, `XDG_DATA_HOME=/data/state`, `TMPDIR=/tmp`; the PVC mounted at `/data` and an `emptyDir` at `/tmp`; the PVC is `ReadWriteOnce` with `volume_size` and `storage_class` (omitted when none); both ownership labels and the name annotation on every object; `nodeSelector` and `tolerations` as given.
- [ ] T020 [P] [US1] [U45] [U46] [U47] [U48] Kubeconfig tests in `core/kube/kubeconfig.rs`, parameterised over Linux, macOS and Windows (FR-022): `candidates` splits `KUBECONFIG` on `;` (Windows) or `:` (else), else `<home>/.kube/config` with home `%USERPROFILE%` (Windows) or `$HOME`; `search_path(process_path, login_shell_path, home, platform)` is the process `PATH`, then the login shell's `PATH` (absent on Windows), then `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/google-cloud-sdk/bin`, de-duplicated keeping the first; `parse_login_shell_output` returns the text between the markers, `None` without them.
- [ ] T021 [P] [US1] [U49] [U50] [U51] [U52] [U53] [U54] [U55] [U56] [U57] Start sequence tests over `FakeCluster` in `core/kube/start.rs`: `StartPlan` for a Pull image is `Config, Preflight, Volume, Secrets, Pod, Schedule, Pull, Tunnel` and for Build has `Image` after `Preflight`; `StartPhase::bound()` is 30 min for `Image`, 15 min for `Pull`, 120 s for every other; a missing namespace → `NamespaceMissing`; the PVC is created on first start and reused on the next; a `201` is recorded in `Attempt.created`; `409` + ours → adopted, not recorded; `409` + not ours → `Conflict { kind, name }` with the contract §2 text; a pod with `deletionTimestamp` → wait until it is gone, then create; a `volume_size` smaller than the existing claim is refused naming the claim; each phase sets its `activity`; with a paused clock each phase fails `PhaseTimeout { phase }` at its `bound()`; any failure rolls back `Attempt.created` in reverse, by name and UID, the PVC only when this attempt created it, adopted objects never (FR-013).
- [ ] T022 [P] [US1] [A1] [A4] [A5] [U111] [U112] [U113] Client tests over `FakeCluster` in `client-tests/kube_daemon_start.rs`: Start → `Starting` → `connected` through `Transport::Tunnel`, row endpoint `<context> / <namespace> / <pod>`; a second Start with the pod present adopts it (US1.4); two actors starting the same daemon on one `FakeCluster` → exactly one pod create succeeds and both connect (FR-023); a second, non-Kubernetes daemon's state is unchanged throughout (FR-019); a handshake refused for version mismatch → `VersionMismatch` and no session started (FR-017); the cluster created no `Service` or `Ingress` (FR-006, US1.5); app launch with the cluster unreachable does not delay the other daemons or the window (FR-019).
- [ ] T023 [P] [US1] [U86] [U87] [U88] [U89] Pure-helper tests in `client/kube/live.rs` `#[cfg(test)]`: `rewrite_exec_commands` turns each non-absolute `users[].exec.command` into the first match on the search path and leaves absolute ones; not found → `ExecPlugin { command, error, search_path }`; contexts are listed from the merged files in order; the form's context list carries no `users[]` data (FR-002); `to_api_failure(&kube::Error)` table: an API `ErrorResponse` keeps its code, reason and message, `409 AlreadyExists` on create becomes `AlreadyExists`, `404` on delete counts as done, a connection error becomes `Unreachable { server, error }`; `delete_params(uid)` sets `Preconditions { uid }` and grace seconds.
- [ ] T024 [P] [US1] [U95] [U96] Reducer tests in `client-tests/features_daemons.rs`: choosing runtime Kubernetes in Add shows the context picker filled from the merged kubeconfig, or "no kubeconfig found; looked in <paths>"; fields namespace and image reference (Pull); Save refuses an invalid field naming it; the saved row shows "Kubernetes" and the endpoint.

#### Implementation

- [ ] T025 [US1] [U35] [U36] [U37] [U38] [U39] [U40] `pod`, `pvc` builders returning `serde_json::Value` in `core/kube/manifest.rs` (security context and credential mounts come in US3, resources in 3b).
- [ ] T026 [P] [US1] [U45] [U46] [U47] [U48] `candidates`, `search_path`, `parse_login_shell_output` in `core/kube/kubeconfig.rs`.
- [ ] T027 [US1] [U49] [U50] [U51] [U52] [U53] [U54] [U55] [U56] [U57] `StartPhase`, `bound()`, `StartPlan`, `Attempt`, `CreatedResource` and the happy-path sequence over `Cluster` in `core/kube/start.rs` with per-phase timeouts and rollback of `Attempt.created` (preflight access reviews and the cluster-status classifiers come in US4).
- [ ] T028 [US1] [U86] [U87] [U88] [U89] `LiveCluster` in `client/kube/live.rs`: merge candidate files with `kube::config::Kubeconfig`, `rewrite_exec_commands`, a `kube::Client` per context, the `Cluster` operations through `to_api_failure` and `delete_params`, `Api::<Pod>::portforward` on 7727 returned as the boxed stream, `logs` with `tail_lines`; the login shell's `PATH` read once per run with a 5 s timeout (skipped on Windows).
- [ ] T029 [US1] Choose `LiveCluster` once in `client/shell/capabilities.rs`; `client-tests/no_concrete_implementations.rs` stays green.
- [ ] T030 [US1] [A1] [A4] [A5] [U111] [U112] [U113] `Dial::Kube` in `client/daemon.rs`: run the start sequence inside the daemon's actor (no cluster I/O on the UI thread, FR-019), dial `Transport::Tunnel`, then the 491/489 handshake; a failed start → `Unreachable` with the reason and no further attempts until the user's Start or Reconnect (489 R7).
- [ ] T031 [US1] [U95] [U96] Kubernetes `Draft`/`Msg` (context, namespace, image reference) in `client/features/daemons.rs`; kubeconfig context listing off the UI thread in `client/shell/daemons.rs`.
- [ ] T032 [US1] [U96] Kubernetes form glue in `client/ui/settings/daemons.rs`; extend `client-tests/daemons_section_geometry.rs` to the Kubernetes form.
- [ ] T033 [US1] User guide: new `docs/user-guide/kubernetes-daemons.md` (adding, starting, what is created in the namespace, no Service, stop, remove, and Forget locally when the cluster is unreachable), linked from `docs/SUMMARY.md`, `docs/user-guide/settings.md` and `docs/user-guide/remote-daemons.md`.

### 3b — The daemon image: build, push, resources (scenario 1 default path; FR-004, FR-005)

#### Tests (write first, must fail)

- [ ] T034 [P] [US1] [U71] [U72] [U73] Template tests in `core/daemon_image/containerfile.rs`: `render(providers)` installs exactly the selected providers' npm packages at the versions pinned in `packaging/sandbox/Containerfile` (read with `include_str!`), creates UID/GID 10001, sets `USER 10001`, `HOME=/data/home`, `XDG_DATA_HOME=/data/state`, `TMPDIR=/tmp`, has no `COPY` or `ENV` of any credential, ends with the daemon as `ENTRYPOINT`; a test fails when an `AiCli` has no pinned `ARG`.
- [ ] T035 [P] [US1] [U74] [U75] Tag tests in `core/daemon_image/tag.rs`: `ImageInputs { daemon_sha256, containerfile_sha256, providers, arch }` → `<registry>/micold-daemon:<CARGO_PKG_VERSION>-<12 hex>`; equal inputs give equal tags; a change of daemon bytes, Containerfile bytes, provider set or arch changes the tag; never a `sandbox::image::MOVING_TAGS` tag; `PushedImage { tag, digest }.pod_image()` = `<registry>/micold-daemon@<digest>`.
- [ ] T036 [P] [US1] [U76] [U77] [U78] [U27] [U28] `CliRuntime` tests over `RecordingRunner` in `core/sandbox/cli.rs`: `build_image` argv `<rt> build --platform linux/<arch> -f <file> -t <tag> <context>`, plus `--build-context micold=<tmp>` for a user Dockerfile; `push_image` argv `<rt> push <tag>`; `repo_digest` runs `<rt> image inspect --format '{{json .RepoDigests}}' <tag>` and picks the `<registry>/micold-daemon` entry; none → `Push` "the registry returned no digest"; `<rt>` is `sandbox::cli::program_of(kind)`. In `core/kube/classify.rs`: `from_runtime` maps a failed build to `Build { output_tail }` (its last lines) and a failed push ("not logged in", "denied", unreachable) to `Push { registry, error }`.
- [ ] T037 [P] [US1] [U90] [U91] [U92] [U93] [U94] Image-step tests over `RecordingRunner` in `client/kube/image.rs` `#[cfg(test)]`: the daemon binary is chosen by `ssh::provision::ArtefactChoice` (own binary on a Linux client of that arch, else `<install>/remote-daemons/linux-<arch>/micold-daemon`; absent → "no daemon build for linux-<arch> ships with this app"); tag present (`inspect_image`) → no build; push always; the digest is what the pod uses; output lines go to `DaemonDiagnostics::push_log`; Cancel kills the child → `Cancelled`; the `Image` phase's 30-minute bound from `start.rs` (paused clock) kills the child and fails `PhaseTimeout { phase: Image }`; a failed build → `Build` with the output tail; no runtime installed → `NoRuntime`.
- [ ] T038 [P] [US1] [U41] [U42] Manifest resource tests in `core/kube/manifest.rs`: for each of `cpu_request`, `cpu_limit`, `memory_request`, `memory_limit`, set alone, all set, none set → exactly those keys under `resources.requests` / `resources.limits` (`<n>m`, bytes); the Build choice adds `nodeSelector` `kubernetes.io/arch=<arch>`; the pod image is the digest reference.
- [ ] T039 [P] [US1] [U97] [U98] [U99] Reducer tests in `client-tests/features_daemons.rs`: image source choice Build (default) or Pull; Build with no Registry is refused naming "Registry"; providers default to the enabled ones; arch; the four resource fields, a request above its limit refused naming the request field; the container form still shows its image and limits fields unchanged.

#### Implementation

- [ ] T040 [P] [US1] [U71] [U72] [U73] `render(providers)` in `core/daemon_image/containerfile.rs`.
- [ ] T041 [P] [US1] [U74] [U75] `ImageInputs`, `ImageTag`, `PushedImage` in `core/daemon_image/tag.rs`; make `sandbox::image::MOVING_TAGS` `pub(crate)` to reuse it.
- [ ] T042 [P] [US1] [U76] [U77] [U78] [U27] [U28] Inherent `CliRuntime::{build_image, push_image, repo_digest}` over its `CommandRunner` (`run_streaming` for build and push) in `core/sandbox/cli.rs`, not on the `ContainerRuntime` trait; `from_runtime` in `core/kube/classify.rs`.
- [ ] T043 [US1] [U90] [U91] [U92] [U93] [U94] The image step in `client/kube/image.rs` over `CliRuntime<SystemRunner>` and its wiring into the `Image` phase of `core/kube/start.rs` and `client/daemon.rs`: activity "Building image" / "Pushing image" with Cancel through the actor, failures through `from_runtime`.
- [ ] T044 [US1] [U41] [U42] `resources` and the Build-choice arch selector in `core/kube/manifest.rs`.
- [ ] T045 [US1] [U97] [U98] [U99] Extract the image-source and limits fields from `sections` in `client/ui/settings/container_fields.rs` into functions in that file, drawn by the container form and the Kubernetes form (one copy, K1); Kubernetes `Draft` fields (choice, Dockerfile picker, providers, registry, arch, four resources) in `client/features/daemons.rs`; the geometry gate covers them.
- [ ] T046 [US1] User guide `docs/user-guide/kubernetes-daemons.md`: image source (Build pushes to your registry with your runtime's login; Pull), the tag and rebuild rule, resources.

### 3c — Clone and worktrees in the pod (scenarios 2, 3; FR-007)

#### Tests (write first, must fail)

- [ ] T047 [P] [US1] [U79] [U80] [U81] Clone rule tests in `core/kube/clone.rs`: `remote_for` picks `origin`, else the only remote; none → `NoRemote`; several without `origin` → `Ambiguous(names)`; `CloneTarget::for_url` = `/data/repos/<slug(repo)>-<hash8(url)>`, deterministic; `CloneFailure` from git stderr → `AuthRequired { remote }`, `DiskFull`, `Unreachable { remote }`, `Other`; texts as contract §6 (clone).
- [ ] T048 [P] [US1] [U82] Protocol tests in `core/protocol/messages.rs` and `core/protocol/version.rs`: `ClientMsg::RepoClone { url, dest }`, `DaemonMsg::RepoCloned { dest }`, `DaemonMsg::RepoCloneFailed { dest, failure }` round-trip; `PROTOCOL_VERSION` is 49.
- [ ] T049 [P] [US1] [U83] [U84] [U85] Daemon tests in `daemon/repo_clone.rs` against a local bare repository: clone into `.partial-<dest>-<rand>` and rename on success; an existing `dest` is fetched; an auth failure (with `GIT_TERMINAL_PROMPT=0`) leaves no directory; a `dest` outside `MICOLD_REPOS_ROOT` is refused.
- [ ] T050 [P] [US1] [A2] [U100] [U101] [U102] Reducer tests in `client-tests/features_daemons.rs` over a fake daemon: binding a worktree of a project with no remote to a Kubernetes daemon is refused with the reason and sends nothing; a project with a remote sends `RepoClone` then `WorktreeCreate`, in that order, showing activity "Cloning <repo>"; the bound worktree is a 489 remote project with identity `micold-remote://<daemon-id>/data/repos/…` (`ssh/remote_path.rs`), its row showing the daemon name and in-pod path.

#### Implementation

- [ ] T051 [P] [US1] [U79] [U80] [U81] `remote_for`, `CloneTarget`, `CloneFailure` in `core/kube/clone.rs`.
- [ ] T052 [US1] [U82] `RepoClone` / `RepoCloned` / `RepoCloneFailed` (`CloneFailureWire`) in `core/protocol/messages.rs`; `PROTOCOL_VERSION` 48 → 49 in `core/protocol/version.rs`.
- [ ] T053 [US1] [U83] [U84] [U85] `daemon/repo_clone.rs` (new) dispatched from `daemon/ops.rs`, `MICOLD_REPOS_ROOT` from the environment.
- [ ] T054 [US1] [A2] [U100] [U101] [U102] Create-worktree clone step for Kubernetes daemons in `client/features/` and `client/shell/workspace.rs`.
- [ ] T055 [US1] Docs: `docs/user-guide/worktrees-and-sessions.md` (worktrees on a Kubernetes daemon come from the project's remote; unpushed commits stay local; a remote needing a credential needs it allowed for the daemon), `docs/user-guide/kubernetes-daemons.md`, and `docs/daemon.md` (`RepoClone`, `MICOLD_REPOS_ROOT`, the pod's listener without a token).
- [ ] T100 [US1] [A2] Acceptance (M3): A2 green in `features_daemons` over the fake daemon.

**Checkpoint**: US1 runs end to end over `FakeCluster`; sessions in the pod use the existing ops.

---

## Phase 4: User Story 2 — Sessions outlive the client (P1)

**Goal**: a dropped tunnel reconnects to the same pod; a pod that is gone stops the daemon with its
reason and is not recreated.

**Independent test**: start a session producing output, quit, wait, reopen; the session runs with
the output produced meanwhile.

### Tests (write first, must fail)

- [ ] T056 [P] [US2] [U29] `PodGone` table tests in `core/kube/classify.rs` (contract §4): `404` or `deletionTimestamp` → `Deleted`; `status.reason == Evicted` → `Evicted { message }`; `phase == Unknown` or its node `404` → `NodeLost { node }`; present, `Running`, ready → `None`; texts "stopped: the pod was deleted", "stopped: the pod was evicted: <message>", "stopped: the pod's node <node> was lost".
- [ ] T057 [P] [US2] [U17] [U103] State tests in `core/daemons.rs`: `Reconnecting` or `Unreachable` + `Gone(reason)` → `Stopped` with `stopped_reason`; `Start` from `Stopped` → `Starting` and clears it; plus a reducer test in `client-tests/features_daemons.rs` that the row shows `stopped` with the reason.
- [ ] T058 [P] [US2] [A6] [A7] [A8] [A9] [U114] [U115] Actor tests over `FakeCluster` in `client-tests/kube_daemon_reconnect.rs`: tunnel dropped with the pod running → re-dial, sessions shown with output since the drop, no pod create; pod deleted, evicted or node lost → `Stopped` with the reason, sessions `lost to its daemon`, no create call, actor ended; a 5xx or timeout on lookup → stays `Reconnecting` with 489's back-off; Start after `Gone` → a new pod on the same PVC (no PVC create); after another client's Remove (pod and PVC gone) → `Gone(Deleted)`, and Start creates a new, empty PVC and pod (FR-023); another daemon untouched.

### Implementation

- [ ] T059 [P] [US2] [U29] `PodGone` and its classifier in `core/kube/classify.rs`.
- [ ] T060 [US2] [U17] `DaemonEvent::Gone`, `DaemonView.stopped_reason` and the transitions in `core/daemons.rs`.
- [ ] T061 [US2] [A6] [A7] [A8] [A9] [U103] [U114] [U115] Pod lookup before each re-dial in `client/daemon.rs`; sessions marked `lost to its daemon` through the 489 FR-019 path in `client/links.rs`; the row shows the reason in `client/ui/settings/daemons.rs`.
- [ ] T062 [US2] User guide `docs/user-guide/kubernetes-daemons.md`: sessions survive the client, reconnect, and what "stopped: the pod was …" means.
- [ ] T101 [US2] [A6] [A7] [A8] [A9] Acceptance (M4): `kube_daemon_reconnect` green for A6–A9.

---

## Phase 5: User Story 3 — Only what the user allows reaches the pod (P1)

**Goal**: allowed credentials and sign-ins become secrets mounted read-only; nothing else; the pod is
non-root with a read-only root filesystem.

**Independent test**: allow one credential and one sign-in, start, inspect the pod (US3.1–3.4).

### Tests (write first, must fail)

- [ ] T063 [P] [US3] [U66] [U67] [U68] [U69] [U70] Credential tests in `core/kube/credentials.rs`: each `KubeCredential` maps to exactly one secret (`ResourceNames`) and one mount: `GitCredentials` → `/run/micold/creds/git-credentials` with `GIT_CONFIG_*` env setting `credential.helper store --file`; `GitSshKey { key }` → `/run/micold/creds/ssh/{id,known_hosts}` with `GIT_SSH_COMMAND` using `IdentitiesOnly=yes` and `StrictHostKeyChecking=yes`, known-hosts lines for the remote's host only; `GitConfig` → `/data/home/.gitconfig` (subPath); `SignIn(cli)` → the provider's `sandbox_auth_file` under `HOME` (subPath); `RegistryPull { auths }` → one `imagePullSecrets` entry of type `kubernetes.io/dockerconfigjson` named `<base>-pull`; a missing source file → `CredentialSource { credential, path }`; there is no SSH-agent credential.
- [ ] T064 [P] [US3] [U43] [U44] Security manifest tests in `core/kube/manifest.rs`: pod `securityContext` `runAsNonRoot: true`, `runAsUser: 10001`, `runAsGroup: 10001`, `fsGroup: 10001`, `seccompProfile: RuntimeDefault`; container `readOnlyRootFilesystem: true`, `allowPrivilegeEscalation: false`, `capabilities.drop: [ALL]`; every secret volume mount corresponds to one `allowed` entry; `allowed` empty → no secret volume and no `imagePullSecrets` (SC-004).
- [ ] T065 [P] [US3] [A10] [A11] [A12] [A13] [U58] [U59] `Secrets` phase tests over `FakeCluster` in `core/kube/start.rs`: allowed secrets are created (recorded) or adopted; a withdrawn secret that is ours is deleted by name and UID; one that is not ours is untouched; none allowed → no secret calls; source files are read only in this phase; the pod the start creates mounts exactly the allowed secrets (none when nothing is allowed) and carries T064's security contexts (US3.1–3.4).
- [ ] T066 [P] [US3] [U104] Reducer tests in `client-tests/features_daemons.rs`: the allowed-credentials checkbox list, nothing checked by default, SSH key and registry auth requiring a file.

### Implementation

- [ ] T067 [P] [US3] [U66] [U67] [U68] [U69] [U70] `KubeCredential` → secret, data, mount, env in `core/kube/credentials.rs`.
- [ ] T068 [US3] [U43] [U44] Security contexts, secret volumes, mounts, env and `imagePullSecrets` in `core/kube/manifest.rs`.
- [ ] T069 [US3] [A10] [A11] [A12] [A13] [U58] [U59] Secrets reconcile in the `Secrets` phase of `core/kube/start.rs`, with the client reading source files in `client/daemon.rs` only during that phase.
- [ ] T070 [US3] [U104] Allowed-credentials list with file pickers in `client/features/daemons.rs` and `client/ui/settings/daemons.rs`; geometry gate.
- [ ] T071 [US3] User guide `docs/user-guide/kubernetes-daemons.md`: what each allowed credential creates and where it is mounted, withdrawal, the pod's security settings.
- [ ] T102 [US3] [A10] [A11] [A12] [A13] Acceptance (M5): the `core/kube/start.rs` start-sequence tests for A10–A13 green over `FakeCluster`.

---

## Phase 6: User Story 4 — Failures are reported with their reason, not as a hang (P1)

**Goal**: every start failure names its cause within its phase bound and leaves nothing this attempt
created.

**Independent test**: provoke RBAC denial, image pull failure and an unschedulable pod; read state and
reason in Settings → Daemons.

### Tests (write first, must fail)

- [ ] T072 [P] [US4] [U30] [U31] [U32] [U33] `KubeFailure` classifier tests in `core/kube/classify.rs`: `from_status` (403 → `Forbidden { verb, resource }`, namespace 404 → `NamespaceMissing`, connect error → `Unreachable { server, error }`); `from_pod` (`ErrImagePull`, `ImagePullBackOff`, `InvalidImageName` → `ImagePull` on first sight; `PodScheduled=False, reason=Unschedulable` → `Unschedulable { message }`); `from_pvc` (`ProvisioningFailed` event or "unbound immediate PersistentVolumeClaims" → `ClaimNotBound`); each classified failure's text is the contract §6 one.
- [ ] T073 [P] [US4] [U60] [U61] [U62] Start tests over `FakeCluster` in `core/kube/start.rs`: `Preflight` runs access reviews for `create pods`, `create persistentvolumeclaims`, `create secrets` (only when `allowed` is not empty), `create pods/portforward`, `delete pods`, and a denial fails `Forbidden` before any create (US4.1); `Config` failures → `NoKubeconfig { looked_in }`, `ContextMissing { context, looked_in }`, `ExecPlugin`; a pod failing `ImagePull` or `Unschedulable`, or a PVC failing `ClaimNotBound`, fails the start on first sight, before the phase bound.
- [ ] T074 [P] [US4] [A14] [A15] [A16] [A17] [A18] [U116] Client tests over `FakeCluster` in `client-tests/kube_daemon_failures.rs`: each failure → `Unreachable` with its reason text and no retry; the reason and the full output reach the daemon's diagnostics; only a pre-existing PVC remains; another daemon's state is unchanged (US4.5, FR-019).

### Implementation

- [ ] T075 [US4] [U30] [U31] [U32] [U33] `from_status`, `from_pod`, `from_pvc` classifiers in `core/kube/classify.rs`.
- [ ] T076 [US4] [U60] [U61] [U62] [A14] [A15] [A16] [A17] Preflight access reviews (`manifest::access_reviews`), `Config` failures, and the classifiers applied in the `Volume`, `Schedule` and `Pull` phases in `core/kube/start.rs` and `core/kube/manifest.rs`.
- [ ] T077 [US4] [A18] [U116] Report failures and build output through `DaemonDiagnostics::push_log` in `client/daemon.rs`.
- [ ] T078 [US4] User guide `docs/user-guide/kubernetes-daemons.md`: a troubleshooting table of the contract §6 texts and what to do about each.
- [ ] T103 [US4] [A14] [A15] [A16] [A17] [A18] Acceptance (M6): `kube_daemon_failures` green for A14–A18.

---

## Phase 7: User Story 5 — Manage a Kubernetes daemon in Settings (P2)

**Goal**: add, edit, start, stop, reconnect, remove with 491's row and actions.

**Independent test**: add, edit resources, start, stop (volume kept), start (worktrees present),
remove (pod and volume gone after confirmation).

### Stop and Remove (shipped with M1)

- [ ] T079 [P] [US5] [A21] [A24] [U63] [U64] [U65] Lifecycle tests over `FakeCluster` in `core/kube/lifecycle.rs`: Stop deletes the pod by name and UID with grace 10 s and waits until gone (120 s), PVC and secrets kept; Remove deletes pod, secrets and PVC (ours only; `404` = done); the forget-locally list is the deterministic names (K5) and makes no cluster call.
- [ ] T080 [P] [US5] [A23] [U105] [U106] Confirmation reducer tests in `client-tests/features_daemons.rs`: Stop with running sessions "Stop <name>? <n> running session(s) will end. Worktrees on its volume are kept."; Remove "Remove <name>? This deletes from <context>/<namespace>: pod <pod>, volume <pvc> with its worktrees, secrets <list>." plus 491's bound-worktree counts; cluster unreachable → "The cluster could not be reached (<error>). Forget <name> on this computer only? These stay in <context>/<namespace>: <pod>, <pvc>, <secrets>."; nothing is deleted before confirmation.
- [ ] T081 [US5] [A21] [A24] [U63] [U64] [U65] Stop, Remove and forget-locally sequences in `core/kube/lifecycle.rs`.
- [ ] T082 [US5] [A23] [U105] [U106] Wire Stop, Remove and Forget locally for Kubernetes daemons in `client/daemon.rs`, `client/shell/daemons.rs` and `client/features/daemons.rs`; confirmation texts in `client/ui/confirm_daemon_removal.rs`; worktrees follow 491 FR-014.
- [ ] T099 [US1] [A1] [A4] [A5] [A21] [A23] [A24] Acceptance (M1): `kube_daemon_start`, `features_daemons` and the `core/kube/lifecycle.rs` tests for A1, A4, A5, A21, A23, A24 green over `FakeCluster`; the milestone is not complete before.

### The rest of the form and actions

- [ ] T083 [P] [US5] [A19] [A20] [A22] [U107] [U108] Reducer tests in `client-tests/features_daemons.rs`: node selector, tolerations, volume size and storage class fields, each refused naming the field when invalid; editing a running daemon saves and shows "applies on next start" (US5.2); Reconnect re-dials (US5.4); Logs fetches the last 200 lines on demand.
- [ ] T084 [P] [US5] [U109] [U110] Parse/format tests for the key/value list and toleration editors in `client/ui/material/key_value_list.rs` and `client/ui/material/toleration_list.rs`.
- [ ] T085 [US5] [U109] [U110] Key/value list and toleration editors with builder `.into()` APIs in `client/ui/material/key_value_list.rs` and `client/ui/material/toleration_list.rs`, registered in `client/ui/material/mod.rs`.
- [ ] T086 [US5] [A19] [A20] [A22] [U107] [U108] Node selector, tolerations, volume size, storage class, "applies on next start", Reconnect and Logs (pod log tail via `Cluster::logs`) in `client/features/daemons.rs`, `client/shell/daemons.rs` and `client/ui/settings/daemons.rs`; geometry gate.
- [ ] T087 [US5] User guide `docs/user-guide/kubernetes-daemons.md` and `docs/user-guide/settings.md`: every field, edit applies on next start, Reconnect, Logs, Forget locally.
- [ ] T104 [US5] [A19] [A20] [A22] Acceptance (M7): `features_daemons` green for A19, A20, A22.

---

## Phase 8: User Story 6 — Proven end to end in CI (P2)

**Goal**: a throwaway kind cluster with a registry in the job drives the real path.

**Independent test**: `mise run test-kube` with docker, kind and kubectl installed; all pass.

- [ ] T088 [US6] Harness `client-tests/kube_real_support/mod.rs` behind feature `kube-real-runtime`: skips with a printed reason when docker, `kind` or `kubectl` is absent; creates kind cluster `micold-kube-test` with a `registry:2` container on `localhost:5001` mirrored into containerd (kind's local-registry recipe), deletes it unless `MICOLD_KUBE_TEST_KEEP=1`; RBAC fixtures: kubeconfig contexts for ServiceAccounts whose Role lacks `create pods`, and one lacking `pods/portforward` (`kubectl create token`); a git remote reachable from the pod for the clone.
- [ ] T089 [P] [US6] `packaging/kube/test.Containerfile`: the app's template plus the stand-in AI CLI installed as `claude`.
- [ ] T090 [P] [US6] [A3] [A25] `client-tests/kube_real_endtoend.rs`: build → push → pull by digest, tunnel, clone and worktree, a session's input and output with the stand-in CLI, 0 Services and 0 Ingresses (SC-006); the time from Start to typed input, minus the `Image` and `Pull` phases, is asserted under 5 minutes (SC-001).
- [ ] T091 [P] [US6] [A25] `client-tests/kube_real_reconnect.rs`: drop the tunnel, reconnect, quit the client for an away window (`MICOLD_KUBE_AWAY_SECS`, default 60 in CI, 600 for the SC-002 run) and reopen: every session still running with its output since the drop; `kubectl delete pod` → `stopped` "the pod was deleted", no new pod (SC-002).
- [ ] T092 [P] [US6] `client-tests/kube_real_security.rs`: the live pod's security context and mounted secrets equal the allowed set; withdrawal deletes the secret (SC-004); a remote needing the allowed git credential clones.
- [ ] T093 [P] [US6] [A26] `client-tests/kube_real_failures.rs`: RBAC `create pods` and `pods/portforward` denial, a digest absent from the registry, a node selector no node has; each reason within its bound, nothing left (SC-003).
- [ ] T094 [P] [US6] [A25] `client-tests/kube_real_lifecycle.rs` and `client-tests/kube_real_concurrent.rs`: stop keeps the PVC and the worktree after restart; remove leaves 0 labelled objects (SC-005); two concurrent starts → one pod (FR-023).
- [ ] T095 [US6] [A27] `mise.toml` task `test-kube` (build-locked, `cargo test -p micold-client --features kube-real-runtime kube_real_ --no-fail-fast -- --test-threads=1`).
- [ ] T096 [US6] [A27] CI job `kube-cluster` in `.github/workflows/ci.yml` beside `ssh-remote`, same docs-only skip, `kind` and `kubectl` at pinned versions with checksums, Linux only with the reason (K13) in a comment; `docs/development/ci-pipeline.md` describes it.
- [ ] T105 [US6] [A3] [A25] [A26] [A27] Acceptance (M8): `mise run test-kube` green for A3, A25, A26 on a kind cluster, and A27 (off by default, on in CI's `kube-cluster`).

---

## Phase 9: Polish

- [ ] T097 Run quickstart Part B at a display (the `visual-pass` skill) and save screenshots under `specs/686-kubernetes-daemon-runtime/visual-pass/`.
- [ ] T098 Re-run `speckit-converge` against the code and tick what is done.

---

## Dependencies & Execution Order

- Setup (T001–T003) → Foundational (T004–T018) → every story.
- US1: 3a (T019–T033) → 3b (T034–T046) → 3c (T047–T055). 3b and 3c are independent of each other
  once 3a is done; 3c needs a connected daemon only.
- US5 Stop and Remove (T079–T082) depend on 3a only and ship with it.
- US2 (T056–T062) depends on 3a. US3 (T063–T071) depends on 3a (manifest, start). US4 (T072–T078)
  depends on 3a, and on 3b for the `Image` phase failures.
- US5 rest (T083–T087) depends on 3a–3b (the form). US6 (T088–T096) depends on every story.
- Within a phase: tests → core types → core sequences → client adapters → client reducers → UI glue
  → docs.

## Parallel Examples

- Foundational tests T004–T010 together (seven files).
- 3a tests T019–T024 together; then T025 and T026 together.
- 3b: T034–T039 together; then T040, T041, T042 together.
- 3c: T047–T050 together; T051 beside T052.
- US6: T089–T094 together once T088 exists.

## Implementation Strategy

MVP is M1: a Kubernetes daemon added in Settings with a given image starts, connects through the
tunnel, and can be stopped and removed. Each further milestone adds one deliverable: the built
image (M2), worktrees and sessions in the pod (M3), surviving the client (M4), allowed credentials
only (M5), failure reasons (M6), the full form (M7), the real-cluster CI proof (M8). Polish (T097,
T098) changes no code and is left to the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). Polish T097 (quickstart Part B visual pass) and T098 (`speckit-converge` re-run) are in no
milestone: they change no code, so the close unit does them in the close PR.

### M1 — Start, connect, stop and remove a Kubernetes daemon 🎯 MVP

- **Tasks**: T001–T033, T079–T082, T099
- **Deliverable**: Settings → Daemons → Add offers runtime Kubernetes (context, namespace, image reference); Start creates or adopts the daemon's PVC and pod and reaches `connected` through the API tunnel; Stop deletes the pod and keeps the volume; Remove deletes what it created after confirmation.
- **Satisfies**: US1 scenarios 1, 4, 5 (given image); US5 scenarios 3, 5, 6; FR-001, FR-002, FR-003, FR-006, FR-013, FR-014, FR-015, FR-016, FR-017, FR-019, FR-020, FR-022, FR-023
- **Verify**: `mise run test-core`; `cargo test -p micold-client --test kube_daemon_start --test features_daemons --test daemons_section_geometry --test no_concrete_implementations`; `cargo test -p micold-client --lib kube::live`; `cargo test -p micold-daemon server::`
- **Depends on**: —
- **Tier**: full

### M2 — Build and push the daemon image; set resources

- **Tasks**: T034–T046
- **Deliverable**: a Kubernetes daemon with image source Build (the default) builds the image with the local runtime, pushes it to the named registry and runs the pod by digest, with live output and Cancel; CPU and memory requests and limits reach the pod.
- **Satisfies**: US1 scenario 1 (default image path); FR-004, FR-005 (requests and limits)
- **Verify**: `mise run test-core` (`daemon_image::`, `sandbox::cli::`, `kube::manifest::`); `cargo test -p micold-client --lib kube::image`; `cargo test -p micold-client --test features_daemons`
- **Depends on**: M1
- **Tier**: full

### M3 — Worktrees and sessions in the pod

- **Tasks**: T047–T055, T100
- **Deliverable**: creating a worktree bound to a Kubernetes daemon clones the project's remote (one needing no credential; M5 lets the user allow a git credential, and M8's T092 proves a credentialed remote clones with it) into the daemon's volume and creates the worktree there, shown with the daemon's name and in-pod path; a project without a remote is refused with the reason.
- **Satisfies**: US1 scenarios 2, 3; FR-007
- **Verify**: `mise run test-core` (`kube::clone::`, `protocol::`); `cargo test -p micold-daemon repo_clone`; `cargo test -p micold-client --test features_daemons`
- **Depends on**: M1
- **Tier**: full

### M4 — Sessions outlive the client

- **Tasks**: T056–T062, T101
- **Deliverable**: a dropped tunnel reconnects to the same pod with sessions intact; a deleted, evicted or node-lost pod shows `stopped` with its reason, sessions `lost to its daemon`, and is not recreated until Start.
- **Satisfies**: US2 scenarios 1–4; FR-010, FR-011
- **Verify**: `cargo test -p micold-client --test kube_daemon_reconnect`; `mise run test-core` (`kube::classify::`, `daemons::`)
- **Depends on**: M1
- **Tier**: full

### M5 — Only allowed credentials reach the pod

- **Tasks**: T063–T071, T102
- **Deliverable**: allowed credentials and sign-ins are created as secrets and mounted read-only, withdrawn ones are deleted on the next start, and the pod runs non-root with a read-only root filesystem.
- **Satisfies**: US3 scenarios 1–4; FR-008, FR-009
- **Verify**: `mise run test-core` (`kube::credentials::`, `kube::manifest::`, `kube::start::`); `cargo test -p micold-client --test features_daemons`
- **Depends on**: M1
- **Tier**: full

### M6 — Failures named within their bound

- **Tasks**: T072–T078, T103
- **Deliverable**: RBAC denial, missing context or namespace, unreachable cluster, credential-plugin failure, push and pull failures, unschedulable pods and unbound volumes each show their contract §6 reason, RBAC denial before anything is created and pull or scheduling failures on first sight.
- **Satisfies**: US4 scenarios 1–5; FR-012
- **Verify**: `mise run test-core` (`kube::classify::`, `kube::start::`); `cargo test -p micold-client --test kube_daemon_failures`
- **Depends on**: M1, M2
- **Tier**: full

### M7 — The full Kubernetes form and actions

- **Tasks**: T083–T087, T104
- **Deliverable**: the Kubernetes form edits node selector, tolerations, volume size and storage class; edits apply on next start; Reconnect and Logs work from the row.
- **Satisfies**: US5 scenarios 1, 2, 4; FR-005 (node selector, tolerations, volume size), FR-018
- **Verify**: `cargo test -p micold-client --test features_daemons --test daemons_section_geometry`; `cargo test -p micold-client --lib ui::material::key_value_list`; `cargo test -p micold-client --lib ui::material::toleration_list`
- **Depends on**: M1, M2
- **Tier**: full

### M8 — Real-cluster suite in CI

- **Tasks**: T088–T096, T105
- **Deliverable**: `mise run test-kube` runs the `kube_real_*` suite on a throwaway kind cluster with a registry, and CI's `kube-cluster` job runs it on every non-docs change; on a live pod, a remote needing an allowed git credential clones with it (T092).
- **Satisfies**: US6 scenarios 1–3; FR-021; SC-001–SC-006 evidence
- **Verify**: `mise run test-kube`; CI job `kube-cluster` green on the PR
- **Depends on**: M1–M7
- **Tier**: full
