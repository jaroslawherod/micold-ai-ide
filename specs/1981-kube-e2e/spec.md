# Feature Specification: End-to-end tests on a kind cluster, with a Helm chart for the orchestrator and web UI

**Feature Branch**: `1981-kube-e2e`

**Created**: 2026-11-15

**Status**: Draft

**Input**: Issue #1981 (epic). jaro: "I would like in parallel to start a work to test setup in k8s cluster (use kind in CI and also for local setup), prepare a helm chart that will deploy orchestrator and web ui (when will be developed) and start to run e2e tests against it. First cover only rest api or what ever api it will expose. It should also cover UI tests when it will be available." Then: "prepare an epic with plan and github issues with milestones", "then start implementation".

## Context (verified in the tree, 2026-11-15)

- The orchestrator is `micold-daemon orchestrator` (`crates/micold-daemon/src/orchestrator/`, rules in `crates/micold-core/src/orchestrator/`). It binds a **local socket only** (`interprocess`), accepts daemon links (#1724, #1725), keeps a registry (JSON file under the data directory, `JsonRegistryStore`) and answers a listing query over the link protocol. It has **no TCP listener, no HTTP, no `/healthz`**. Today "the API it exposes" is the link protocol on that socket.
- Not built yet: the network API (#1732: JSON over WebSocket `/api/ws`, `/auth/*`), ops endpoints (#1742: `/healthz`, `/readyz`, `/metrics`), the PostgreSQL store (#1948, M1 merged; SQLite for local mode), the web image (#1954) and the web client.
- `specs/1722-rn-client-k8s/plan.md`, section "Kubernetes deployment", is the design of the chart: chart path `deploy/helm/micold`, image `ghcr.io/jaroslawherod/micold-orchestrator` (distroless `cc`, UID 65532, read-only root FS), `database.existingSecret`, probes on the ops port 9090, `strategy: Recreate` at `replicas: 1`. #1740 (open, blocked) owns the production chart and release images. This epic **builds the first slice of that chart under those names** so #1740 extends it instead of replacing it.
- CI already has a `kube-cluster` job (`.github/workflows/ci.yml`) with `kind` v0.30.0 and `kubectl` v1.34.0 pinned by sha256, and `scripts/test-kube.sh`. This epic reuses that install pattern.
- Cloud threads cannot run `kind`; every live try is one CI round (~25 min). Linux only.

## Ownership split

| Concern | Owner |
|---|---|
| Orchestrator image (Dockerfile, build in CI, local `:dev` tag), chart skeleton, optional disabled web component, `helm lint` + `kubeconform` | **this epic** (M1) |
| kind harness: local task, CI job, teardown, log capture | **this epic** (M2) |
| Minimal ops listener: `/healthz` and `/readyz` only, on a configurable address (default off outside the chart) | **this epic** (M3), so probes and the first HTTP e2e exist |
| `/metrics`, richer readiness (DB migrated, CA loaded), shutdown choreography, ServiceMonitor | #1742 |
| Release images, multi-arch, cosign, ingress / Gateway API, NetworkPolicies, cert-manager, CloudNativePG in CI, Playwright smoke through the gateway | #1740 |
| Web image (`micold-web`) | #1954 (this epic adds only the disabled chart slot) |
| Network API (`/api/ws`, `/auth/*`) | #1732 (this epic adds its e2e suite when it lands) |
| API e2e suite on the link protocol and ops endpoints | **this epic** (M4) |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Deploy the orchestrator with one chart and see it healthy (Priority: P1)

A maintainer runs `helm install` of `deploy/helm/micold` into a cluster and gets a running orchestrator pod that Kubernetes reports ready, using the same image CI builds.

**Why this priority**: Every later test needs something deployable; it is also the first slice of #1740.

**Independent Test**: `helm lint` and `kubeconform` pass on the chart with default and each documented values file, with no cluster. Then `helm install` on a kind cluster reaches `Ready`.

**Acceptance Scenarios**:

1. **Given** default values, **When** the chart is rendered, **Then** it yields a Deployment (`replicas: 1`, `strategy: Recreate`), a Service and a PersistentVolumeClaim, with `runAsNonRoot`, UID 65532, read-only root FS, no privilege escalation, all capabilities dropped, `RuntimeDefault` seccomp, and an `emptyDir` at `/tmp`.
2. **Given** `web.enabled=false` (default), **When** rendered, **Then** no web resource exists; **Given** `web.enabled=true` with an image, **Then** a web Deployment and Service render.
3. **Given** `state.mode=file` (the only mode this epic supports), **When** rendered, **Then** a PersistentVolumeClaim (or `emptyDir` when `state.persistence.enabled=false`) is mounted at the data directory the orchestrator is configured with (FR-019). **Given** any other `state.mode` (`sqlite`, `postgres`), **Then** rendering fails with a message saying the mode is not available yet and naming #1948 and #1740. The value shape `state.mode` and `database.existingSecret` is reserved so those issues fill it in.

---

### User Story 2 - One command brings up a kind cluster and runs the e2e suite, locally and in CI (Priority: P1)

A developer runs one `mise run` task on Linux and gets: a kind cluster, the images built and loaded, the chart installed, the suite run, the cluster removed, logs kept on failure. CI runs the same script.

**Why this priority**: jaro asked for kind in CI and for local setup; one script keeps them identical.

**Independent Test**: The script's pure parts (argument handling, cleanup on failure, log capture paths) have shell tests under `scripts/tests/`. The live run is the CI job.

**Acceptance Scenarios**:

1. **Given** docker, kind and kubectl on PATH, **When** the task runs, **Then** the cluster `micold-e2e` is created, torn down at exit (kept with `MICOLD_E2E_KEEP=1`), and exit status is the suite's.
2. **Given** a missing tool, **When** the task runs, **Then** it stops before any cluster work and names the tool.
3. **Given** a failing suite, **When** the run ends, **Then** pod logs, `kubectl describe` and events are saved under a known directory and CI uploads them as an artifact.
4. **Given** a cloud thread or a macOS machine, **When** the e2e test binary runs without the harness, **Then** it skips with a stated reason, and fails instead when `MICOLD_E2E_REQUIRED=1` (as `MICOLD_KUBE_TEST_REQUIRED` does).

---

### User Story 3 - API e2e tests against the deployed orchestrator (Priority: P1)

A maintainer sees tests that drive the orchestrator's real API inside the cluster and fail when the deployment, the image or the protocol breaks.

**Why this priority**: The point of the epic.

**Independent Test**: In kind, the suite passes against the chart; breaking the chart (wrong socket path, missing PVC) fails it.

**Acceptance Scenarios**:

1. **Given** the installed chart, **When** the suite calls `/healthz` and `/readyz` on the ops port, **Then** both answer 200. (Not-ready before the registry loads is proven by an integration test with a slow or failing injected store, FR-015; the orchestrator restores the registry before binding, so a cluster test cannot observe it.)
2. **Given** the installed chart, **When** a test daemon link connects to the orchestrator and registers, **Then** a listing query returns that daemon, and after it disconnects the listing shows it offline.
3. **Given** `state.mode=file` on a PVC, **When** the orchestrator pod is deleted and its replacement is ready, **Then** the registry still lists the daemon registered before.
4. **Given** a corrupt registry file on the volume, **When** the pod starts, **Then** it never becomes Ready (probe or restart loop) and its log names the file (existing FR-004 behaviour, now proven in the cluster).

---

### User Story 4 - The suite grows to the network API and the web UI without restructuring (Priority: P2)

When #1732 lands, its WebSocket and auth endpoints get their own suite in the same harness; when the web client lands, Playwright tests run against the same release.

**Why this priority**: jaro: "It should also cover UI tests when it will be available." Blocked on other issues, so P2, but the layout is decided now.

**Independent Test**: Adding a suite directory plus one line in the harness's suite list is the only change needed to run a new suite; a placeholder suite documents and skips with the blocking issue number.

**Acceptance Scenarios**:

1. **Given** the harness, **When** a suite named in `MICOLD_E2E_SUITES` (default: all enabled) is selected, **Then** only that suite runs.
2. **Given** `web.enabled=true` and a reachable web URL, **When** the `ui` suite runs, **Then** it runs Playwright with the base URL from the harness; with no web image it is skipped with "blocked on #1954".

---

### Edge Cases

- Fresh, empty PVC (empty registry): the pod becomes Ready and the listing is empty.
- Cluster already exists from an aborted run: the harness reuses or recreates it by name, never leaves two.
- Image not loaded into kind (`ImagePullBackOff`): the harness fails with the image name, not a generic timeout.
- Two e2e runs on one machine: distinct cluster names via `MICOLD_E2E_CLUSTER`; one build at a time through `scripts/build-lock.sh` (shared target directory).
- Slow CI node: every wait has a stated timeout and prints pod status on expiry.
- A pod restarts mid-test (Recreate rollout): tests wait for readiness, never sleep a fixed time.
- Arm64 or macOS developer: the image build and harness are Linux-only; `mise run e2e-kind` says so and exits 0 with a skip, not a failure.
- Multi-session (Principle II): the listing test runs two test daemons at once and sees both.
- Browser differences (Principle VI): only the `ui` suite is affected; it declares its browser matrix (chromium first, others when #1954 lands).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The repository MUST contain a Dockerfile that builds the orchestrator image from the workspace, running `micold-daemon orchestrator` as UID 65532 on a distroless `cc` base, tagged `ghcr.io/jaroslawherod/micold-orchestrator` (`:dev` locally).
- **FR-002**: The chart `deploy/helm/micold` MUST render the orchestrator Deployment, Service and PersistentVolumeClaim with the security context of User Story 1, and MUST take image repository, tag and pull policy as values.
- **FR-003**: The chart MUST support only `state.mode=file` and MUST fail rendering for any other mode as in User Story 1. It reserves the value names `state.mode` and `database.existingSecret` per the 1722 plan; the `sqlite` and `postgres` modes belong to #1948 and #1740.
- **FR-004**: The chart MUST contain a `web` component, disabled by default, rendering nothing when disabled.
- **FR-005**: `helm lint` and `kubeconform` (pinned versions, checksums) MUST run on the chart for default values and every values file under `deploy/helm/micold/ci/`, in CI on every chart or Dockerfile change and in `mise run gate`'s chart step where the tools exist.
- **FR-006**: The orchestrator MUST offer an ops HTTP listener, off unless an address is configured (`--ops-addr` / `MICOLD_ORCHESTRATOR_OPS_ADDR`), serving `GET /healthz` (200 when the event loop answers) and `GET /readyz` (200 only after the registry is loaded and the link listener is bound). It MUST NOT serve anything else; `/metrics` is #1742's.
- **FR-007**: The chart MUST wire `startupProbe`, `readinessProbe` (`/readyz`) and `livenessProbe` (`/healthz`) to the ops port.
- **FR-008**: One script (`scripts/e2e-kind.sh`) MUST create the kind cluster, build and `kind load` the image, install the chart with a values file, run the selected suites, and clean up, locally and in CI alike; `mise run e2e-kind` MUST call it.
- **FR-009**: CI MUST run the script in a `kube-e2e` job, with `kind` and `kubectl` installed by the same pinned-sha256 step as `kube-cluster` (extracted to one shared script or composite so the pins live once), uploading logs on failure.
- **FR-010**: The e2e tests MUST be a Rust test target (`crates/micold-e2e`, a workspace member whose tests all skip unless `MICOLD_E2E=1`, so `cargo test --workspace` stays green with no cluster) that talks to the cluster through `kubectl port-forward` or an in-cluster test pod, and MUST report the skip reason when not run by the harness.
- **FR-011**: The API suite MUST reach the link protocol the way a real daemon does: the orchestrator pod gets a second, test-only `micold-daemon` container (enabled by a CI values file) that shares the orchestrator's `emptyDir` socket volume. Decision D1 (agent-resolved, jaro can overturn). A network-link variant follows when #1729 lands.
- **FR-012**: The API suite MUST cover: ops endpoints, register and list, offline after disconnect, two concurrent daemons, restart persistence on a PVC, corrupt-registry refusal.
- **FR-013**: The suites MUST be directories under `crates/micold-e2e/tests/` (`ops`, `link`, `restart`, later `network`, `ui`) plus `e2e/ui/` for Playwright, selected by `MICOLD_E2E_SUITES`; a suite blocked on another issue MUST exist as a documented, skipping placeholder naming the issue.
- **FR-014**: The harness MUST save pod logs, `kubectl describe` output and events on failure and CI MUST upload them.
- **FR-015**: Diff coverage of `micold-daemon` and `micold-core` code this epic adds (the ops listener) MUST meet the repository's 80% guardrail through ordinary tests that need no cluster.
- **FR-016**: The chart's database value names and the image name MUST match `specs/1722-rn-client-k8s/plan.md` so #1740 extends this chart; a conformance test (rendered template check) MUST fail when a name drifts.
- **FR-017**: The default and only `state.mode` is `file` on a PVC (Decision D2, agent-resolved, jaro can overturn); `sqlite` and `postgres` arrive with #1948 and #1740.
- **FR-019**: A release build of `micold-daemon orchestrator` MUST accept its data directory (`--data-dir`), its local-socket path (`--socket`) and its ops address (`--ops-addr`) as flags, each also settable by environment (`MICOLD_ORCHESTRATOR_DATA_DIR`, `MICOLD_ORCHESTRATOR_SOCKET`, `MICOLD_ORCHESTRATOR_OPS_ADDR`); today the data directory comes from `micold_core::paths::data_dir()` (XDG/HOME, test-env override only) and the socket from `endpoint::resolve()`. Flags win over environment, environment over the defaults. M1 delivers the data dir and socket, M3 the ops address. No config file is added: the chart sets flags and env on the container, so no ConfigMap is needed in this epic. Fixed container paths: data `/var/lib/micold`, socket `/run/micold/orchestrator.sock` (an `emptyDir` at `/run/micold`, shared with the test daemon container per FR-011), ops `0.0.0.0:9090`. A chart render test MUST assert the volume mount path equals the `--data-dir` argument.
- **FR-018**: Documentation MUST say how to run the suite locally (`docs/development/kube-e2e.md`), what skips where, and how to add a suite.

### Key Entities

- **Release under test**: the Helm release `micold` in namespace `micold-e2e`, with its values file.
- **Suite**: a directory of tests with one transport (ops HTTP, link protocol, WebSocket, browser).
- **Harness**: the script and its environment variables (`MICOLD_E2E`, `MICOLD_E2E_REQUIRED`, `MICOLD_E2E_KEEP`, `MICOLD_E2E_CLUSTER`, `MICOLD_E2E_SUITES`).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A maintainer with docker, kind and kubectl runs one command and gets a pass or fail verdict for the deployed orchestrator, with no manual cluster step.
- **SC-002**: The `kube-e2e` CI job finishes within 20 minutes on a warm cache and leaves no cluster behind.
- **SC-003**: Breaking the chart or the image (wrong command, wrong user, missing volume, wrong data-dir flag, wrong ops port) fails the suite in 100% of the five seeded breakages listed here.
- **SC-004**: Chart lint and schema checks complete in under 1 minute with no cluster.
- **SC-005**: Adding a new suite (network API, UI) needs no change outside its own directory, the suite list and the values file.
- **SC-006**: The ops listener's added lines have at least 80% diff coverage.

## Milestones (GitHub milestones and issues)

| Milestone | Delivers | Needs a live cluster to prove |
|---|---|---|
| M1 Image and chart | FR-001 to FR-005, FR-016, FR-019 (data dir and socket) | no (lint, kubeconform, template tests) |
| M2 kind harness | FR-008 to FR-010 (harness, local task, CI job with a smoke "install and wait Ready"), FR-014, FR-018 | yes, one CI round |
| M3 Ops endpoints | FR-006, FR-007, FR-015, FR-019 (ops address) | unit/integration no; probes in M2's smoke |
| M4 API e2e suite | FR-011 to FR-013, FR-017 | yes |
| M5 Network API and UI slots | Placeholder suites, then real tests as #1732 and #1954 land (US4) | yes, later |

M3 can ship before M2 (it needs no cluster). M5 stays open until the blocking issues land; the epic closes after M4 with M5 tracked as separate issues.

## Assumptions

- The orchestrator keeps its local-socket link until #1729 adds a network link; the first suite therefore runs a test daemon next to it.
- `docker` is available to build the image locally and in CI; no registry is pushed to by this epic.
- The existing `kube-cluster` job stays as is; `kube-e2e` is separate so a failure points at one of the two.
- Playwright and a web client do not exist; M5 is scoped as a slot only.

## Out of Scope

Release publishing, multi-arch images, cosign, ingress/Gateway API, NetworkPolicies, cert-manager, CloudNativePG, metrics, authentication, the web client itself, Windows and macOS runs, load and chaos testing.
