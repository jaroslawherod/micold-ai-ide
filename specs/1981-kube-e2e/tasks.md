# Tasks: End-to-end tests on a kind cluster, with a Helm chart (#1981)

**Input**: spec.md, plan.md, research.md, data-model.md, contracts/. Test-first (Constitution I): each
implementation task follows its test task, and the test is seen failing first. Live kind runs happen
only in CI (one try = one CI round, ~25 min); keep every other check local. No user-guide task: no
end-user surface changes (plan, Constitution VII); PRs use `docs-not-needed` where the guide gate asks.

Format: `[ID] [P?] [Story] description with file path`. `[P]` = parallel (disjoint files).
Stories: US1 chart, US2 harness, US3 API e2e suite, US4 extensible suites.

## Phase 1: Setup and Foundational (M1)

- [ ] T001 [US1] Test first: table tests for `OrchestratorConfig::from_args_env(args, env)` in `crates/micold-daemon/src/orchestrator/config.rs` (flag over env over default, `--flag value` and `--flag=value`, unknown flag error names it, injected env map). Must fail to compile/run before T002.
- [ ] T002 [US1] Implement `config.rs` (`data_dir`, `socket`), wire `OrchestratorConfig` into `run()` in `crates/micold-daemon/src/orchestrator/mod.rs`; lock path `socket.with_extension("lock")`; defaults unchanged (`paths::data_dir()`, `endpoint::orchestrator_endpoint()`). FR-019.
- [ ] T003 [P] [US1] Check first: write the `docker build` plus `docker run --user` smoke step of T009 (script in `scripts/chart-check.sh` or the job) and see it fail with no Dockerfile. Then `Dockerfile.orchestrator` (builder stage `cargo build --release -p micold-daemon`, digest-pinned distroless cc nonroot, `USER 65532`, entrypoint `/micold-daemon orchestrator`) and `.dockerignore`. FR-001.
- [ ] T004 [US1] Test first: `scripts/tests/chart-conformance.test.sh` (skips with reason without `helm`): renders default, `web.enabled`, `state.persistence.enabled=false`, `state.mode=postgres` (fails with the #1948/#1740 message), checks security context, `Recreate`, mount path equals `--data-dir`, image repository, `state.mode`, `database.existingSecret` against 1722-plan literals. Red before T005. FR-002..004, FR-016, FR-019.
- [ ] T005 [US1] Chart `deploy/helm/micold/`: `Chart.yaml`, `values.yaml`, `values.schema.json`, `templates/{_helpers.tpl,deployment.yaml,pvc.yaml,web-deployment.yaml,web-service.yaml}`, `ci/{default,web-enabled,no-persistence}.yaml`, `README.md` (no Service or probes in M1). Makes T004 green.
- [ ] T006 [P] [US1] `scripts/install-kube-tools.sh` with pinned sha256 for `helm` and `kubeconform` (versions chosen here), plus `scripts/tests/install-kube-tools.test.sh` written first (checksum mismatch aborts, local file URL). FR-005, FR-009 (helm/kubeconform part; M3 adds kind and kubectl).
- [ ] T007 [US1] Test first then implement `scripts/chart-check.sh` (helm lint plus kubeconform over default and every `ci/*.yaml`, then the conformance test): shell test with a bad-chart fixture; `mise.toml` task `chart-check`; hook into `scripts/gate.sh` when `helm` and `kubeconform` exist. FR-005, SC-004.
- [ ] T008 [P] [US1] Test first then implement `kube_paths` output in `scripts/classify-change.sh` with a case in `scripts/tests/classify-change.test.sh` (paths: `deploy/helm/**`, `Dockerfile.orchestrator`, `scripts/e2e-kind.sh`, `scripts/install-kube-tools.sh`, `crates/micold-e2e/**`).
- [ ] T009 [US1] CI job `kube-chart` in `.github/workflows/ci.yml` (install tools via T006, `mise run chart-check`-equivalent, `docker build -f Dockerfile.orchestrator` smoke, `docker run --user` check), gated by `kube_paths`, added to `ci-complete` `needs:` and result checks (`crates/micold-core/tests/ci_gate_covers_every_job.rs` passes).

## Phase 2: Ops endpoints (M2)

- [ ] T010 [US1] Test first: integration tests for `serve_ops` in `crates/micold-daemon/src/orchestrator/ops.rs` over a real `TcpListener` on port 0: `/healthz` 200 immediately, `/readyz` 503 until both `Readiness` flags set then 200, other path 404, other method 405, `Connection: close` (contracts/ops-http.md). FR-006, FR-015.
- [ ] T011 [US1] Implement `ops.rs` (`Readiness`, `serve_ops`, hand-written HTTP/1.1, no new dependency).
- [ ] T012 [US1] Test first: `ops_addr` resolution (flag `--ops-addr`, env `MICOLD_ORCHESTRATOR_OPS_ADDR`, default `None`, invalid address error names the source) in `config.rs`; then implement. FR-019.
- [ ] T013 [US1] Test first: `serve` with `Some(addr)` is not-ready while the store loads, ready after the link listener binds; a failing store makes `serve` return the error (non-zero exit). Then wire `serve` in `mod.rs` to set `registry_loaded` and `link_bound`. FR-006, US3-4.
- [ ] T014 [US1] Test first: extend `chart-conformance.test.sh` with Service (port 9090), `--ops-addr=0.0.0.0:9090` arg, startup/readiness/liveness probes gated by `ops.enabled`; then add `templates/service.yaml`, probes in `deployment.yaml`, values and schema, README. FR-007.
- [ ] T015 [US1] Run `scripts/diff-coverage.py` on the ops and config code; add tests until at least 80%. FR-015, SC-006.

## Phase 3: kind harness (M3)

- [ ] T016 [US2] Test first: `scripts/tests/e2e-kind.test.sh` with stub `docker`/`kind`/`kubectl`/`helm`/`cargo` on PATH: missing tool named before cluster work, `ImagePullBackOff` diagnostics name the image, a distinct `MICOLD_E2E_CLUSTER` name is honoured, cluster deleted at exit, `MICOLD_E2E_KEEP=1` keeps it, exit status is the suite's, `collect_logs` runs on failure into `target/e2e-logs/<cluster>/`, `MICOLD_E2E_SUITES` selects `--test <suite>`. FR-008, FR-013, FR-014.
- [ ] T017 [US2] Implement `scripts/e2e-kind.sh` per plan harness flow (delete-then-create cluster, `scripts/build-lock.sh docker build`, `kind load`, `helm upgrade --install ... -f ci/e2e.yaml`, failure diagnostics, trap cleanup) and a first `deploy/helm/micold/ci/e2e.yaml` (ops enabled); mise task `e2e-kind`. macOS/arm64: exit 0 with a skip.
- [ ] T018 [P] [US2] Extend `scripts/install-kube-tools.sh` with the `kind` v0.30.0 and `kubectl` v1.34.0 pins and tests; make the `kube-cluster` job call it and drop its `ci.yml` env pins. FR-009.
- [ ] T019 [US2] Test first then implement `crates/micold-e2e` skeleton (workspace member in `Cargo.toml`): `env.rs` unit tests (skip unless `MICOLD_E2E=1`, reason printed, `MICOLD_E2E_REQUIRED=1` fails instead), `cluster.rs` (`kubectl port-forward`/`exec` wrappers), and `tests/ops/main.rs` (`mod smoke;`, a cargo test target per suite directory; `/healthz`, `/readyz` through port-forward). FR-010, US2-4.
- [ ] T020 [US2] CI job `kube-e2e` (30 min timeout, tools via T018, image build cache decided from measured time, `upload-artifact` of `target/e2e-logs` `if: failure()`), gated by `kube_paths`, added to `ci-complete`. FR-009, FR-014, SC-002.
- [ ] T021 [P] [US2] `docs/development/kube-e2e.md`: local run, skips, env vars, adding a suite, debugging with `kubectl debug`; confirm `scripts/check-user-guide-updated.sh` accepts a development-docs-only change (else add the `docs-not-needed` label). FR-018.
- [ ] T022 [US2] Live verification: push and read the `kube-e2e` CI round(s) (budget two), fix harness issues from the uploaded logs until the smoke passes and the cluster is removed.

## Phase 4: API e2e suite (M4)

- [ ] T023 [US3] Test first: unit tests for `micold-daemon e2e-link` verbs (`idle`, `register --name N [--hold] --socket P`, `list --socket P` printing JSON) against an in-process orchestrator link endpoint, in `crates/micold-daemon/src/e2e_link.rs`. Then implement and wire in `main.rs`. FR-011.
- [ ] T024 [US3] Red first: suites in `crates/micold-e2e/tests/{ops,link,restart}/main.rs` (`mod` per file, cargo test targets): `ops` (both endpoints), `link` (register, list, offline after disconnect, two concurrent daemons via `--hold`), `restart` (fresh empty PVC lists no daemons; register A, delete pod, wait Ready, list shows A offline). Run against the chart without the sidecar: `link` and `restart` fail (no `test-daemon` container). FR-012, US3-1..3.
- [ ] T025 [US3] Test first: extend `chart-conformance.test.sh` for `testDaemon.enabled` (second container, same image, command `/micold-daemon e2e-link idle`, shares the `/run/micold` emptyDir, read-only root FS; absent in production values). FR-011.
- [ ] T026 [US3] Implement the `testDaemon` template and `ci/e2e.yaml` enabling it; T024's suites go green in the live round T030.
- [ ] T027 [US3] Test first: conformance test for the values-gated corrupt-registry init container (`testing.corruptRegistry.enabled`, digest-pinned busybox writing invalid JSON to `orchestrator/registry.json`); then the template and `ci/corrupt-registry.yaml`.
- [ ] T028 [US3] Test first: `e2e-kind.test.sh` expects the harness to run the `refusal` suite after a reinstall with `ci/corrupt-registry.yaml` (default selection includes it); then `scripts/e2e-kind.sh`, and `crates/micold-e2e/tests/refusal/main.rs`: pod never Ready, log names `registry.json`. US3-4, FR-012.
- [ ] T029 [US3] Seeded-breakage matrix (SC-003): `ci/no-persistence.yaml` makes the restart test see A missing; wrong command, wrong user, missing volume, wrong `--data-dir`, wrong ops port each as a values override that must fail the suite; run inside one cluster via `helm upgrade`; shell test in `e2e-kind.test.sh` first. SC-003.
- [ ] T030 [US3] Live verification: `kube-e2e` CI round(s) (budget two) until the suites and the matrix pass; record durations against SC-002.

## Phase 5: Network API and UI slots (M5)

- [ ] T031 [P] [US4] Placeholder suites `crates/micold-e2e/tests/network/main.rs` and `tests/ui/main.rs`: tests that skip printing "blocked on #1732" / "blocked on #1954"; `e2e/ui/README.md` for the future Playwright suite with base URL from the harness. FR-013.
- [ ] T032 [US4] Test first: shell test that a new suite name in `MICOLD_E2E_SUITES` selects only it, every listed suite is a cargo test target (`tests/<suite>/main.rs`), and a placeholder is listed as skipped (extend `e2e-kind.test.sh`); adjust the harness suite list; add "adding a suite" steps for network and ui to `docs/development/kube-e2e.md`. SC-005.

## Dependencies

M1 -> M2 -> M3 -> M4 -> M5. M5's real tests are tracked as separate issues blocked by #1732 and #1954.

## Implementation Strategy

MVP is M1 (image, chart, flags). Land each milestone as its own PR. Cloud threads cannot run kind: M3
and M4 have a live-verification task, each a CI round; every shell and chart behaviour is covered
beforehand by stubs and rendered-template tests so a live round only finds environment problems.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 (#1982) — Image and chart 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: `mise run chart-check` lints and renders the `deploy/helm/micold` chart (Deployment, PVC, optional web), the orchestrator image builds, and `micold-daemon orchestrator` accepts `--data-dir` and `--socket`.
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001–005, FR-016, FR-019 (data dir, socket)
- **Verify**: `scripts/chart-check.sh`; `scripts/build-lock.sh cargo test -p micold-daemon orchestrator::config`; CI job `kube-chart`
- **Depends on**: —
- **Tier**: full

### M2 (#1983) — Ops endpoints

- **Tasks**: T010–T015
- **Deliverable**: with `--ops-addr`, the orchestrator serves `/healthz` and `/readyz`; the chart renders the Service and probes.
- **Satisfies**: US1 (Service and probes); corrupt-store exit (prerequisite of US3 scenario 4, proven in M4); FR-006, FR-007, FR-015, FR-019 (ops address)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon orchestrator::ops`; `scripts/chart-check.sh`; `scripts/diff-coverage.py`
- **Depends on**: M1
- **Tier**: full

### M3 (#1984) — kind harness

- **Tasks**: T016–T022
- **Deliverable**: `mise run e2e-kind` and CI job `kube-e2e` create a kind cluster, install the chart, run the ops smoke, clean up and upload logs on failure.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-008–010, FR-014, FR-018, SC-001, SC-002
- **Verify**: `scripts/tests/e2e-kind.test.sh`; CI job `kube-e2e` green; `mise run e2e-kind` on Linux
- **Depends on**: M2
- **Tier**: full

### M4 (#1985) — API e2e suite

- **Tasks**: T023–T030
- **Deliverable**: the kind job runs the ops, link, restart and refusal suites and fails on each of the five seeded breakages.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-011–013, FR-017, SC-003
- **Verify**: CI job `kube-e2e` green with the suites and the breakage matrix; `scripts/build-lock.sh cargo test -p micold-daemon e2e_link`
- **Depends on**: M3
- **Tier**: full

### M5 (#1986) — Network API and UI slots

- **Tasks**: T031–T032
- **Deliverable**: `network` and `ui` suites exist as documented placeholders that skip naming #1732 and #1954; selecting a suite runs only it. Real tests follow as separate issues when those land.
- **Satisfies**: US4 acceptance scenarios 1–2; FR-013, SC-005
- **Verify**: `scripts/tests/e2e-kind.test.sh`; `MICOLD_E2E=1 cargo test -p micold-e2e --test ui` prints the skip reason
- **Depends on**: M4
- **Tier**: light
