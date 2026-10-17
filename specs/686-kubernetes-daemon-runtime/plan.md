# Implementation Plan: Kubernetes session daemon runtime

**Branch**: `claude/project-thread-xelisc` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/686-kubernetes-daemon-runtime/spec.md`

## Summary

Add a fourth `DaemonRuntime` variant, **`Kubernetes(KubeSettings)`**, to spec 491's registry. The
client talks to the cluster with the `kube` crate (client crate only) behind a core port, creates or
adopts a bare, locked-down pod plus a PVC and only the allowed secrets, all named from
(context, namespace, daemon name) and labelled as ours, and reaches the daemon through the API's
port-forward as an in-process stream — no Service, no listener on either side. The daemon in the pod
listens on the pod's loopback without a token (only `pods/portforward` holders can reach it). Start
is a phase sequence with per-phase bounds and classified failures, rolled back on failure. The 489
reconnect actor is reused; a pod found gone stops the daemon with its reason. The default image is
built locally from the client's own Linux daemon plus the selected CLIs, tagged by version and an
inputs hash, pushed to the user's registry and run by digest (shared `DaemonImageSource` model,
introduced here for #806). Worktrees live on the PVC: the daemon clones the project's remote with a
new `RepoClone` request, and the clone is a 489-style remote project. CI proves it on a kind cluster
with a registry in the job.

Decisions and rejected alternatives: [research.md](./research.md). Types: [data-model.md](./data-model.md).
Interfaces: [contracts/](./contracts/). Proof: [quickstart.md](./quickstart.md).

**Separable parts** (FR-004/FR-007 run on orchestrator defaults pending the owner's decision cards):
the image source is `micold-core/src/daemon_image/` + `micold-client/src/kube/image.rs` +
[contracts/daemon-image.md](./contracts/daemon-image.md) (research K1-K3); the clone source is
`micold-core/src/kube/clone.rs` + `micold-daemon/src/repo_clone.rs` + the `RepoClone` op (K8). A
different answer replaces those and leaves the pod, tunnel, lifecycle and CI design unchanged.

## Technical Context

**Language/Version**: Rust stable (`rust-toolchain.toml`, workspace `rust-version` 1.97); iced 0.14
for the Settings form.

**Primary Dependencies**: new, `micold-client` only: `kube` 4.2 (`client`, `config`, `rustls-tls`,
`ring`, `ws`) and `k8s-openapi` 0.28 (`v1_32`), both Apache-2.0 (vetting in research K0). `sha2` 0.10
(RustCrypto, MIT/Apache-2.0, not yet in `Cargo.lock`) in `micold-core` for the image-inputs hash and
daemon key. Existing:
`tokio`, `serde_json`, `sandbox::cli::ContainerCli` (docker/podman), 489's `ssh::provision`
artefact rule, `ssh::remote_path`, `ssh::reconnect::Backoff`.

**Storage**: local only. `settings.json` `daemons[].runtime = {"kind":"kubernetes",...}` (additive, no
`settings_version` bump). No cluster credential, registry password or kubeconfig content is stored
(FR-002). The cluster holds the pod, PVC and allowed secrets; nothing else leaves the device.

**Testing**: `mise run test-core` (manifests, names, ownership, phase plan and bounds, classifiers,
kubeconfig candidates and exec search path per platform, image tag and Containerfile render, clone
rules, listener auth, state machine); `mise run test` (client start/reconnect/stop/remove sequences
over `FakeCluster`, Settings reducers, daemon `RepoClone`); new `mise run test-kube` +
`kube_real_*` on kind; geometry gates for the new form; quickstart §B visual pass.

**Target Platform**: Linux, macOS, Windows clients; Linux pods (amd64, arm64 nodes).

**Project Type**: desktop app (Rust workspace: `micold-core`, `micold-client`, `micold-daemon`).

**Performance Goals**: SC-001 under 5 minutes excluding image build/push/pull; failure reasons
within the phase bound (FR-012), image pull failures on first sight.

**Constraints**: the window never blocks on the cluster (all I/O in the daemon's actor); a Kubernetes
daemon's failure touches no other daemon (FR-019); no concrete implementation named outside
`shell/` (`no_concrete_implementations.rs`): `LiveCluster` is chosen once in `shell/capabilities.rs`.

**Scale/Scope**: handfuls of daemons; one pod, one PVC, ≤ 9 secrets and one tunnel per Kubernetes
daemon.

## Constitution Check

| Principle | Result |
|---|---|
| I Test-First | PASS: every rule is a pure core function written test-first (manifests, names, classifiers, phase bounds, tag, Containerfile, kubeconfig/exec path, clone rules, listener auth, state machine); the I/O sequences run over `FakeCluster` in client tests before `LiveCluster` exists; `kube_real_*` is written before the adapter it drives. Settings view glue keeps the documented glue exception; its decisions live in `features/daemons.rs`. |
| II Multi-Session | PASS: one actor, state, outbox and tunnel per daemon; resources named per daemon key so two daemons never share pod, PVC or secrets; sessions addressable by (daemon, session id); the pod clone is a remote project namespaced by daemon id (489 R5). |
| III Worktree Integration | PASS: the app clones and creates worktrees through the daemon (`RepoClone`, `WorktreeCreate`), no manual git; the pod clone's root is the project root ("Default") of that remote project; worktrees stay bound to their one daemon (491). |
| IV Local-First (NON-NEGOTIABLE) | PASS: all client state local; other daemons and the app work offline. The cluster and registry are ones the user names (explicit opt-in, as 489's host); credentials leave the device only when allowed per daemon (FR-008); nothing stored about the cluster beyond context and namespace names. |
| V Rust + iced | PASS: validated newtypes (`Namespace`, `KubeContextName`, `RegistryRepo`, `Quantity`, `Toleration`, `DaemonKey`), `KubeCredential` as an enum mapped 1:1 to secrets, `StartPhase` with its bound, `KubeFailure`/`PodGone` enums; iced only for UI. |
| VI Cross-Platform | PASS: one code path; platform differences (KUBECONFIG separator, home directory, login-shell PATH for exec plugins, Linux daemon artefact for macOS/Windows clients) are pure, platform-parameterised core rules tested on all three CI platforms; the real-cluster job is Linux-only with the reason in `ci.yml` (K13). |
| VII Documentation | PASS: `docs/user-guide/kubernetes-daemons.md` (new), `settings.md`, `worktrees-and-sessions.md`, `remote-daemons.md` cross-link, `docs/daemon.md`, `docs/development/ci-pipeline.md`, `docs/SUMMARY.md`, same change. |
| VIII UI Components | PASS: the form reuses 491's Daemons rows, text inputs and badges; the image-source + resources section is a shared builder component (`ui/settings/daemon_image.rs` → shared library) for #806 to reuse; key/value list and toleration editors are added to the shared library with builder `.into()` APIs if none exists. |

Re-checked after Phase 1: unchanged. Deviations: Complexity Tracking.

## Requirement → design map

| FR | Where |
|---|---|
| FR-001, FR-018, FR-020 | `daemons.rs` `DaemonRuntime::Kubernetes`, label, endpoint; `kube/settings.rs` validation (`InvalidField` naming the field); `features/daemons.rs` Draft/Msg; `ui/settings/daemons.rs`; pod log tail via `Cluster::logs`; contract §7 |
| FR-002, FR-022 | `kube/kubeconfig.rs` (candidates, separators, home, exec search path, login-shell parse); `micold-client/src/kube/live.rs` merges files with `kube::config::Kubeconfig` and rewrites exec commands to absolute paths (K9) |
| FR-003, FR-016, FR-023 | `kube/names.rs` (`DaemonKey`, `ResourceNames`, ownership labels); create/409/adopt rule in `kube/start.rs`; deletes by name + UID precondition (K5, contract §2) |
| FR-004 | `daemon_image/` (`DaemonImageSource`, `containerfile.rs`, `tag.rs`); `sandbox/cli.rs` `ContainerCli::{build, push, repo_digest}` (new methods); `micold-client/src/kube/image.rs` (K1-K3, contract daemon-image) |
| FR-005 | `KubeSettings.{budget, node_selector, tolerations, volume_size, storage_class}` → `kube/manifest.rs` |
| FR-006 | `Cluster::port_forward` → `connect::Transport::Tunnel(Box<dyn AsyncReadWrite>)` (new, generic, no kube type in core); manifest test: no Service/Ingress/hostPort; `kube_real_endtoend` lists Services/Ingresses = 0 |
| FR-007 | `kube/clone.rs` (remote choice, `CloneTarget`, `CloneFailure`); `ClientMsg::RepoClone` (protocol 49); `micold-daemon/src/repo_clone.rs`; remote-project identity via `ssh/remote_path.rs`; create-worktree flow in `features/` + `shell/workspace.rs` (K8) |
| FR-008, FR-009 | `kube/credentials.rs` (`KubeCredential` → secret + mount); `kube/manifest.rs` pod security context; secrets reconcile phase deletes withdrawn ones (K7) |
| FR-010, FR-011 | 489 actor (`micold-client/src/daemon.rs`) gains `Dial::Kube`; pod lookup before re-dial → `PodGone` → `DaemonEvent::Gone` (new) → `Stopped` + `stopped_reason`; sessions via 489 FR-019 path in `links.rs` (K10) |
| FR-012, FR-013 | `kube/start.rs` (`StartPhase`, bounds, `Attempt` rollback); `kube/classify.rs` (`KubeFailure`); build output to `DaemonDiagnostics::push_log`, Cancel via the actor (K6) |
| FR-014, FR-015 | `kube/lifecycle.rs` stop/remove sequences; confirmations in `features/daemons.rs` with 491 counts; unreachable → "Forget locally" (K11, contract §5) |
| FR-017 | existing handshake refusal → `VersionMismatch` (491 FR-008); tag carries the version, pod runs by digest |
| FR-019 | per-daemon actor (491); kube I/O only inside it; app launch never awaits a cluster |
| FR-021 | `crates/micold-client/tests/kube_real_{endtoend,reconnect,security,failures,lifecycle,concurrent}.rs` + `kube_real_support/` (kind, registry, RBAC fixtures, stand-in CLI image); cargo feature `kube-real-runtime` (new `[features]` in `micold-client/Cargo.toml`); `mise run test-kube`; CI job `kube-cluster` (K12) |
| K4 (enables FR-006/FR-009) | `protocol/auth.rs` `listener_auth`; `micold-daemon/src/server.rs` `adopt_auth_token` uses it |

## Project Structure

### Documentation (this feature)

```text
specs/686-kubernetes-daemon-runtime/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/
│   ├── kube-runtime.md     # cluster objects, ownership, start/reconnect/stop/remove, strings, form
│   └── daemon-image.md     # shared image source (#806): build, tag, push, template, rebuild rule
└── tasks.md                # /speckit-tasks (not created here)
```

### Source Code

```text
crates/micold-core/src/
├── kube/                     # NEW, pure (no kube crate)
│   ├── mod.rs                # Cluster port (boxed-future trait), PodStatusView, EventView, ApiFailure
│   ├── settings.rs           # KubeSettings + newtypes + validation
│   ├── names.rs              # DaemonKey, ResourceNames, ownership labels
│   ├── credentials.rs        # KubeCredential -> secret, mount, env
│   ├── manifest.rs           # pod / pvc / secret / access-review JSON
│   ├── start.rs              # StartPhase, bounds, StartPlan, Attempt, the sequence over Cluster
│   ├── lifecycle.rs          # stop, remove, forget-locally lists
│   ├── classify.rs           # KubeFailure, PodGone classifiers
│   ├── kubeconfig.rs         # candidates, exec search path, login-shell output parse
│   ├── clone.rs              # remote choice, CloneTarget, CloneFailure   (clone-source part)
│   └── fake.rs               # FakeCluster (cfg(test) + feature for client tests)
├── daemon_image/             # NEW, shared with #806                     (image-source part)
│   ├── mod.rs  containerfile.rs  tag.rs
├── daemons.rs                # DaemonRuntime::Kubernetes, DaemonEvent::Gone, DaemonView.stopped_reason
├── connect.rs                # Transport::Tunnel
├── protocol/{auth,messages,version}.rs   # listener_auth; RepoClone; PROTOCOL_VERSION 49
└── sandbox/cli.rs            # ContainerCli build / push / repo_digest
crates/micold-daemon/src/
├── repo_clone.rs             # NEW: atomic clone/fetch under MICOLD_REPOS_ROOT
├── ops.rs                    # dispatch RepoClone
└── server.rs                 # listener_auth
crates/micold-client/src/
├── kube/                     # NEW, render-free
│   ├── live.rs               # LiveCluster: kube client, kubeconfig merge, exec rewrite, port-forward, logs
│   └── image.rs              # build/push step over ContainerCli, live output, cancel
├── daemon.rs                 # Dial::Kube, start sequence, pod-gone check before re-dial
├── features/daemons.rs       # Kubernetes Draft fields, confirmations, forget-locally, logs
├── shell/{capabilities,daemons,workspace}.rs   # choose LiveCluster; create-worktree clone step
└── ui/settings/{daemons,daemon_image}.rs       # form; shared image + resources section
crates/micold-client/tests/kube_real_*.rs, kube_real_support/
packaging/kube/test.Containerfile               # CI stand-in CLI image
.github/workflows/ci.yml (job kube-cluster), mise.toml (task test-kube)
docs/user-guide/kubernetes-daemons.md, settings.md, worktrees-and-sessions.md, remote-daemons.md;
docs/daemon.md; docs/development/ci-pipeline.md; docs/SUMMARY.md
```

**Structure Decision**: all rules in `micold-core::kube` and `micold-core::daemon_image` (pure; the
I/O sequences over the injected `Cluster` port and `ContainerCli` runner), the `kube` crate confined
to `micold-client/src/kube/live.rs`, render-free client reducers in `features/`, `ui/` as glue. The
daemon crate gains only `RepoClone` and the listener-auth rule.

## Test strategy

| Requirement | Layer |
|---|---|
| FR-002, FR-022 (candidates, separators, home, exec search path) | core unit, platform-parameterised, all three CI platforms |
| FR-003, FR-016, FR-023 (names, ownership, adopt on 409, conflict) | core unit (`names.rs`); client sequence test over `FakeCluster` (two racing starts → one pod) |
| FR-004 (tag, template, rebuild rule) | core unit (`daemon_image`); client test of the build/push step over a fake `ContainerCli` runner (skip build when tag present, push always, digest used, cancel, 30-min bound with a paused clock) |
| FR-005, FR-006, FR-008, FR-009 (manifests) | core unit (`manifest.rs`, `credentials.rs`); `kube_real_security` inspects the live pod (SC-004, SC-006) |
| FR-007 (remote choice, clone target, failures) | core unit (`clone.rs`); daemon unit (`repo_clone.rs` against a local bare repo: atomic, fetch on existing, auth failure leaves nothing) |
| FR-010, FR-011, FR-019 | core state-machine unit (`Gone`); client actor test over `FakeCluster` (pod deleted/evicted/node lost → `Stopped` + reason, no re-create; other daemon untouched); `kube_real_reconnect` |
| FR-012, FR-013 | core unit (`classify.rs` tables, phase bounds); client sequence tests (each failure → reason, rollback leaves only pre-existing PVC); `kube_real_failures` (RBAC, pull, unschedulable; SC-003) |
| FR-014, FR-015 | client sequence tests; `kube_real_lifecycle` (stop keeps PVC and worktrees, remove leaves 0 objects; SC-005) |
| FR-017 | existing 491 mismatch tests + a client test with a pulled image reporting another version |
| FR-001, FR-018, FR-020 | core unit (validation, settings round-trip incl. `Unsupported` on an old build); client feature reducers; geometry gate `daemons_section_geometry.rs` extended for the Kubernetes form; quickstart §B visual pass |
| K4 listener auth | core unit (`listener_auth` table); daemon test: empty token path + non-loopback stays fatal |
| FR-021 | `kube_real_*` in CI `kube-cluster` |

## Complexity Tracking

| Deviation | Why | Alternative rejected because |
|---|---|---|
| Two new crates (`kube`, `k8s-openapi`) and their transitive hyper/tower/tungstenite stack in the client | exec plugins, kubeconfig merging and an in-process port-forward stream (FR-002, FR-006, FR-022) | `kubectl` is not a stated requirement and opens a loopback listener; a hand-written client re-implements auth plugins and the port-forward protocol (K0) |
| Daemon runs without a token when it listens on loopback inside the pod | a token would need a secret FR-009/SC-004 forbid; port-forward RBAC is the access control (K4) | a token secret or env var (forbidden or readable by `get pods`) |
| `DaemonImageSource` introduced ahead of #806 | the owner asked for one shared image-source form; #806 is unimplemented (K1) | reusing `sandbox::ImageSource` defaults to the release image FR-004 forbids |
| `PROTOCOL_VERSION` 49 (`RepoClone`) | the pod must clone; no existing op does (K8) | a git bundle through the tunnel carries unpushed commits the spec drops and needs a bulk-transfer path |
| Real-cluster suite Linux-only | kind needs Linux containers; macOS/Windows runners have none (K13) | parity is proven by platform-parameterised core tests on all three |
