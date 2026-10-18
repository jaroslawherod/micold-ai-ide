# Tasks: Kubernetes session daemon runtime

**Input**: Design documents from `/specs/686-kubernetes-daemon-runtime/` (revision 2)

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/kube-runtime.md](./contracts/kube-runtime.md),
[contracts/shared-ssh-image.md](./contracts/shared-ssh-image.md), [quickstart.md](./quickstart.md)

**Tests**: Mandatory (Constitution I). In every phase the test tasks come first and must fail before
the implementation tasks after them. Core rules are `#[cfg(test)]` unit tests in the module named;
sequences run over `FakeCluster` and `FakeKeyGen` (`micold-core` feature `kube-fake`); the
real-cluster suite is US6.

**Documentation**: milestones before the Add entry (M1a–M4b) change no user-visible behaviour
(`docs-not-needed`, plan Constitution VII). The user guide `docs/user-guide/kubernetes-daemons.md`
is written in M6, the milestone that shows the Add entry, and extended by M7 and M8.

**Behavior markers**: `[A<n>]` and `[U<n>]` are the behavior ids of [tdd/test-list.md](./tdd/test-list.md).
`/speckit.tdd.run` ticks a task once every behavior it carries is done; a test task must be seen
failing for the right reason before the implementation task with the same ids starts. Each
milestone with outer-loop behaviors ends with an acceptance task: those behaviors green before it
is complete. Before #806 the highest level that runs is the composed core sequence over
`FakeCluster`; M5/M6 confirm the same behaviors at client level.

**Visual pass**: each milestone that changes the Settings form (M4a, M4b, M6) runs the `visual-pass`
skill at its gate on the parts it adds; T097 is the full quickstart Part B pass at the end.

**Cross-platform**: the platform-dependent rules (kubeconfig separator, home, exec search path) are
pure functions taking the platform, tested on all three CI platforms (Constitution VI, research K13).

**Blocked on #806**: tasks marked **(#806)** call the shared SSH image mechanism
([contracts/shared-ssh-image.md](./contracts/shared-ssh-image.md)); they use the names #806 ships.
Milestones M5–M9 start only after #806 merges. Everything before them runs without it (contract §5).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US6) the task serves

Paths: `core/` = `crates/micold-core/src/`, `client/` = `crates/micold-client/src/`,
`client-tests/` = `crates/micold-client/tests/`, `daemon/` = `crates/micold-daemon/src/`.

## Revision 2 task map

Revision 2 (ledger decisions 8–11) replaced the image build/push and the in-pod token-less listener
with #806's SSH image. Task ids are kept for history; new tasks start at T106. T137 is unused.

`stash@{0}` (cited by tasks below) is branch `claude/project-thread-xelisc`'s stash "686 M1a red WIP",
backed up in `/mnt/project-files/backups/686-m1-wip.bundle`; the ledger's Cut notes list what of it
survives. Where it is absent, write the named code from scratch.

| Old | Status | Where now |
|---|---|---|
| T001, T002, T006, T013 | kept, done | as is |
| T003 | done; `core/daemon_image/` removed | T111 |
| T004, T011 | done; rework (drop `registry`, `arch`; add `image`, `providers`, `resources`) | T107, T112 |
| T005, T012 | done; `DaemonImageSource` removed, `DaemonResources` moved | T106, T111 |
| T007, T014 | done; rework (access secret, `all_secrets`) | T108, T113 |
| T008, T016, T010, T017 | done; **removed** (research K2, K4) | reverted by T116 |
| T009, T018 | done; rework (revision-2 variant list) | T109, T114 |
| T015 | done; rework (`replace`, port 2222, no `node_arch`) | T110, T115 |
| T019, T021, T025, T027 | rewritten in place | M1b |
| T020, T026, T023, T029, T079, T081 | kept (T023/T028 port 2222) | M1a, M1c, M2 |
| T022, T024, T028, T030–T033 | rewritten in place | M1c, M4a, M5, M6 |
| T034–T046 | **removed** (no build, push, registry, arch) | resources parts: T019, T130, T131 |
| T047–T055, T056–T062, T063–T078, T080, T082–T096 | kept or rewritten in place | see each task |
| T099, T103, T104 | **removed** (milestone acceptance re-cut) | T121, T128, T129, T136, T140 |

---

## Phase 1: Setup (done)

- [x] T001 Add workspace dependencies `kube` 4.2 (`default-features = false`; features `client`, `config`, `rustls-tls`, `ring`, `ws`) and `k8s-openapi` 0.28 (feature `v1_32`) in the root `Cargo.toml` and use them from `crates/micold-client/Cargo.toml` only; add `[features] kube-real-runtime = []` to `crates/micold-client/Cargo.toml`. Confirm `cargo tree -p micold-client -i openssl-sys` prints nothing and both crates are Apache-2.0 (research K0); `micold-core` and `micold-daemon` gain no new dependency.
- [x] T002 Add feature `kube-fake = []` to `crates/micold-core/Cargo.toml` (comment: exposes `kube::fake::{FakeCluster, FakeKeyGen}` to client tests) and enable it on the `micold-core` entry of `[dev-dependencies]` in `crates/micold-client/Cargo.toml`.
- [x] T003 [P] Create empty modules `core/kube/mod.rs` and `core/daemon_image/mod.rs` registered in `core/lib.rs`, and `client/kube/mod.rs` registered in `client/lib.rs`. (`daemon_image` is removed by T111.)

---

## Phase 2: Foundational — revision-2 core types and kubeconfig (M1a)

**Purpose**: the persisted settings, resource identity, the cluster port and its fake, failure texts,
and kubeconfig discovery; revision 1's pieces reverted.

### Done in revision 1 (history; reworked below)

- [x] T004 `KubeSettings` validation tests (revision 1 fields) in `core/kube/settings.rs`.
- [x] T005 `DaemonImageSource` and `DaemonResources` tests in `core/daemon_image/mod.rs`.
- [x] T006 [U13] [U14] [U15] [U16] `DaemonRuntime::Kubernetes` tests in `core/daemons.rs`: serde round trip as `{"kind":"kubernetes", ...}`, no `settings_version` bump; unknown-kind reader keeps `Unsupported` verbatim; `label()` "Kubernetes"; `is_startable()`; endpoint `<context> / <namespace> / <pod name>`; duplicate name → `RegistryError::DuplicateName` (form text names it, decision 6).
- [x] T007 Name and ownership tests in `core/kube/names.rs` (revision 1 names).
- [x] T008 `listener_auth` tests in `core/protocol/auth.rs` — **removed** (K4), reverted by T116.
- [x] T009 `KubeFailure` text tests (revision 1 variants) in `core/kube/classify.rs`.
- [x] T010 `Transport::Tunnel` test in `core/connect.rs` — **removed** (K2), reverted by T116.
- [x] T011 `KubeSettings` and newtypes in `core/kube/settings.rs` (revision 1).
- [x] T012 `DaemonImageSource` and `DaemonResources` in `core/daemon_image/mod.rs`.
- [x] T013 [U13] [U14] [U15] [U16] `DaemonRuntime::Kubernetes(KubeSettings)`, `label`, `is_startable`, endpoint text in `core/daemons.rs`.
- [x] T014 `DaemonKey`, `slug`, `ResourceNames`, `is_ours` in `core/kube/names.rs` (revision 1).
- [x] T015 `Cluster` port and `FakeCluster` in `core/kube/mod.rs` and `core/kube/fake.rs` (revision 1: `node_arch`, port 7727).
- [x] T016 `listener_auth` and its use in `daemon/server.rs` — **removed**, reverted by T116.
- [x] T017 `Transport::Tunnel` — **removed**, reverted by T116.
- [x] T018 `KubeFailure` with `Display` in `core/kube/classify.rs` (revision 1 variants).

