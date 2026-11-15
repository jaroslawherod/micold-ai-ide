# Implementation Plan: End-to-end tests on a kind cluster, with a Helm chart for the orchestrator and web UI

**Branch**: `1981-kube-e2e` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: `specs/1981-kube-e2e/spec.md` (review round 2 CLEAN; D1 and D2 settled in the ledger). The clarify unit was skipped: the spec had no markers left.

## Summary

Build the first slice of the Kubernetes deployment designed in `specs/1722-rn-client-k8s/plan.md` ("Kubernetes deployment"): an orchestrator image, the chart `deploy/helm/micold`, a kind harness (`scripts/e2e-kind.sh`, `mise run e2e-kind`, CI job `kube-e2e`), and a Rust e2e test crate `crates/micold-e2e`. The orchestrator gains three flags (`--data-dir`, `--socket`, `--ops-addr`, each with an env twin) and a minimal ops HTTP listener (`/healthz`, `/readyz`). The API suite reaches the link protocol through a test-only `micold-daemon` sidecar sharing the orchestrator's `emptyDir` socket volume (D1). State is `file` on a PVC (D2). Network API and UI suites are placeholder slots until #1732 and #1954.

**Milestone order differs from the spec's table.** The spec lists M2 harness before M3 ops. The harness smoke ("install and wait Ready") is only meaningful with the readiness probe, and the chart's probes (FR-007) need the ops listener, so ops comes first. The split below is final; the spec table is updated to match in the same commit.

| Milestone | Delivers | Depends on | Live cluster needed |
|---|---|---|---|
| M1 Image and chart | FR-001..005, FR-016, FR-019 (data dir, socket) | none | no |
| M2 Ops endpoints | FR-006, FR-007 (probes, values-gated), FR-015, FR-019 (ops addr) | M1 (flags module, chart) | no |
| M3 kind harness | FR-008..010, FR-014, FR-018 (harness part) | M1, M2 | yes: CI rounds |
| M4 API e2e suite | FR-011..013, FR-017, SC-003 | M3 | yes: CI rounds |
| M5 Network API and UI slots | US4, FR-013 placeholders; real tests when #1732 / #1954 land | M4; external #1732, #1954 | later |

M1 and M2 touch disjoint files except `deploy/helm/micold/templates/deployment.yaml` (probes block) and the orchestrator argument parser; M2 starts after M1 merges. Each milestone becomes one GitHub sub-issue of #1981 with a GitHub milestone of the same name; "Depends on" is recorded as "blocked by" links between the sub-issues.

## Technical Context

**Language/Version**: Rust (workspace edition and MSRV from the root `Cargo.toml`), Helm v3 templates, POSIX/bash scripts, Dockerfile.

**Primary Dependencies**: existing `tokio`, `interprocess`, `tracing` in `micold-daemon`; the ops listener uses `tokio::net::TcpListener` with a hand-written HTTP/1.1 response for two fixed GET paths, adding no HTTP framework (research R3). `micold-e2e` uses `tokio`, `serde_json`, and the existing link-protocol types from `micold-core::orchestrator`. External tools, pinned with sha256 in one script: `kind` v0.30.0, `kubectl` v1.34.0 (existing pins), `helm` and `kubeconform` (new pins, versions chosen in the M1 task, checksums verified against the release files).

**Storage**: the orchestrator's `JsonRegistryStore` (`registry.json` under the data directory); on Kubernetes a PVC at `/var/lib/micold`. No database (D2).

**Testing**: `cargo test` for the flag parser and the ops listener (no cluster); `scripts/tests/*.test.sh` for the harness's pure parts; a chart conformance test (rendered-template checks) in `scripts/tests/` driven by `helm template`; `helm lint` and `kubeconform` in CI; `crates/micold-e2e` against kind in the `kube-e2e` job.

**Target Platform**: Linux (docker, kind). macOS/arm64: harness exits 0 with a skip.

**Project Type**: infrastructure and test harness over an existing Rust daemon.

**Performance Goals**: `kube-e2e` job under 20 minutes on a warm cache (SC-002); chart lint under 1 minute (SC-004).

**Constraints**: kind cannot run in cloud threads, so every live harness try is one CI round (~25 min); budget two rounds for M3 and two for M4 and keep the harness debuggable from uploaded logs. Builds serialize through `scripts/build-lock.sh`.

