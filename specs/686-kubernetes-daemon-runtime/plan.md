# Implementation Plan: Kubernetes session daemon runtime

**Branch**: `claude/project-thread-xelisc` | **Date**: 2026-10-17 (revision 2) | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/686-kubernetes-daemon-runtime/spec.md`

## Summary

Add a fourth `DaemonRuntime` variant, **`Kubernetes(KubeSettings)`**, to spec 491's registry. The
client talks to the cluster with the `kube` crate (client crate only) behind a core port, creates or
adopts a bare, locked-down pod running a basic **SSH image**, plus a PVC, the daemon's SSH access
secret and only the allowed secrets, all named from (context, namespace, daemon name) and labelled as
ours. It reaches the pod's SSH server (pod loopback, port 2222) through the API's port-forward behind
a client loopback relay, and from there uses spec 489's own path: probe, copy the client's daemon
build in, start it, and bridge to it over the user's `ssh`, with a pinned host key. The AI CLIs the
user selected are installed on the volume on start. The SSH image, the copy-and-start and the CLI
install are **one mechanism shared with the local container runtime, built by #806** and consumed
here ([contracts/shared-ssh-image.md](./contracts/shared-ssh-image.md)). Start is a phase sequence
with per-phase bounds and classified failures, rolled back on failure. The 489 reconnect actor is
reused; a pod found gone stops the daemon with its reason. Worktrees live on the PVC: the daemon
clones the project's remote with a new `RepoClone` request, and the clone is a 489-style remote
project. CI proves it on a kind cluster.

**Revision 2** (owner decisions 8-11 in the ledger): no image build, push or registry; no in-pod
token-less listener; no `Transport::Tunnel`. **Order of work**: everything that does not need #806
first (settings, kubeconfig, manifests and the pod/PVC/secret lifecycle, the relay and `Tunnel`
phase, Stop/Remove, failure reasons, the Settings form with the Add entry hidden); then, **blocked
on #806**, the copy, install and connect phases, the Add entry, clone and worktrees, reconnect, and
the real-cluster suite.

Decisions and rejected alternatives: [research.md](./research.md). Types: [data-model.md](./data-model.md).
Interfaces: [contracts/](./contracts/). Proof: [quickstart.md](./quickstart.md).

## Technical Context

**Language/Version**: Rust stable (`rust-toolchain.toml`, workspace `rust-version` 1.97); iced 0.14
for the Settings form.

**Primary Dependencies**: new, `micold-client` only: `kube` 4.2 (`client`, `config`, `rustls-tls`,
`ring`, `ws`) and `k8s-openapi` 0.28 (`v1_32`), both Apache-2.0 (vetting in research K0). Existing:
`tokio`, `serde_json`, `protocol::hashing::sha256_hex` (daemon key K5, pod stamp; no new hash crate),
489's `ssh::{argv, provision, classify, reconnect::Backoff, remote_path}` and `ClientPlatform`,
`owner_only.rs` for the client key files. From #806 (blocked): the SSH image reference, key
generation, provisioning over SSH and the AI CLI install (contract shared-ssh-image).

**Storage**: local only. `settings.json` `daemons[].runtime = {"kind":"kubernetes",...}` (additive, no
`settings_version` bump). No cluster credential or kubeconfig content is stored (FR-002). Each
client's SSH key and pinned host key per daemon live in owner-only files in the app's data directory
(FR-008). The cluster holds the pod, PVC, access secret and allowed secrets.

**Testing**: `mise run test-core` (settings, names, ownership, access secret, pod stamp and drift,
manifests, phase plan and bounds, classifiers, kubeconfig candidates and exec search path per
platform, clone rules, state machine, the start/stop/remove sequences over `FakeCluster`);
`mise run test` (`LiveCluster` pure helpers, relay over an in-memory stream, actor over
`FakeCluster`, Settings reducers, daemon `RepoClone`); new `mise run test-kube` + `kube_real_*` on
kind (blocked on #806); geometry gates for the new form; quickstart §B visual pass.

**Target Platform**: Linux, macOS, Windows clients; Linux pods (amd64, arm64 nodes).

**Project Type**: desktop app (Rust workspace: `micold-core`, `micold-client`, `micold-daemon`).

**Performance Goals**: SC-001 under 5 minutes excluding image pull and AI CLI install; failure reasons
within the phase bound (FR-012), image pull failures on first sight.

**Constraints**: the window never blocks on the cluster (all I/O in the daemon's actor); a Kubernetes
daemon's failure touches no other daemon (FR-019); no concrete implementation named outside
`shell/` (`no_concrete_implementations.rs`): `LiveCluster` is chosen once in `shell/capabilities.rs`;
no second copy of #806's mechanism (FR-024).

**Scale/Scope**: handfuls of daemons; one pod, one PVC, ≤ 10 secrets, one relay per Kubernetes daemon.

## Constitution Check

| Principle | Result |
|---|---|
| I Test-First | PASS: every rule is a pure core function written test-first (manifests, names, access secret, pod stamp, classifiers, phase bounds, kubeconfig/exec path, clone rules, state machine); the start/stop/remove sequences run over `FakeCluster` and a fake `KeyGen`; `LiveCluster`'s decisions (`kube::Error` → `ApiFailure`, delete preconditions, exec-command rewrite) are pure helpers table-tested in `live.rs`; the relay is tested over an in-memory duplex stream; `kube_real_*` proves the adapter against a real cluster. Settings view glue keeps the documented glue exception; its decisions live in `features/daemons.rs`. |
| II Multi-Session | PASS: one actor, state, outbox and relay per daemon; resources named per daemon key; sessions addressable by (daemon, session id); the pod clone is a remote project namespaced by daemon id (489 R5). |
| III Worktree Integration | PASS: the app clones and creates worktrees through the daemon (`RepoClone`, `WorktreeCreate`), no manual git; worktrees stay bound to their one daemon (491). |
| IV Local-First (NON-NEGOTIABLE) | PASS: all client state local; other daemons and the app work offline. The cluster is one the user names (explicit opt-in, as 489's host); credentials leave the device only when allowed per daemon (FR-008); the client's private key never leaves it. |
| V Rust + iced | PASS: validated newtypes (`Namespace`, `KubeContextName`, `Quantity`, `Toleration`, `DaemonKey`), `KubeCredential` mapped 1:1 to secrets, `StartPhase` with its bound, `KubeFailure`/`PodGone` enums; iced only for UI. |
| VI Cross-Platform | PASS: one code path; platform differences (KUBECONFIG separator, home directory, login-shell PATH for exec plugins, the user's `ssh` per 489) are pure, platform-parameterised core rules tested on all three CI platforms; the real-cluster job is Linux-only with the reason in `ci.yml` (K13). |
| VII Documentation | PASS: `docs/user-guide/kubernetes-daemons.md` (new, with the milestone that adds the Add entry), `settings.md`, `worktrees-and-sessions.md`, `remote-daemons.md` cross-link, `docs/daemon.md`, `docs/development/ci-pipeline.md`, `docs/SUMMARY.md`. Milestones before the Add entry change no user-visible behaviour (`docs-not-needed`). |
| VIII UI Components | PASS: the form reuses 491's Daemons rows, text inputs and badges; the resources fields are extracted from `ui/settings/container_fields.rs` into functions both forms draw (#806 inherits them); key/value list and toleration editors are added to the shared library with builder `.into()` APIs if none exists. |

Re-checked after Phase 1: unchanged. Deviations: Complexity Tracking.

## Requirement → design map

| FR | Where |
|---|---|
| FR-001, FR-018, FR-020 | `daemons.rs` `DaemonRuntime::Kubernetes`, label, endpoint; `kube/settings.rs` validation (`InvalidField` naming the field); `features/daemons.rs` Draft/Msg (incl. AI CLI checkboxes, volume-size notice); `ui/settings/daemons.rs`; pod log tail via `Cluster::logs`; contract kube-runtime §7 |
| FR-002, FR-022 | `kube/kubeconfig.rs` (candidates, separators, home, exec search path, login-shell parse); `micold-client/src/kube/live.rs` merges files with `kube::config::Kubeconfig` and rewrites exec commands to absolute paths (K9) |
| FR-003, FR-016, FR-023 | `kube/names.rs` (`DaemonKey`, `ResourceNames`, labels); create/409/adopt in `kube/start.rs`; deletes by name + UID; access-secret `replace` with `resourceVersion` retry in `kube/access.rs`; copy/install concurrency is #806's (contract shared-ssh-image §3-§4); `RepoClone` serialised per `dest` in the daemon (K5) |
| FR-004 | `KubeSettings.{image, providers}`; default image = #806's pinned reference; `Copy` and `Install` phases call #806's mechanism (**blocked on #806**) |
| FR-005 | `KubeSettings.{resources, node_selector, tolerations, volume_size, storage_class}`; `resources` = shared `daemon_resources::DaemonResources` → `kube/manifest.rs` `resources.requests` / `.limits`; `PodStamp` annotation and `drift` → `StartOutcome::Drift` → the row's Recreate / Keep choice |
| FR-006 | `Cluster::port_forward` behind `micold-client/src/kube/relay.rs` (loopback listener, one stream per connection); manifest test: no Service/Ingress/hostPort/containerPort; `kube_real_endtoend` lists Services/Ingresses = 0 |
| FR-007 | `kube/clone.rs`; `ClientMsg::RepoClone` (protocol 49); `micold-daemon/src/repo_clone.rs`; remote-project identity via `ssh/remote_path.rs`; create-worktree flow in `features/` + `shell/workspace.rs` (K8; **blocked on #806**) |
| FR-008, FR-009 | `kube/credentials.rs` (`KubeCredential` → secret + mount, `daemon_env`); `kube/access.rs` (`AccessSecret`, `KeyGen` port); `micold-client/src/kube/keys.rs` (owner-only key files, known_hosts with `HostKeyAlias`); `kube/manifest.rs` pod security context; `Secrets` phase deletes withdrawn ones (K3, K7) |
| FR-010, FR-011 | 489 actor (`micold-client/src/daemon.rs`) gains `Dial::Kube` (relay, then 489's bridge); pod lookup before re-dial → `PodGone` → `DaemonEvent::Gone` → `Stopped` + `stopped_reason` (K10; **blocked on #806**) |
| FR-012, FR-013 | `kube/start.rs` (`StartPhase`, bounds, `Attempt` rollback incl. an added key); `kube/classify.rs` (`KubeFailure`); install output to `DaemonDiagnostics::push_log`, Cancel via the actor (K6) |
| FR-014, FR-015 | `kube/lifecycle.rs` stop/remove sequences; `client/kube/keys.rs` delete on Remove; confirmations in `features/daemons.rs` with 491 counts; unreachable → "Forget locally" naming the remaining key (K11, contract kube-runtime §5) |
| FR-017 | the client's own build copied in (#806 provisioning); existing handshake refusal → `VersionMismatch` (491 FR-008) |
| FR-019 | per-daemon actor (491); kube I/O only inside it; app launch never awaits a cluster |
| FR-021 | `crates/micold-client/tests/kube_real_{endtoend,reconnect,security,failures,lifecycle,concurrent}.rs` + `kube_real_support/` (kind, RBAC fixtures, stand-in CLI source); cargo feature `kube-real-runtime`; `mise run test-kube`; CI job `kube-cluster` (K12; **blocked on #806**) |
| FR-024 | no copy of #806's code in #686; the Add form's runtime list omits Kubernetes until the milestone that wires the mechanism (`features/daemons.rs` runtime choices, tested) |

## Project Structure

### Documentation (this feature)

```text
specs/686-kubernetes-daemon-runtime/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/
│   ├── kube-runtime.md       # cluster objects, ownership, start/reconnect/stop/remove, strings, form
│   └── shared-ssh-image.md   # what #686 needs from #806's shared mechanism
└── tasks.md
```

### Source Code

```text
crates/micold-core/src/
├── kube/                     # NEW, pure (no kube crate)
│   ├── mod.rs                # Cluster port (create/get/replace/delete/…/port_forward/logs), views, ApiFailure
│   ├── settings.rs           # KubeSettings + newtypes + validation, Quantity::{to_k8s, parse_k8s}
│   ├── names.rs              # DaemonKey, ResourceNames (+ access secret, all_secrets), labels
│   ├── credentials.rs        # KubeCredential -> secret, mount; daemon_env
│   ├── access.rs             # AccessSecret, KeyGen port
│   ├── manifest.rs           # pod / pvc / secret JSON, PodStamp, drift, access reviews
│   ├── start.rs              # StartPhase, bounds, StartPlan, Attempt, the sequence over Cluster
│   ├── lifecycle.rs          # stop, remove, forget-locally lists
│   ├── classify.rs           # KubeFailure, PodGone classifiers
│   ├── kubeconfig.rs         # candidates, exec search path, login-shell output parse
│   ├── clone.rs              # remote choice, CloneTarget, CloneFailure       (blocked on #806)
│   └── fake.rs               # FakeCluster, FakeKeyGen (cfg(test) + feature for client tests)
├── daemon_resources.rs       # NEW, shared with #806 (moved from revision 1's daemon_image/)
├── daemons.rs                # DaemonRuntime::Kubernetes, DaemonEvent::{Gone, Drift}, DaemonView.stopped_reason
└── protocol/{messages,version}.rs   # RepoClone; PROTOCOL_VERSION 49   (blocked on #806)
crates/micold-daemon/src/
├── repo_clone.rs             # NEW: atomic clone/fetch under MICOLD_REPOS_ROOT  (blocked on #806)
└── ops.rs                    # dispatch RepoClone
crates/micold-client/src/
├── kube/                     # NEW, render-free
│   ├── live.rs               # LiveCluster: kube client, kubeconfig merge, exec rewrite, port-forward, logs
│   ├── relay.rs              # loopback relay: one port-forward per accepted connection
│   └── keys.rs               # owner-only client key + known_hosts per daemon
├── daemon.rs                 # Dial::Kube, start sequence, drift prompt, pod-gone check before re-dial
├── features/daemons.rs       # Kubernetes Draft fields, confirmations, forget-locally, logs, runtime choices
├── shell/{capabilities,daemons,workspace}.rs   # choose LiveCluster; create-worktree clone step
└── ui/settings/{daemons,container_fields}.rs   # form; resources fields extracted for both forms
crates/micold-client/tests/kube_real_*.rs, kube_real_support/
.github/workflows/ci.yml (job kube-cluster), mise.toml (task test-kube)
docs/user-guide/kubernetes-daemons.md, settings.md, worktrees-and-sessions.md, remote-daemons.md;
docs/daemon.md; docs/development/ci-pipeline.md; docs/SUMMARY.md
```

**Reverted from the unmerged revision-1 M1a commits**: `crates/micold-core/src/daemon_image/`
(`DaemonResources` moves to `daemon_resources.rs`), `RegistryRepo`/`NodeArch` in `kube/settings.rs`,
`KubeFailure::{NoRuntime, Build, Push, VolumeTooSmall}`, `StartPhase::Image`, `Cluster::node_arch`,
`protocol/auth.rs` and its use in `micold-daemon/src/server.rs` (K4), `Transport::Tunnel` in
`connect.rs` (K2).

**Structure Decision**: all rules in `micold-core::kube` (pure; the I/O sequences over the injected
`Cluster` and `KeyGen` ports), the `kube` crate confined to `micold-client/src/kube/live.rs`,
render-free client reducers in `features/`, `ui/` as glue. The daemon crate gains only `RepoClone`.
#806's mechanism is called, never copied.

## Test strategy

| Requirement | Layer |
|---|---|
| FR-002, FR-022 (candidates, separators, home, exec search path) | core unit, platform-parameterised, all three CI platforms |
| FR-003, FR-016, FR-023 (names, ownership, adopt on 409, conflict, key race) | core unit (`names.rs`, `access.rs`: `with_client` idempotent, a `409` on `replace` re-reads and keeps the other client's key); start sequence over `FakeCluster` (two racing starts → one pod) |
| FR-004 | core unit: default image = the pinned reference, a given image used as is, `providers` passed to `Install` (blocked); `kube_real_endtoend` (blocked) |
| FR-005 (resources, drift) | core unit (`manifest.rs`: request alone, limit alone, both, neither → exactly those keys; request > limit refused naming the field; `drift` names the differing fields, empty when equal); start over `FakeCluster`: a differing adopted pod → `Drift`, Recreate deletes and recreates, Keep continues |
| FR-006, FR-008, FR-009 (manifests, access, keys) | core unit (`manifest.rs`, `credentials.rs`, `access.rs`); client unit (`keys.rs` files owner-only, known_hosts line with the alias; `relay.rs` copies both ways over a duplex stream, one stream per connection, closes on drop); `kube_real_security` (blocked; SC-004, SC-006) |
| FR-007 | core unit (`clone.rs`); daemon unit (`repo_clone.rs` against a local bare repo: atomic, fetch on existing, auth failure leaves nothing, two requests for one `dest` serialised); client reducer tests over a fake daemon (blocked) |
| FR-010, FR-011, FR-019 | core state-machine unit (`Gone`); client actor test over `FakeCluster` (pod deleted/evicted/node lost → `Stopped` + reason, no re-create; other daemon untouched); `kube_real_reconnect` (blocked) |
| FR-012, FR-013 | core unit (`classify.rs` tables, phase bounds, `Tunnel` reads the SSH identification line or fails `ImageContract`); start over `FakeCluster` (each failure → reason, rollback leaves only the pre-existing PVC and removes an added key); `kube_real_failures` (blocked) |
| FR-014, FR-015 | core sequence tests over `FakeCluster` (`StopTimeout`, Remove deletes all ours and only ours, unreachable lists the access secret); client test: Remove deletes the key files; `kube_real_lifecycle` (blocked; SC-005) |
| FR-017 | existing 491 mismatch tests; provisioning copies the client build (#806's tests) |
| FR-001, FR-018, FR-020, FR-024 | core unit (validation, settings round-trip incl. `Unsupported` on an old build); client reducers (form fields, CLI checkboxes default to enabled providers, volume-size notice, runtime choices omit Kubernetes until the wiring milestone); geometry gate `daemons_section_geometry.rs` extended; quickstart §B visual pass |
| FR-021 | `kube_real_*` in CI `kube-cluster` (blocked) |

## Complexity Tracking

| Deviation | Why | Alternative rejected because |
|---|---|---|
| Two new crates (`kube`, `k8s-openapi`) and their transitive hyper/tower/tungstenite stack in the client | exec plugins, kubeconfig merging and an in-process port-forward stream (FR-002, FR-006, FR-022) | `kubectl` is not a stated requirement and opens a loopback listener; a hand-written client re-implements auth plugins and the port-forward protocol (K0) |
| A client loopback relay in front of the port-forward | 489's provisioning drives the user's `ssh`, which needs a TCP endpoint (K2) | `ProxyCommand` re-running the app re-reads kubeconfig and exec plugins; a second, in-process SSH stack |
| `KeyGen` port with only a fake until #806 | builds and tests the access secret before #806 lands (K1, K3) | waiting for #806 for all secret logic, or a second copy of key generation (FR-024) |
| `PROTOCOL_VERSION` 49 (`RepoClone`) | the pod must clone; no existing op does (K8) | a git bundle through the tunnel carries unpushed commits the spec drops and needs a bulk-transfer path |
| Real-cluster suite Linux-only | kind needs Linux containers; macOS/Windows runners have none (K13) | parity is proven by platform-parameterised core tests on all three |