### Tests (write first, must fail)

- [x] T106 [P] [U11] [U12] `DaemonResources` tests in `core/daemon_resources.rs` (moved from `core/daemon_image/mod.rs`): defaults `cpu_request: None`, `cpu_limit: Some(sandbox::DEFAULT_MILLI_CPUS)`, `memory_request: None`, `memory_limit: Some(sandbox's default memory)`; each of the four set or unset on its own; a request equal to its limit accepted, one above it refused naming the request field.
- [x] T107 [P] [U1] [U2] [U5] [U6] [U7] [U8] [U117] [U118] [U119] `KubeSettings` revision-2 tests in `core/kube/settings.rs` (data model "KubeSettings"; each refusal `InvalidField` naming the field, FR-018): `image: Option<ImageRef>` is `None` by default and a given reference is kept as is, a moving tag flagged as `sandbox::image` flags it; `providers` defaults to the providers enabled in Settings and may be empty; `resources: DaemonResources`; no `registry` or `arch` field; `Quantity::{to_k8s, parse_k8s}` (write the largest exact binary unit, read binary, decimal or no suffix, round trip equal — from stash@{0}).
- [X] T108 [P] [U18] [U19] [U20] [U21] [U120] Name tests in `core/kube/names.rs`: the revision-1 key, slug and `is_ours` tests stay as they are; `ResourceNames.access_secret` = `<base>-ssh-access`; `all_secrets(&allowed)` lists the access secret first, then one name per allowed credential; `ObjectKind::noun` (from stash@{0}).
- [x] T109 [P] [U26] `KubeFailure` text tests in `core/kube/classify.rs`: every revision-2 variant of the data model except `Provision` (added in M5) — `ContextMissing`, `NoKubeconfig`, `Unreachable`, `ExecPlugin`, `NamespaceMissing`, `Forbidden`, `ImagePull`, `Unschedulable`, `ClaimNotBound`, `Conflict`, `CredentialSource`, `ImageContract`, `HostKeyMismatch`, `PhaseTimeout`, `Cancelled`, `StopTimeout`, `Api`, `Unavailable` — displays exactly its contract §6 text (`Unavailable`: data model `StartOutcome` text); `NoRuntime`, `Build`, `Push`, `VolumeTooSmall` no longer exist.
- [x] T110 [P] [U121] `FakeCluster` tests in `core/kube/fake.rs`: `replace` with the current `resourceVersion` stores the object under a new one; with a stale one answers `ApiFailure` 409 and changes nothing; `hold_on_delete` keeps a deleted pod terminating (`deletionTimestamp` set) until released (from stash@{0}); `port_forward(namespace, pod, port)` records the port.
- [x] T143 [U13] [U14] [U15] [U16] Rework the persistence test `core/tests/daemons_kube.rs` to revision-2 settings (no `DaemonImageSource`; the revision-2 `KubeSettings` with `image`, `providers`, `resources`): the runtime round-trips through `settings.json` as `{"kind":"kubernetes", ...}` without a `settings_version` bump, an unreadable entry stays `Unsupported` verbatim, label/endpoint and the duplicate-name refusal as before. Before T111/T112.
- [X] T020 [P] [US1] [U45] [U46] [U47] [U48] Kubeconfig tests in `core/kube/kubeconfig.rs` (registered as `pub mod kubeconfig;` in `core/kube/mod.rs`), parameterised over Linux, macOS and Windows (FR-022): `candidates` splits `KUBECONFIG` on `;` (Windows) or `:` (else), else `<home>/.kube/config` with home `%USERPROFILE%` (Windows) or `$HOME`; `search_path(process_path, login_shell_path, home, platform)` is the process `PATH`, then the login shell's `PATH` (absent on Windows), then `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/google-cloud-sdk/bin`, de-duplicated keeping the first; `parse_login_shell_output` returns the text between the markers, `None` without them. (Tests in stash@{0}.)

### Implementation