**Scale/Scope**: one orchestrator replica, two concurrent test daemons.

## Constitution Check

| Principle | Status | Note |
|---|---|---|
| I. Test-First | PASS | Every Rust change starts with a failing test: flag parser table test, ops listener integration tests, `micold-e2e` tests that fail against a deliberately broken chart (SC-003). Shell and chart code has shell tests written first (see Test strategy). |
| II. Multi-Session | PASS | The orchestrator holds no session; the listing test runs two daemons at once (spec edge case). No new per-session state. |
| III. Worktree Integration | PASS (n/a) | Nothing here touches worktrees or session locations. |
| IV. Local-First Storage | PASS | Local mode is unchanged: the new flags default to today's paths and the ops listener is off unless an address is given. `file` state on a PVC needs no external service; PostgreSQL stays out (rendering `postgres` fails). |
| V. TS+RN client, Rust core | PASS | Rust for the daemon changes and the e2e crate; no client code. The `ui` slot is Playwright against the web client when it exists, outside the RN client's own tests. |
| VI. Linux daemons, web reach | PASS | Linux-only harness; browser matrix declared by the `ui` slot (chromium first). |
| VII. Documentation | PASS | `docs/development/kube-e2e.md` (FR-018) and chart `README` land with M3 and M1; no end-user surface changes, so no user-guide page (M3 checks `scripts/check-user-guide-updated.sh` accepts a development-docs-only change). |
| VIII. Reusable UI | PASS (n/a) | No UI. |

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation

```text
specs/1981-kube-e2e/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
├── contracts/ {chart-values.md, harness-env.md, ops-http.md}
└── tasks.md            # next unit
```

### Source code (new and touched)

```text
Dockerfile.orchestrator                         # new: M1, multi-stage, distroless cc, UID 65532
.dockerignore                                    # touched/new: M1
deploy/helm/micold/                              # new: M1
├── Chart.yaml  values.yaml  values.schema.json  README.md
├── templates/{_helpers.tpl,deployment.yaml,service.yaml,pvc.yaml,web-deployment.yaml,web-service.yaml}
└── ci/{default.yaml,web-enabled.yaml,no-persistence.yaml,e2e.yaml}   # e2e.yaml: M4 (test sidecar)
crates/micold-daemon/src/orchestrator/{mod.rs,config.rs,ops.rs}        # config: M1; ops: M2
crates/micold-core/src/paths.rs (read only)      # data_dir() stays the default
crates/micold-e2e/                               # new workspace member: M3 skeleton, M4 suites
├── Cargo.toml  src/{lib.rs,cluster.rs,env.rs}
└── tests/{ops/,link/,restart/,network/,ui/}   # network/ui: M5 placeholders
e2e/ui/README.md                                 # M5 placeholder for Playwright
scripts/{e2e-kind.sh,install-kube-tools.sh,chart-check.sh}              # M3 / M3 / M1
scripts/tests/{e2e-kind.test.sh,install-kube-tools.test.sh,chart-conformance.test.sh}
.github/workflows/ci.yml                         # kube-chart (M1), kube-e2e (M3), gate needs (M3)
mise.toml                                        # tasks e2e-kind (M3), chart-check (M1)
docs/development/kube-e2e.md                     # M3
Cargo.toml                                       # members += micold-e2e (M3)
```

