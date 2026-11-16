Title: #1982: kind e2e M1: orchestrator image and Helm chart

## Milestone M1 of 5 — kube-e2e (specs/1981-kube-e2e)

**Deliverable**: `mise run chart-check` lints and renders the `deploy/helm/micold` chart (Deployment, PVC, optional web), the orchestrator image builds, and `micold-daemon orchestrator` accepts `--data-dir` and `--socket`.
**Verify**: `scripts/chart-check.sh`; `scripts/build-lock.sh cargo test -p micold-daemon orchestrator::config`; CI job `kube-chart`
**Tasks**: T001–T009 · **Satisfies**: US1 acceptance scenarios 1–3; FR-001–005, FR-016, FR-019 (data dir, socket)

What changed:
- `micold-daemon orchestrator --data-dir <dir> --socket <path>` (env `MICOLD_ORCHESTRATOR_DATA_DIR`, `MICOLD_ORCHESTRATOR_SOCKET`). A flag wins over its variable, and a variable over the default. Defaults are resolved only when needed, so a container never touches `HOME` or `XDG_RUNTIME_DIR`. An unknown argument exits 1 and names it.
- `Dockerfile.orchestrator` + `.dockerignore`: a digest-pinned `rust:1.97-bookworm` builder and a digest-pinned `gcr.io/distroless/cc-debian12:nonroot` runtime, `USER 65532`, entrypoint `/micold-daemon orchestrator`.
- `deploy/helm/micold`: the orchestrator Deployment (1 replica, `Recreate`, hardened security context) and its PVC, or an `emptyDir` when persistence is off. The web Deployment and Service render only with `web.enabled`. Only `state.mode=file` renders; `sqlite` and `postgres` fail with a message naming #1948 and #1740. `database.existingSecret` is reserved. The values schema refuses unknown keys.
- `scripts/install-kube-tools.sh`: `helm` v3.19.0 and `kubeconform` v0.7.0, each checked against its pinned sha256. `scripts/chart-check.sh` runs helm lint, kubeconform and the conformance test; `--image` runs the docker build and the UID 65532 smoke run. The `mise run chart-check` task runs it, and `scripts/gate.sh` runs it when both tools are present.
- `classify-change.sh` has a new `kube_paths` output (attribute `micold-kube` in `.gitattributes`). The new CI job `kube-chart` is gated by it and is in `ci-complete`.

## Agent review
- Review A (code-review, high), 3 rounds:
  - r1: 2 MAJOR (the kube path set left out the image's inputs and the job's own files) and 8 MINOR. 8 were fixed and 2 declined: the appVersion tag, because publishing images is #1740's work, and the explicit socket's directory mode, which is now documented.
  - r2: 2 MAJOR (the smoke cleanup trap read a local variable; the `pr_proven` skip lost the main-push safety net) and 8 MINOR. 9 were fixed and 1 declined: splitting the gate for the M3 kind job is M3's call.
  - r3: CLEAN, 5 MINOR, 2 of them fixed.
- Review B (conformance, sonnet): dispatched by the orchestrator after this unit.

## Local gate
The full `scripts/gate.sh` was not run: the thread had about 5 GB of free disk, too little for a workspace test build. CI's full run is the gate for this PR. Results of what did run:
- fmt and clippy (`micold-daemon`, `-D warnings`) are clean, and `cargo test -p micold-daemon` passed every target. The two root-only permission tests were skipped because the gate runs them unprivileged.
- `micold-core` `ci_gate_covers_every_job` passed (4 tests), and every `scripts/tests/*.test.sh` passed. Those include chart-conformance (43 checks), chart-check (4 cases), classify-change (92 cases) and install-kube-tools (7 cases), run with helm and kubeconform built from source at the pinned versions.
- No docker daemon was available here, so the image build and `chart-check.sh --image` were not run locally. `kube-chart` runs them for the first time, along with the real tarball downloads that prove the sha256 pins.

Refs #1982
Refs #1981
https://claude.ai/code/session_01NCdYvPYKU7SsSjL4sMLBUt