- [x] T111 [U11] [U12] `DaemonResources` in `core/daemon_resources.rs` (new, shared with #806), registered in `core/lib.rs`; delete `core/daemon_image/` (`DaemonImageSource`, `ImageInputs`) and its registration.
- [x] T112 [U1] [U2] [U5] [U6] [U7] [U8] [U117] [U118] [U119] `KubeSettings` revision 2 in `core/kube/settings.rs`: remove `RegistryRepo`, `NodeArch`, `registry`, `arch`; add `image: Option<ImageRef>`, `providers: BTreeSet<AiCli>`, `resources: DaemonResources`; `Quantity::{to_k8s, parse_k8s}`. `core/daemons.rs` and its tests follow (depends on T111).
- [X] T113 [P] [U18] [U19] [U20] [U21] [U120] `ResourceNames.access_secret`, `all_secrets`, `ObjectKind::noun` in `core/kube/names.rs` and `core/kube/mod.rs`.
- [x] T114 [P] [U26] `KubeFailure` revision 2 in `core/kube/classify.rs` (add `ImageContract`, `HostKeyMismatch`, `StopTimeout`, `Api`, `Unavailable`; remove `NoRuntime`, `Build`, `Push`, `VolumeTooSmall`; `From<ApiFailure>` from stash@{0}).
- [x] T115 [U121] `Cluster` port revision 2 in `core/kube/mod.rs`: add `replace(namespace, kind, name, resource_version, manifest)`; `port_forward(namespace, pod, port)`; remove `node_arch`. `FakeCluster` gains `replace` and `hold_on_delete` in `core/kube/fake.rs`. `StartPhase::Image` in `core/kube/start.rs` stays until T027 (M1b) replaces the phase list.
- [x] T116 Revert revision 1's listener auth and tunnel transport (research K2, K4): remove its additions (`ListenerAuth`, `AuthConfigError`, `listener_auth`) from `core/protocol/auth.rs`, which stays as feature 027's token module; restore `core/protocol/auth.rs`, `daemon/server.rs` and `core/connect.rs` to `origin/main` (`git diff origin/main -- crates/micold-core/src/protocol/auth.rs crates/micold-daemon/src/server.rs crates/micold-core/src/connect.rs` is empty); feature 027's empty-token-is-fatal tests stay green.
- [X] T026 [P] [US1] [U45] [U46] [U47] [U48] `candidates`, `search_path`, `parse_login_shell_output` in `core/kube/kubeconfig.rs`, registered in `core/kube/mod.rs` (stub in stash@{0}).

**Checkpoint**: `mise run test-core` green; a revision-2 Kubernetes daemon round-trips through `settings.json`.

---

## Phase 3: Pod, volume and access secret (US1, US5; M1b)

**Goal**: the start sequence creates or adopts the daemon's PVC, SSH access secret and pod, with
per-phase bounds and rollback, over `FakeCluster` and `FakeKeyGen`.

### Tests (write first, must fail)

- [x] T019 [P] [US1] [U35] [U36] [U37] [U38] [U39] [U40] [U41] Manifest tests in `core/kube/manifest.rs` for `pod(..)`, `pvc(..)`, `secret(..)` (contract §1, research K7): never a `Service` or `Ingress` kind; no `hostPort`, `hostNetwork`, `hostPID`, `privileged`, `containerPort`; `restartPolicy: Always`, `automountServiceAccountToken: false`, `enableServiceLinks: false`; one container, image = the given image or the default passed in, command left to the image; pod env only `HOME=/data/home`, `TMPDIR=/tmp` and the access-secret directory (shared-ssh-image §1); the access secret mounted read-only at `/run/micold/ssh` (`defaultMode: 0440`); the PVC at `/data`, an `emptyDir` at `/tmp`; the PVC `ReadWriteOnce` with `volume_size` and `storage_class` (omitted when none); both ownership labels and the name annotation on every object; `nodeSelector` and `tolerations` as given; `resources.requests` / `.limits` hold exactly the set `DaemonResources` fields (`<n>m`, `Quantity::to_k8s`). Starts from stash@{0}'s `manifest.rs` (replace port 7727, the daemon env and the listener env).
- [x] T117 [P] [US5] [U122] [U123] `PodStamp` and `drift` tests in `core/kube/manifest.rs`: the stamp is the canonical JSON of image, resources, node selector, tolerations and allowed credentials, equal for equal settings, written as annotation `micold.dev/pod-settings`; `drift` names the differing settings in form order and is empty when equal (FR-005, US5.2).
- [x] T118 [P] [US1] [U124] [U125] `AccessSecret` tests in `core/kube/access.rs` (research K3): `parse` / `to_data` round trip over keys `ssh_host_ed25519_key`, `ssh_host_ed25519_key.pub`, `authorized_keys` (one line per client, comment `micold-client:<client id>`); `with_client` is idempotent, replaces a line with the same client id and reports whether it changed; `without_client` removes only that line; `FakeKeyGen` returns fixed pairs.
- [x] T021 [P] [US1] [U49] [U50] [U51] [U52] [U53] [U54] [U55] [U57] [U126] [U127] [U128] Start sequence tests over `FakeCluster` and `FakeKeyGen` in `core/kube/start.rs` (contract §2–§3): `StartPlan::kube()` is `Config, Preflight, Volume, Secrets, Pod, Schedule, Pull, Tunnel, Copy, Install, Connect`; `bound()` is 15 min for `Pull` and `Install`, 120 s for every other, and with a paused clock each phase run here fails `PhaseTimeout { phase }` at its bound and not a tick before; a missing namespace → `NamespaceMissing`, nothing created; the PVC is created on first start and reused on the next; a `201` is recorded in `Attempt.created`; `409` + ours → adopted, not recorded; `409` + not ours → `Conflict { kind, name }`; a pod with `deletionTimestamp` is waited out, then created; the first start creates the access secret with a generated host key and this client's key; another client's start adds its key by `replace` with the `resourceVersion` it read, a `409` re-reads and keeps the other key, a key already present writes nothing; an adopted pod whose stamp differs ends the start with `Drift { fields }`, Recreate deletes it as Stop does and creates a new one, Keep continues; any failure rolls back `Attempt.created` in reverse by name and UID (the PVC only when this attempt created it, adopted objects never) and removes a key this attempt added. Each phase sets its `activity`. Starts from stash@{0}'s `start_tests.rs` (drop the `Image` phase and the volume-size refusal).

### Implementation

- [x] T025 [US1] [U35] [U36] [U37] [U38] [U39] [U40] [U41] `pod`, `pvc`, `secret` builders returning `serde_json::Value` in `core/kube/manifest.rs`, constants `SSH_PORT = 2222`, `IMAGE_USER_UID = 10001`, mount paths (security contexts and credential mounts come in M3).
- [x] T119 [P] [US5] [U122] [U123] `PodStamp`, `drift` in `core/kube/manifest.rs`.
- [x] T120 [P] [US1] [U124] [U125] `AccessSecret`, `KeyPair`, the `KeyGen` port in `core/kube/access.rs`; `FakeKeyGen` in `core/kube/fake.rs` under the `kube-fake` feature.
- [x] T027 [US1] [U49] [U50] [U51] [U52] [U53] [U54] [U55] [U57] [U126] [U127] [U128] `StartPhase`, `bound()`, `activity()`, `StartPlan::kube()`, `Attempt { created, added_key }`, `CreatedResource`, `StartOutcome { Ready, Drift, Failed }` and the sequence over `Cluster` and `KeyGen` through `Pull` in `core/kube/start.rs`, with per-phase timeouts and rollback (preflight reviews and classifiers come in M2, allowed secrets in M3, `Tunnel` in M1c).
- [x] T121 [US1] [A4] [A5] Acceptance (M1b): the `core/kube/start.rs` tests show a second start adopts the pod and creates nothing (A4, core half) and the call log holds no `Service` or `Ingress` (A5, core half).

---

## Phase 4: Tunnel, relay, keys and the live cluster (US1; M1c)

**Goal**: the `Tunnel` phase reaches the pod's SSH server; the client side has the live adapter, the
loopback relay and the owner-only key files.

### Tests (write first, must fail)

- [x] T122 [P] [US1] [U129] `Tunnel` phase tests in `core/kube/start.rs` over `FakeCluster`: it opens `port_forward(namespace, pod, SSH_PORT)` and reads an identification line starting `SSH-2.0-` within its bound → `StartOutcome::Ready`; no line within the bound, a line that is not SSH, or the container exited → `ImageContract { image, missing }` with the contract §6 examples, and rollback runs.
- [x] T023 [P] [US1] [U86] [U87] [U88] [U89] Pure-helper tests in `client/kube/live.rs` `#[cfg(test)]`: `rewrite_exec_commands` turns each non-absolute `users[].exec.command` into the first match on the search path and leaves absolute ones; not found → `ExecPlugin { command, error, search_path }`; contexts are listed from the merged files in order and carry no `users[]` data (FR-002); `to_api_failure(&kube::Error)`: an API `ErrorResponse` keeps its code, reason and message, `409 AlreadyExists` on create is `AlreadyExists`, `409` on replace stays a 409 `ApiFailure`, `404` on delete is done, a connection error is `Unreachable { server, error }`; `delete_params(uid)` sets `Preconditions { uid }` and grace seconds.
- [x] T123 [P] [US1] [U130] Relay tests in `client/kube/relay.rs` over `FakeCluster`'s duplex port-forward: `Relay::open` listens on `127.0.0.1:0`; each accepted connection opens one `port_forward(.., SSH_PORT)` and copies bytes both ways; two connections → two streams; dropping the relay closes the listener and its connections; a refused port-forward closes only that connection.
- [x] T124 [P] [US1] [U131] Key-file tests in `client/kube/keys.rs` over a temp directory: `id_ed25519`, `id_ed25519.pub`, `known_hosts`, `client_id` per daemon key are owner-only (`owner_only.rs`); `known_hosts` is one line `micold-kube-<daemon key> <host public key>`; `client_id` is stable across loads; `delete` removes them all.

### Implementation

- [x] T125 [US1] [U129] The `Tunnel` phase in `core/kube/start.rs`; the sequence ends after it with `Ready { tunnel }` until M5 adds `Copy`, `Install`, `Connect`.
- [x] T028 [US1] [U86] [U87] [U88] [U89] `LiveCluster` in `client/kube/live.rs`: merge candidate files with `kube::config::Kubeconfig`, `rewrite_exec_commands`, a `kube::Client` per context, the `Cluster` operations (incl. `replace`) through `to_api_failure` and `delete_params`, `Api::<Pod>::portforward` on the given port returned as the boxed stream, `logs` with `tail_lines`; the login shell's `PATH` read once per run with a 5 s timeout (skipped on Windows).
- [x] T029 [US1] Choose `LiveCluster` once in `client/shell/capabilities.rs`; `client-tests/no_concrete_implementations.rs` stays green.
- [x] T126 [P] [US1] [U130] `Relay` in `client/kube/relay.rs`.
- [x] T127 [P] [US1] [U131] Key files in `client/kube/keys.rs` with `owner_only.rs`.

---

## Phase 5: Stop, Remove and failure reasons in core (US5, US4; M2)

### Tests (write first, must fail)

- [x] T079 [P] [US5] [A21] [A24] [U63] [U64] [U65] Lifecycle tests over `FakeCluster` in `core/kube/lifecycle.rs`: Stop deletes the pod by name and UID with grace 10 s and waits until gone (120 s, else `StopTimeout`), PVC and secrets kept; Remove deletes pod, secrets (access secret included) and PVC that are ours (`404` = done); the forget-locally list is the deterministic names (K5), the access secret among them, and makes no cluster call. (Tests in stash@{0}.)
- [x] T072 [P] [US4] [U30] [U31] [U32] [U33] `KubeFailure` classifier tests in `core/kube/classify.rs`: `from_status` (403 → `Forbidden { verb, resource }`, namespace 404 → `NamespaceMissing`, connect error → `Unreachable { server, error }`, other → `Api`); `from_pod` (`ErrImagePull`, `ImagePullBackOff`, `InvalidImageName` → `ImagePull` on first sight; `PodScheduled=False, reason=Unschedulable` → `Unschedulable { message }`; the container terminated → `ImageContract`); `from_pvc` (`ProvisioningFailed` event or "unbound immediate PersistentVolumeClaims" → `ClaimNotBound`); texts as contract §6.
- [x] T056 [P] [US2] [U29] `PodGone` table tests in `core/kube/classify.rs` (contract §4): `404` or `deletionTimestamp` → `Deleted`; `status.reason == Evicted` → `Evicted { message }`; `phase == Unknown` or its node `404` → `NodeLost { node }`; present, `Running`, ready → `None`; texts "stopped: the pod was deleted", "stopped: the pod was evicted: <message>", "stopped: the pod's node <node> was lost".
- [x] T073 [P] [US4] [U60] [U61] [U62] Start tests over `FakeCluster` in `core/kube/start.rs`: `Preflight` runs access reviews for `create pods`, `create persistentvolumeclaims`, `create secrets`, `update secrets`, `create pods/portforward`, `delete pods`, and a denial fails `Forbidden` before any create (US4.1); `Config` failures → `NoKubeconfig { looked_in }`, `ContextMissing { context, looked_in }`, `ExecPlugin`; a pod failing `ImagePull` or `Unschedulable`, or a PVC failing `ClaimNotBound`, fails the start on first sight, before the phase bound.

### Implementation

- [x] T081 [US5] [A21] [A24] [U63] [U64] [U65] Stop, Remove and forget-locally sequences in `core/kube/lifecycle.rs` (stub in stash@{0}).
- [x] T075 [US4] [U30] [U31] [U32] [U33] `from_status`, `from_pod`, `from_pvc` in `core/kube/classify.rs`.
- [x] T059 [P] [US2] [U29] `PodGone` and its classifier in `core/kube/classify.rs`.
- [x] T076 [US4] [U60] [U61] [U62] Preflight access reviews (`manifest::access_reviews`), `Config` failures, and the classifiers applied in the `Volume`, `Schedule` and `Pull` phases in `core/kube/start.rs` and `core/kube/manifest.rs`.
- [x] T128 [US4] [A14] [A15] [A16] [A17] [A21] [A24] Acceptance (M2): the `core/kube/start.rs` tests for A14–A17 and the `core/kube/lifecycle.rs` tests for A21, A24 green over `FakeCluster`.

---

## Phase 6: Only what the user allows reaches the pod (US3; M3)

### Tests (write first, must fail)

- [ ] T063 [P] [US3] [U66] [U67] [U68] [U69] [U70] [U136] Credential tests in `core/kube/credentials.rs`: each `KubeCredential` maps to exactly one secret (`ResourceNames`) and one mount: `GitCredentials` → `/run/micold/creds/git-credentials`; `GitSshKey { key }` → `/run/micold/creds/ssh/{id,known_hosts}`, known-hosts lines for the remote's host only; `GitConfig` → `/data/home/.gitconfig` (subPath); `SignIn(cli)` → the provider's `sandbox_auth_file` under `HOME` (subPath); `RegistryPull { auths }` → one `imagePullSecrets` entry of type `kubernetes.io/dockerconfigjson` named `<base>-pull`; a missing source file → `CredentialSource { credential, path }`; no SSH-agent credential. `daemon_env(&allowed)` = `MICOLD_IDLE_STOP=off`, `MICOLD_REPOS_ROOT=/data/repos`, plus `GIT_CONFIG_*` (store helper) for `GitCredentials` and `GIT_SSH_COMMAND` (`IdentitiesOnly=yes`, `StrictHostKeyChecking=yes`) for `GitSshKey`, nothing else.
- [ ] T064 [P] [US3] [U43] [U44] Security manifest tests in `core/kube/manifest.rs`: pod `securityContext` `runAsNonRoot: true`, `runAsUser: 10001`, `runAsGroup: 10001`, `fsGroup: 10001`, `seccompProfile: RuntimeDefault`; container `readOnlyRootFilesystem: true`, `allowPrivilegeEscalation: false`, `capabilities.drop: [ALL]`; the secret volumes are the access secret plus one per `allowed` entry; `allowed` empty → only the access secret and no `imagePullSecrets` (SC-004).
- [ ] T065 [P] [US3] [A10] [A11] [A12] [A13] [U58] [U59] `Secrets` phase tests over `FakeCluster` in `core/kube/start.rs`: allowed secrets are created (recorded) or adopted; a withdrawn secret that is ours is deleted by name and UID; one that is not ours is untouched; none allowed → no credential-secret calls; source files are read through the start's file-reader argument only in this phase; the pod mounts exactly the access secret and the allowed secrets and carries T064's security contexts (US3.1–3.4).

### Implementation

- [ ] T067 [P] [US3] [U66] [U67] [U68] [U69] [U70] [U136] `KubeCredential` → secret, data, mount; `daemon_env` in `core/kube/credentials.rs`.
- [ ] T068 [US3] [U43] [U44] Security contexts, secret volumes, mounts and `imagePullSecrets` in `core/kube/manifest.rs`.
- [ ] T069 [US3] [A10] [A11] [A12] [A13] [U58] [U59] Secrets reconcile in the `Secrets` phase of `core/kube/start.rs`, reading source files through a reader the caller passes (the client's real reader is wired in M5).
- [ ] T102 [US3] [A10] [A11] [A12] [A13] Acceptance (M3): the `core/kube/start.rs` tests for A10–A13 green over `FakeCluster`.

---

## Phase 7: The Kubernetes form, Add entry hidden (US5; M4a)

**Goal**: the form and its reducer exist and are tested; Kubernetes daemons already in settings show
in the list and open in Edit; the Add form's runtime list omits Kubernetes (FR-024, contract §7).

### Tests (write first, must fail)

- [ ] T024 [P] [US5] [U95] [U96] [U134] [U135] Reducer tests in `client-tests/features_daemons.rs`: the Add form's runtime choices omit Kubernetes in a build without the mechanism; a Kubernetes daemon opened in Edit shows the context picker filled from the merged kubeconfig, or "no kubeconfig found; looked in <paths>"; fields namespace, image (empty = the default SSH image), AI CLI checkboxes defaulting to the providers enabled in Settings; editing the volume size of a daemon started before shows "Applies only to a new volume, after Remove"; Save refuses an invalid field naming it; the row shows "Kubernetes" and the endpoint.
- [ ] T130 [P] [US5] [U97] [U99] Resources-field tests in `client-tests/features_daemons.rs`: the container form's limits fields behave as before the extraction (characterization, written first); the Kubernetes form's four resource fields are edited on their own, a request above its limit refused naming the request field.
- [ ] T066 [P] [US3] [U104] Reducer tests in `client-tests/features_daemons.rs`: the allowed-credentials checkbox list, nothing checked by default, SSH key and registry auth requiring a file.

### Implementation

- [ ] T031 [US5] [U95] [U96] [U134] [U135] Kubernetes `Draft`/`Msg` (context, namespace, image, AI CLIs, volume notice) and runtime choices in `client/features/daemons.rs`; kubeconfig context listing off the UI thread in `client/shell/daemons.rs`.
- [ ] T131 [US5] [U97] [U99] Extract the limits fields from `sections` in `client/ui/settings/container_fields.rs` into functions in that file, drawn by the container form and the Kubernetes form (one copy; #806 inherits them, contract shared-ssh-image §5); the four resource `Draft` fields in `client/features/daemons.rs`.
- [ ] T070 [US3] [U104] Allowed-credentials list with file pickers in `client/features/daemons.rs` and `client/ui/settings/daemons.rs`.
- [ ] T032 [US5] [U96] Kubernetes form glue in `client/ui/settings/daemons.rs`; extend `client-tests/daemons_section_geometry.rs` to the Kubernetes form.

---

## Phase 8: The rest of the form and the row actions (US5; M4b)

### Tests (write first, must fail)

- [ ] T083 [P] [US5] [A19] [A20] [U107] [U108] Reducer tests in `client-tests/features_daemons.rs`: node selector, tolerations, volume size and storage class fields, each refused naming the field when invalid; editing a running daemon saves and shows "applies on next start" and sends nothing to the cluster (US5.2); Logs fetches the last 200 lines on demand.
- [ ] T084 [P] [US5] [U109] [U110] Parse/format tests for the key/value list and toleration editors in `client/ui/material/key_value_list.rs` and `client/ui/material/toleration_list.rs`.
- [ ] T080 [P] [US5] [A23] [U105] [U106] [U143] Confirmation reducer tests in `client-tests/features_daemons.rs`: Stop with running sessions "Stop <name>? <n> running session(s) will end. Worktrees on its volume are kept."; Remove names pod, volume with its worktrees, secrets and this computer's SSH key, plus 491's bound-worktree counts; cluster unreachable → contract §5 text naming what stays, the access secret and this computer's still-authorized key among it; nothing is deleted before confirmation; Remove and Forget locally delete this client's key files.
- [ ] T132 [P] [US5] [U133] Actor test in `client-tests/kube_daemon_start.rs` over `FakeCluster`: Start of a Kubernetes daemon in a build without the mechanism fails at once with `Unavailable`, makes no cluster call, and the row shows `unreachable` with that reason.

### Implementation

- [ ] T085 [US5] [U109] [U110] Key/value list and toleration editors with builder `.into()` APIs in `client/ui/material/key_value_list.rs` and `client/ui/material/toleration_list.rs`, registered in `client/ui/material/mod.rs`.
- [ ] T086 [US5] [A19] [A20] [U107] [U108] Node selector, tolerations, volume size, storage class, "applies on next start" and Logs (pod log tail via `Cluster::logs`) in `client/features/daemons.rs`, `client/shell/daemons.rs` and `client/ui/settings/daemons.rs`; geometry gate.
- [ ] T082 [US5] [A23] [U105] [U106] [U143] Wire Stop, Remove and Forget locally for Kubernetes daemons (`core/kube/lifecycle.rs` over `LiveCluster`, key files deleted through `client/kube/keys.rs`) in `client/daemon.rs`, `client/shell/daemons.rs` and `client/features/daemons.rs`; confirmation texts in `client/ui/confirm_daemon_removal.rs`; worktrees follow 491 FR-014.
- [ ] T133 [US5] [U133] `Unavailable` start of a Kubernetes daemon in `client/daemon.rs` (the `Dial` match), until M5 replaces it.
- [ ] T129 [US5] [A19] [A20] [A23] Acceptance (M4b): `features_daemons` green for A19, A20, A23.

---

## Phase 9: Connect over SSH — blocked on #806 (US1, US4; M5)

### Tests (write first, must fail)

- [ ] T134 [P] [US1] [U138] [U140] **(#806)** `Copy` and `Connect` phase tests in `core/kube/start.rs` over `FakeCluster`, `FakeKeyGen` and #806's provisioning fake: `Copy` passes the SSH target `{ 127.0.0.1, relay port, user, key file, known_hosts file, HostKeyAlias micold-kube-<key> }` and `daemon_env(&allowed)` (shared-ssh-image §3); a host-key mismatch → `HostKeyMismatch { pod }`; other provisioning failures → `Provision(..)`; `Connect` returns the bridge; bounds 120 s each.
- [ ] T022 [P] [US1] [A4] [A5] [U111] [U112] [U113] **(#806)** Client tests over `FakeCluster` in `client-tests/kube_daemon_start.rs`: Start → `Starting` → `connected` through the relay and 489's bridge, row endpoint `<context> / <namespace> / <pod>`; a second Start with the pod present adopts it and copies nothing (US1.4); two actors starting the same daemon on one `FakeCluster` → exactly one pod create succeeds and both connect, both clients' keys in the access secret (FR-023); a second, non-Kubernetes daemon's state is unchanged throughout (FR-019); a version mismatch → `VersionMismatch`, no session started (FR-017); no `Service` or `Ingress` (FR-006); app launch with the cluster unreachable does not delay the other daemons or the window (FR-019).
- [ ] T074 [P] [US4] [A14] [A15] [A16] [A17] [A18] [U116] **(#806)** Client tests over `FakeCluster` in `client-tests/kube_daemon_failures.rs`: each failure → `Unreachable` with its reason text and no retry; the reason and output reach the daemon's diagnostics; only a pre-existing PVC remains; another daemon's state is unchanged (US4.6, FR-019).

### Implementation

- [ ] T135 [US1] [U138] [U140] **(#806)** `Copy` and `Connect` phases in `core/kube/start.rs` calling #806's provisioning; `KubeFailure::Provision`; the live `KeyGen` (#806's key generation) chosen in `client/shell/capabilities.rs`.
- [ ] T030 [US1] [A4] [A5] [U111] [U112] [U113] **(#806)** `Dial::Kube` in `client/daemon.rs` replacing T133's `Unavailable`: run the start sequence inside the daemon's actor (no cluster I/O on the UI thread, FR-019) with `LiveCluster`, the real credential-file reader and `client/kube/keys.rs`; open the relay; 489's bridge and the 491 handshake over it; a failed start → `Unreachable` with the reason and no further attempts until Start or Reconnect (489 R7).
- [ ] T077 [US4] [A18] [U116] **(#806)** Report failures and phase output through `DaemonDiagnostics::push_log` in `client/daemon.rs`.
- [ ] T136 [US1] [A4] [A5] [A14] [A15] [A16] [A17] [A18] Acceptance (M5): `kube_daemon_start` green for A4, A5 and `kube_daemon_failures` for A14–A18.

---

## Phase 10: AI CLIs, drift prompt and the Add entry — blocked on #806 (US1, US4, US5; M6) 🎯 MVP

### Tests (write first, must fail)

- [ ] T138 [P] [US1] [U139] **(#806)** `Install` phase tests in `core/kube/start.rs` over #806's install fake: it passes `providers` and the volume's install root; output lines reach the activity log; Cancel → `Cancelled` and rollback; a paused clock at 15 min → `PhaseTimeout { phase: Install }`; a failed install → `Provision(..)` naming the CLI (US4.5); a CLI already installed is skipped by #806 and not re-run here.
- [ ] T139 [P] [US5] [A1] [A28] [U134] [U141] **(#806)** Tests: in `client-tests/features_daemons.rs`, the Add form lists Kubernetes once the mechanism is wired; a `Drift { fields }` start shows "The running pod differs from the settings (<fields>). Recreate it? Its <n> running session(s) will end; the volume is kept." with Recreate pod / Keep running pod, and each choice continues the start (contract §3); in `client-tests/kube_daemon_start.rs`, an added daemon with the default image starts, its selected CLIs are installed and it reaches `connected` (A1); an install failure or an image without the SSH contract shows its reason (A28).

### Implementation

- [ ] T141 [US1] [U139] **(#806)** `Install` phase in `core/kube/start.rs` with live output and Cancel through the actor; activity "Installing <cli>".
- [ ] T142 [US5] [A1] [A28] [U134] [U141] **(#806)** Show Kubernetes in the Add form's runtime choices; drift prompt in `client/features/daemons.rs` and `client/ui/settings/daemons.rs`; the default image = #806's pinned reference.
- [ ] T033 [US1] User guide: new `docs/user-guide/kubernetes-daemons.md` (adding, the default image or your own and its contract, AI CLIs installed on start, starting, what is created in the namespace, no Service, allowed credentials and where they are mounted, the pod's security settings, every form field, edits apply on next start and the drift prompt, Stop, Remove, Forget locally, Logs, a troubleshooting table of the contract §6 texts), linked from `docs/SUMMARY.md`, `docs/user-guide/settings.md` and `docs/user-guide/remote-daemons.md`. (Replaces T046, T071, T078, T087.)
- [ ] T140 [US1] [A1] [A28] Acceptance (M6): `kube_daemon_start` and `features_daemons` green for A1, A28; quickstart §B form steps pass at the visual pass.

---

## Phase 11: Clone and worktrees in the pod — blocked on #806 (US1; M7)

### Tests (write first, must fail)

- [ ] T047 [P] [US1] [U79] [U80] [U81] Clone rule tests in `core/kube/clone.rs`: `remote_for` picks `origin`, else the only remote; none → `NoRemote`; several without `origin` → `Ambiguous(names)`; `CloneTarget::for_url` = `/data/repos/<slug(repo)>-<hash8(url)>`, deterministic; `CloneFailure` from git stderr → `AuthRequired { remote }`, `DiskFull`, `Unreachable { remote }`, `Other`; texts as contract §6 (clone).
- [ ] T048 [P] [US1] [U82] Protocol tests in `core/protocol/messages.rs` and `core/protocol/version.rs`: `ClientMsg::RepoClone { url, dest }`, `DaemonMsg::RepoCloned { dest }`, `DaemonMsg::RepoCloneFailed { dest, failure }` round-trip; `PROTOCOL_VERSION` is the next number after `main`'s (49 unless #806 bumped first; re-pin `schema_hash.rs`).
- [ ] T049 [P] [US1] [U83] [U84] [U85] Daemon tests in `daemon/repo_clone.rs` against a local bare repository: clone into `.partial-<dest>-<rand>` and rename on success; an existing `dest` is fetched; two requests for one `dest` are serialised (the second fetches); an auth failure (with `GIT_TERMINAL_PROMPT=0`) leaves no directory; a `dest` outside `MICOLD_REPOS_ROOT` is refused.
- [ ] T050 [P] [US1] [A2] [U100] [U101] [U102] Reducer tests in `client-tests/features_daemons.rs` over a fake daemon: binding a worktree of a project with no remote to a Kubernetes daemon is refused with the reason and sends nothing; a project with a remote sends `RepoClone` then `WorktreeCreate`, in that order, showing activity "Cloning <repo>"; the bound worktree is a 489 remote project with identity `micold-remote://<daemon-id>/data/repos/…` (`ssh/remote_path.rs`), its row showing the daemon name and in-pod path.

### Implementation

- [ ] T051 [P] [US1] [U79] [U80] [U81] `remote_for`, `CloneTarget`, `CloneFailure` in `core/kube/clone.rs`.
- [ ] T052 [US1] [U82] `RepoClone` / `RepoCloned` / `RepoCloneFailed` (`CloneFailureWire`) in `core/protocol/messages.rs`; the `PROTOCOL_VERSION` bump in `core/protocol/version.rs`.
- [ ] T053 [US1] [U83] [U84] [U85] `daemon/repo_clone.rs` (new) dispatched from `daemon/ops.rs`, `MICOLD_REPOS_ROOT` from the environment, one lock per `dest`.
- [ ] T054 [US1] [A2] [U100] [U101] [U102] Create-worktree clone step for Kubernetes daemons in `client/features/` and `client/shell/workspace.rs`.
- [ ] T055 [US1] Docs: `docs/user-guide/worktrees-and-sessions.md` (worktrees on a Kubernetes daemon come from the project's remote; unpushed commits stay local; a remote needing a credential needs it allowed for the daemon), `docs/user-guide/kubernetes-daemons.md`, and `docs/daemon.md` (`RepoClone`, `MICOLD_REPOS_ROOT`).
- [ ] T100 [US1] [A2] Acceptance (M7): A2 green in `features_daemons` over the fake daemon.

---

## Phase 12: Sessions outlive the client — blocked on #806 (US2, US5; M8)

### Tests (write first, must fail)

- [ ] T057 [P] [US2] [U17] [U103] State tests in `core/daemons.rs`: `Reconnecting` or `Unreachable` + `Gone(reason)` → `Stopped` with `stopped_reason`; `Start` from `Stopped` → `Starting` and clears it; plus a reducer test in `client-tests/features_daemons.rs` that the row shows `stopped` with the reason.
- [ ] T058 [P] [US2] [A6] [A7] [A8] [A9] [A22] [U114] [U115] Actor tests over `FakeCluster` in `client-tests/kube_daemon_reconnect.rs`: tunnel dropped with the pod running → relay re-opened, re-dial, sessions shown with output since the drop, no pod create; Reconnect on an `unreachable` daemon whose pod runs does the same (US5.4); pod deleted, evicted or node lost → `Stopped` with the reason, sessions `lost to its daemon`, no create call, actor ended; a 5xx or timeout on lookup → stays `Reconnecting` with 489's back-off; Start after `Gone` → a new pod on the same PVC; after another client's Remove → `Gone(Deleted)`, and Start creates a new, empty PVC and pod (FR-023); another daemon untouched.

### Implementation

- [ ] T060 [US2] [U17] `DaemonEvent::Gone`, `DaemonView.stopped_reason` and the transitions in `core/daemons.rs`.
- [ ] T061 [US2] [A6] [A7] [A8] [A9] [A22] [U103] [U114] [U115] Pod lookup (`PodGone`) before each re-dial and the Reconnect action in `client/daemon.rs`; sessions marked `lost to its daemon` through the 489 FR-019 path in `client/links.rs`; the row shows the reason in `client/ui/settings/daemons.rs`.
- [ ] T062 [US2] User guide `docs/user-guide/kubernetes-daemons.md`: sessions survive the client, Reconnect, and what "stopped: the pod was …" means.
- [ ] T101 [US2] [A6] [A7] [A8] [A9] [A22] Acceptance (M8): `kube_daemon_reconnect` green for A6–A9, A22.

---

## Phase 13: Proven end to end in CI — blocked on #806 (US6; M9)

- [ ] T088 [US6] Harness `client-tests/kube_real_support/mod.rs` behind feature `kube-real-runtime`: skips with a printed reason when docker, `kind` or `kubectl` is absent; creates kind cluster `micold-kube-test` and deletes it unless `MICOLD_KUBE_TEST_KEEP=1`; makes the SSH image available (pulled from its reference, or the stand-in test image via `kind load docker-image`, K12); RBAC fixtures: kubeconfig contexts for ServiceAccounts whose Role lacks `create pods`, and one lacking `pods/portforward` (`kubectl create token`); a package source for the stand-in AI CLI reachable from the pod (#806's install contract §4); a git remote reachable from the pod for the clone.
- [ ] T089 [P] [US6] `packaging/kube/test.Containerfile`: #806's SSH image plus the stand-in AI CLI's package source, if #806's install contract needs it in the image (K12).
- [ ] T090 [P] [US6] [A3] [A25] `client-tests/kube_real_endtoend.rs`: pod, tunnel, copy, install of the stand-in CLI, clone and worktree, a session's input and output, 0 Services and 0 Ingresses (SC-006); the time from Start to typed input, minus the `Pull` and `Install` phases, is asserted under 5 minutes (SC-001).
- [ ] T091 [P] [US6] [A25] `client-tests/kube_real_reconnect.rs`: drop the tunnel, reconnect, quit the client for an away window (`MICOLD_KUBE_AWAY_SECS`, default 60 in CI, 600 for the SC-002 run) and reopen: every session still running with its output since the drop; `kubectl delete pod` → `stopped` "the pod was deleted", no new pod (SC-002).
- [ ] T092 [P] [US6] `client-tests/kube_real_security.rs`: the live pod's security context and mounted secrets equal the access secret plus the allowed set; withdrawal deletes the secret (SC-004); the SSH server accepts only the access secret's keys; a remote needing the allowed git credential clones.
- [ ] T093 [P] [US6] [A26] `client-tests/kube_real_failures.rs`: RBAC `create pods` and `pods/portforward` denial, an image reference that does not exist, a node selector no node has, an install from an unreachable source; each reason within its bound, nothing left (SC-003).
- [ ] T094 [P] [US6] [A25] `client-tests/kube_real_lifecycle.rs` and `client-tests/kube_real_concurrent.rs`: stop keeps the PVC and the worktree after restart; remove leaves 0 labelled objects (SC-005); two concurrent starts → one pod, one daemon build, one install per CLI, both keys authorized (FR-023).
- [ ] T095 [US6] [A27] `mise.toml` task `test-kube` (build-locked, `cargo test -p micold-client --features kube-real-runtime kube_real_ --no-fail-fast -- --test-threads=1`).
- [ ] T096 [US6] [A27] CI job `kube-cluster` in `.github/workflows/ci.yml` beside `ssh-remote`, same docs-only skip, `kind` and `kubectl` at pinned versions with checksums, Linux only with the reason (K13) in a comment; `docs/development/ci-pipeline.md` describes it.
- [ ] T105 [US6] [A3] [A25] [A26] [A27] Acceptance (M9): `mise run test-kube` green for A3, A25, A26 on a kind cluster, and A27 (off by default, on in CI's `kube-cluster`).

---

## Phase 14: Polish

- [ ] T097 Run quickstart Part B at a display (the `visual-pass` skill) and save screenshots under `specs/686-kubernetes-daemon-runtime/visual-pass/`.
- [ ] T098 Re-run `speckit-converge` against the code and tick what is done.

---

## Dependencies & Execution Order

- Setup (done) → M1a → M1b → M1c → M2 → M3 → M4a → M4b, all without #806.
- M5 needs #806 merged and M1c, M2, M3, M4b. M6 needs M5. M7 and M8 need M6 (independent of each
  other). M9 needs M6–M8.
- Within a phase: tests → core types → core sequences → client adapters → client reducers → UI glue
  → docs.

## Parallel Examples

- M1a tests T106–T110, T143 and T020 together (seven files); then T111 → T112, with T113, T114, T115, T116, T026 beside.
- M1b: T019, T117, T118 together, then T021; T025, T119, T120 together.
- M1c: T122, T023, T123, T124 together; T126 beside T127.
- M9: T089–T094 together once T088 exists.

## Implementation Strategy

Everything that does not need #806 lands first, proven over `FakeCluster` and `FakeKeyGen`, with the
Add entry hidden (FR-024): core types and kubeconfig (M1a), the pod, volume and access secret (M1b),
the tunnel, relay, keys and live adapter (M1c), Stop/Remove and failure reasons (M2), allowed
credentials (M3), the form (M4a, M4b). After #806 merges: connect over SSH (M5), AI CLIs and the Add
entry, the MVP (M6), clone and worktrees (M7), reconnect (M8), the real-cluster CI proof (M9).
Polish (T097, T098) changes no code and is left to the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). Polish T097
(quickstart Part B visual pass) and T098 (`speckit-converge` re-run) are in no milestone: they
change no code, so the close unit does them in the close PR. M1a–M4b change no user-visible
behaviour (Kubernetes cannot be added before M6): `docs-not-needed`.

### M1a — Revision-2 core types and kubeconfig discovery

- **Tasks**: T106–T116, T143, T020, T026
- **Deliverable**: `micold-core` holds revision-2 `KubeSettings` (image, AI CLIs, shared `DaemonResources`) persisted as `DaemonRuntime::Kubernetes`, the access-secret name, the revision-2 failure texts, a `Cluster` port with `replace`, and kubeconfig discovery on Linux, macOS and Windows; revision 1's listener auth, `Transport::Tunnel` and `daemon_image` are gone.
- **Satisfies**: FR-002, FR-018, FR-020, FR-022 (discovery); FR-005 (resources type); FR-016 (names)
- **Verify**: `mise run test-core`; `git diff origin/main -- crates/micold-core/src/protocol/auth.rs crates/micold-daemon/src/server.rs crates/micold-core/src/connect.rs` prints nothing
- **Depends on**: —
- **Tier**: full

### M1b — Pod, volume and access secret over a fake cluster

- **Tasks**: T019, T117, T118, T021, T025, T119, T120, T027, T121
- **Deliverable**: the core start sequence creates or adopts the daemon's PVC, SSH access secret and pod (no Service, locked-down manifest, resources, pod stamp), adds a second client's key without losing another's, offers a drift choice, and rolls back what an attempt created, within per-phase bounds — proven over `FakeCluster`.
- **Satisfies**: US1 scenarios 4, 5 (core half); US5 scenario 2 (drift detection); FR-003, FR-005, FR-006 (manifest), FR-013, FR-016, FR-023 (adopt, key race)
- **Verify**: `mise run test-core` (`kube::manifest::`, `kube::access::`, `kube::start::`)
- **Depends on**: M1a
- **Tier**: full

### M1c — Tunnel to the pod's SSH server; relay, keys, live adapter

- **Tasks**: T122, T023, T123, T124, T125, T028, T029, T126, T127
- **Deliverable**: the start sequence's `Tunnel` phase reads the pod's SSH identification line through the API port-forward (else `ImageContract`); the client has `LiveCluster`, the loopback relay and owner-only key files, each unit-tested.
- **Satisfies**: FR-006 (port-forward only), FR-008 (key files), FR-002, FR-022 (exec plugins), FR-012 (`ImageContract`)
- **Verify**: `mise run test-core` (`kube::start::`); `cargo test -p micold-client --lib kube::`; `cargo test -p micold-client --test no_concrete_implementations`
- **Depends on**: M1b
- **Tier**: full

### M2 — Stop, Remove and failure reasons in core

- **Tasks**: T079, T072, T056, T073, T081, T075, T059, T076, T128
- **Deliverable**: core Stop, Remove and forget-locally sequences, and every start failure (RBAC before any create, config, namespace, pull, scheduling, volume) classified with its contract §6 reason on first sight, plus the pod-gone classifier — proven over `FakeCluster`.
- **Satisfies**: US4 scenarios 1–4 (core); US5 scenarios 3, 6 (core); FR-012, FR-014, FR-015; FR-011 (classifier)
- **Verify**: `mise run test-core` (`kube::lifecycle::`, `kube::classify::`, `kube::start::`)
- **Depends on**: M1c
- **Tier**: full

### M3 — Only allowed credentials reach the pod

- **Tasks**: T063, T064, T065, T067, T068, T069, T102
- **Deliverable**: the start sequence creates allowed credentials as secrets mounted read-only beside the access secret, deletes withdrawn ones, builds the daemon's environment for them, and the pod runs non-root with a read-only root filesystem — proven over `FakeCluster`.
- **Satisfies**: US3 scenarios 1–4 (core); FR-008, FR-009
- **Verify**: `mise run test-core` (`kube::credentials::`, `kube::manifest::`, `kube::start::`)
- **Depends on**: M1b
- **Tier**: full

### M4a — The Kubernetes form, Add entry hidden

- **Tasks**: T024, T130, T066, T031, T131, T070, T032
- **Deliverable**: a Kubernetes daemon in settings opens in Edit with context, namespace, image, AI CLIs, resources (fields shared with the container form) and allowed credentials; the Add form does not offer Kubernetes yet (FR-024).
- **Satisfies**: US5 scenario 1 (form, minus placement and volume); FR-001, FR-002, FR-004 (fields), FR-005 (resources), FR-018, FR-024
- **Verify**: `cargo test -p micold-client --test features_daemons`; `cargo test -p micold-client --test daemons_section_geometry`
- **Depends on**: M1c (context listing in `LiveCluster`, T023/T028)
- **Tier**: full

### M4b — Placement fields, Stop, Remove, Logs from the row

- **Tasks**: T083, T084, T080, T132, T085, T086, T082, T133, T129
- **Deliverable**: the form edits node selector, tolerations, volume size and storage class; edits show "applies on next start"; Stop, Remove (with confirmations and key-file deletion), Forget locally and Logs work from a Kubernetes row; Start of a Kubernetes daemon says the build lacks the shared SSH image (`Unavailable`).
- **Satisfies**: US5 scenarios 1, 2, 3, 5, 6; FR-005, FR-014, FR-015, FR-018, FR-024
- **Verify**: `cargo test -p micold-client --test features_daemons`; `cargo test -p micold-client --test daemons_section_geometry`; `cargo test -p micold-client --test kube_daemon_start`; `cargo test -p micold-client --lib ui::material::key_value_list`; `cargo test -p micold-client --lib ui::material::toleration_list`
- **Depends on**: M2, M4a
- **Tier**: full

### M5 — Connect over SSH (blocked on #806)

- **Tasks**: T134, T022, T074, T135, T030, T077, T136
- **Deliverable**: Start of a Kubernetes daemon copies the client's daemon in over SSH through the relay with the pinned host key and reaches `connected`; failures show their reason with no retry; two clients share one pod.
- **Satisfies**: US1 scenarios 4, 5; US4 scenarios 1–4, 6; FR-006, FR-010 (detached daemon), FR-017, FR-019, FR-023
- **Verify**: `mise run test-core` (`kube::start::`); `cargo test -p micold-client --test kube_daemon_start`; `cargo test -p micold-client --test kube_daemon_failures`
- **Depends on**: #806, M1c, M2, M3, M4b
- **Tier**: full

### M6 — AI CLIs on start and the Add entry 🎯 MVP (blocked on #806)

- **Tasks**: T138, T139, T141, T142, T033, T140
- **Deliverable**: Settings → Daemons → Add offers Kubernetes; Start installs the selected AI CLIs (live output, Cancel) and connects; a pod running with other settings offers Recreate or Keep; the user guide describes it.
- **Satisfies**: US1 scenarios 1, 4; US4 scenario 5; US5 scenarios 1, 2; FR-001, FR-004, FR-012, FR-024
- **Verify**: `mise run test-core` (`kube::start::`); `cargo test -p micold-client --test kube_daemon_start`; `cargo test -p micold-client --test features_daemons`
- **Depends on**: M5
- **Tier**: full

### M7 — Worktrees and sessions in the pod (blocked on #806)

- **Tasks**: T047–T055, T100
- **Deliverable**: creating a worktree bound to a Kubernetes daemon clones the project's remote into the daemon's volume and creates the worktree there, shown with the daemon's name and in-pod path; a project without a remote is refused with the reason.
- **Satisfies**: US1 scenarios 2, 3; FR-007
- **Verify**: `mise run test-core` (`kube::clone::`, `protocol::`); `cargo test -p micold-daemon repo_clone`; `cargo test -p micold-client --test features_daemons`
- **Depends on**: M6
- **Tier**: full

### M8 — Sessions outlive the client (blocked on #806)

- **Tasks**: T057, T058, T060, T061, T062, T101
- **Deliverable**: a dropped tunnel or the user's Reconnect re-dials the same pod with sessions intact; a deleted, evicted or node-lost pod shows `stopped` with its reason, sessions `lost to its daemon`, and is not recreated until Start.
- **Satisfies**: US2 scenarios 1–4; US5 scenario 4; FR-010, FR-011
- **Verify**: `cargo test -p micold-client --test kube_daemon_reconnect`; `mise run test-core` (`daemons::`)
- **Depends on**: M6
- **Tier**: full

### M9 — Real-cluster suite in CI (blocked on #806)

- **Tasks**: T088–T096, T105
- **Deliverable**: `mise run test-kube` runs the `kube_real_*` suite on a throwaway kind cluster, and CI's `kube-cluster` job runs it on every non-docs change.
- **Satisfies**: US6 scenarios 1–3; FR-021; SC-001–SC-006 evidence
- **Verify**: `mise run test-kube`; CI job `kube-cluster` green on the PR
- **Depends on**: M6, M7, M8
- **Tier**: full