**Structure Decision**: one new Rust crate for cluster tests (`micold-e2e`, mirrors `micold-client`'s feature-gated real-runtime tests but as its own target so `cargo test --workspace` cost stays small), chart under `deploy/helm/micold` exactly as the 1722 plan names it, scripts beside `scripts/test-kube.sh`.

## Design by requirement

| FR | Where | Layer that tests it |
|---|---|---|
| FR-001 | `Dockerfile.orchestrator` (builder `rust` stage `cargo build --release -p micold-daemon`, runtime `gcr.io/distroless/cc-debian12:nonroot`, digest-pinned, `USER 65532`, `ENTRYPOINT ["/micold-daemon","orchestrator"]`) | CI builds it (M1 job `kube-chart`); `docker run --user` check in the harness; seeded breakage "wrong user" in M4 |
| FR-002, FR-004 | chart templates, `values.yaml`, `values.schema.json` | `helm template` conformance test, `helm lint`, `kubeconform` |
| FR-003, FR-017 | `_helpers.tpl` `micold.stateMode` calls `fail` with the #1948/#1740 message; schema enum `["file"]` plus the helper for the friendlier message | conformance test renders `--set state.mode=postgres` and greps the message |
| FR-005 | `scripts/chart-check.sh` (lint plus kubeconform over default and `ci/*.yaml`), `mise run chart-check`, CI job `kube-chart` on chart/Dockerfile paths; `scripts/gate.sh` runs it when `helm` and `kubeconform` exist | the script itself; shell test with a bad chart fixture |
| FR-006 | `ops.rs`: `serve_ops(addr, Readiness)`; `Readiness` is an `Arc<AtomicBool>` pair (registry loaded, link listener bound) set by `serve` | integration tests over a real `TcpListener` on port 0 (unit layer: core logic is just state; HTTP framing in daemon) |
| FR-007 | chart probes block, enabled by `ops.enabled` (default true from M2; absent in M1 so M1's chart runs with no probe) | conformance test on rendered probes and port; live in M3 smoke |
| FR-008 | `scripts/e2e-kind.sh`; `mise run e2e-kind` wraps it with `scripts/build-lock.sh` | `scripts/tests/e2e-kind.test.sh` with stub `docker`/`kind`/`kubectl`/`helm` on PATH (missing tool, cleanup on failure, `KEEP`, exit status) |
| FR-009 | `scripts/install-kube-tools.sh` holds kind and kubectl pins and sha256; `kube-cluster` and `kube-e2e` both call it; `ci.yml` env pins removed from `kube-cluster` | `install-kube-tools.test.sh` (checksum mismatch aborts, using a local file URL) |
| FR-010 | `crates/micold-e2e`: `env.rs` reads `MICOLD_E2E`, skips with reason; `cluster.rs` wraps `kubectl port-forward` and `kubectl exec` | `cargo test --workspace` shows the skip reason; unit tests for env parsing |
| FR-011 | chart `ci/e2e.yaml` enables values `testDaemon.enabled` which adds a second container (image `micold-orchestrator` too, command `micold-daemon`) sharing the `/run/micold` emptyDir; tests drive it with `kubectl exec` of a small `micold-daemon` client command or the link crate | live (M4) |
| FR-012 | `tests/ops`, `tests/link`, `tests/restart` | live (M4) |
| FR-013 | `MICOLD_E2E_SUITES` handled by the harness (passes `--test <suite>` per suite); placeholders in `tests/network`, `tests/ui` with `#[ignore = "blocked on #1732"]`-style skip printing the issue | shell test of suite selection; `cargo test` shows placeholders skipped |
| FR-014 | `e2e-kind.sh` `collect_logs` into `target/e2e-logs/<cluster>/`; CI `actions/upload-artifact` `if: failure()` | shell test (failing suite stub leaves files); live artifact check in M3 |
| FR-015 | ops listener code kept pure enough that its added lines are covered by M2's integration tests | `scripts/diff-coverage.py` guardrail in CI |
| FR-016 | `scripts/tests/chart-conformance.test.sh` compares rendered image repository, `state.mode`, `database.existingSecret` and probe port against literal expectations copied from the 1722 plan | the test itself |
| FR-018 | `docs/development/kube-e2e.md` | `docs` CI job |
| FR-019 | `config.rs`: `OrchestratorConfig::from_args_env(args, env)` resolving flag over env over default (`paths::data_dir()`, `endpoint::orchestrator_endpoint()`), used by `run()`; M2 adds `ops_addr` | table-driven unit tests with an injected env map (no process env mutation); chart test asserts `--data-dir` equals the mount path |

### Key design points

- **Flag parsing** (`config.rs`): `parse_mode` still looks at the first argument; the rest of the arguments after `orchestrator` go to `OrchestratorConfig::parse`. Hand-rolled `--flag value` / `--flag=value`, matching the daemon's current style (research R4). Unknown flag: error naming it. `run()` builds the `Endpoint` from `--socket` (lock path `socket.with_extension("lock")`, same as `run_with`) and the store from `--data-dir`.
- **Ops listener** (`ops.rs`): `serve` takes an `Option<SocketAddr>`; when `Some`, it binds first and answers `/healthz` 200 immediately and `/readyz` 503 until `Readiness` has both flags, so the `startupProbe` sees the process alive while the registry loads. `serve` sets `registry_loaded` after `start()` and `link_bound` after the link listener binds. Any other path or method answers 404 or 405. A corrupt registry makes `start()` fail, `serve` returns the error, the process exits non-zero and the pod restarts (US3-4).
- **Chart**: `Deployment` `strategy: Recreate`, container args `["orchestrator","--data-dir=/var/lib/micold","--socket=/run/micold/orchestrator.sock"]` (+ `--ops-addr=0.0.0.0:9090` from M2), volumes `data` (PVC or `emptyDir`), `run` (`emptyDir` at `/run/micold`), `tmp` (`emptyDir` at `/tmp`), `automountServiceAccountToken: false`. Service exposes the ops port only in M2 onward; the link socket has no Service (it is local). `web.*` templates wrapped in `{{- if .Values.web.enabled }}`.
- **Harness flow** (`e2e-kind.sh`): check tools (`docker kind kubectl helm cargo`), `kind delete`-then-create cluster `${MICOLD_E2E_CLUSTER:-micold-e2e}` (reuse by name never leaves two), build image via `scripts/build-lock.sh docker build`, `kind load docker-image`, `helm upgrade --install micold deploy/helm/micold -n micold-e2e --create-namespace -f deploy/helm/micold/ci/e2e.yaml --wait --timeout 180s`, on wait failure print `kubectl get pods`, describe and log tail with the image name when `ImagePullBackOff`, run `cargo test -p micold-e2e --test <suite>` per suite with `MICOLD_E2E=1 MICOLD_E2E_REQUIRED=1`, trap cleans the cluster unless `MICOLD_E2E_KEEP=1`, `collect_logs` runs on non-zero exit before cleanup.
- **CI jobs**: `kube-chart` (no cluster: helm, kubeconform, conformance test, `docker build` smoke) added in M1; `kube-e2e` added in M3. Both are added to the final `gate` job's `needs:` and result checks, which `crates/micold-core/tests/ci_gate_covers_every_job.rs` enforces. `kube-e2e` has `timeout-minutes: 30` and `if: failure()` log upload.
- **Test daemon sidecar (D1)**: the sidecar is the same orchestrator image running `micold-daemon` (no new image). It registers over `/run/micold/orchestrator.sock`. A CI-only values file turns it on; production values never contain it.

## Test strategy by layer

| Layer | Covers |
|---|---|
| `micold-daemon` unit/integration (`cargo test -p micold-daemon`) | flag/env resolution, ops listener readiness transitions, corrupt-store exit |
| `scripts/tests/*.test.sh` (stubs on PATH) | harness arguments, cleanup, log capture, suite selection, tool pinning |
| chart tests (`chart-conformance.test.sh`, needs `helm`; skips with reason when absent, runs in `kube-chart`) | FR-002, 003, 004, 007, 016, 019 mount equals flag |
| `helm lint`, `kubeconform` | schema validity for every values file |
| `crates/micold-e2e` in kind | FR-011, FR-012, SC-003 seeded breakages |
| Geometry gates, visual pass | not applicable (no UI) |

## Risks and CI-round budget

| Risk | Mitigation |
|---|---|
| First live run fails for reasons invisible in the cloud | M3 first lands the harness with a minimal smoke only; logs, `describe` and events are uploaded from the first round; the shell stubs cover control flow |
| Docker build of the workspace is slow or large | `.dockerignore`, builder builds only `-p micold-daemon`, `Swatinem/rust-cache` is not usable inside docker so the job uses BuildKit cache via `docker/build-push-action` cache or `actions/cache` on a local layer cache (decide in M3 from measured time against SC-002) |
| distroless has no shell | probes are HTTP, no `exec` probes; debugging via `kubectl debug` ephemeral container, documented |
| PVC provisioning on kind | kind's default `standard` StorageClass (local-path) is used; `ReadWriteOnce` |
| Registry file ownership on the PVC | `securityContext.fsGroup: 65532` in the chart |
| Seeded breakages (SC-003) are slow | five breakages run as one extra values-file matrix inside the same cluster (each a `helm upgrade` expected to fail the suite), not five clusters |

## Complexity Tracking

None.
