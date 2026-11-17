Title: #1985: kind e2e M4: API e2e suite (ops, link, restart, refusal, seeded breakages)

## Milestone M4 of 5 — kube-e2e (specs/1981-kube-e2e)

**Deliverable**: the kind job runs the ops, restart, link and refusal suites and fails on each of the five seeded breakages.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon e2e_link`; `scripts/tests/chart-conformance.test.sh`; `scripts/tests/e2e-kind.test.sh`; CI job `kube-e2e`
**Tasks**: T023–T030 (T030 is the live CI round) · **Satisfies**: US3 acceptance scenarios 1–4; FR-011–013, FR-017, SC-003

What changed:
- `micold-daemon e2e-link idle | register --name N [--hold] --socket P | list --socket P` (`e2e_link.rs`): test support that dials the orchestrator's link socket with the real link handshake. `register` lists the daemon online and, without `--hold`, lets go so it lists offline; `--hold` stays linked until stdin closes; `list` prints the listing JSON; `idle` waits for SIGTERM.
- Chart: `testDaemon.enabled` adds a second container (same image, `/micold-daemon e2e-link idle`, read-only root, shares `/run/micold`); `testing.corruptRegistry.enabled` adds a digest-pinned busybox init container that writes an invalid `orchestrator/registry.json`; `testing.breakage` (`none|command|user|volume|dataDir|opsPort`) renders one deliberately wrong manifest. `ci/e2e.yaml` enables the sidecar; `ci/corrupt-registry.yaml` is new.
- `crates/micold-e2e`: suites `link` (register, offline after disconnect, two concurrent daemons), `restart` (fresh volume empty, then register, delete pod, listed offline), `refusal` (corrupt registry: never Ready, log names `registry.json`), and `ops` gains a 404 check and a security-context check (non-root, read-only root).
- `scripts/e2e-kind.sh`: one `cargo test` per suite in the order ops, restart, link; then `refusal` after a reinstall with `ci/corrupt-registry.yaml`; then the SC-003 matrix (uninstall + PVC delete, install with `--set testing.breakage=<v>`, run ops/restart/link until one fails; a breakage nobody catches fails the run). `MICOLD_E2E_MATRIX` forces the matrix on or off. Later steps still run after a failure so one CI round reports everything.

## Local gate
Cloud thread: no kind, kubectl or docker daemon, so the suites skip and `kube-e2e` is the live run (T030, not yet done). Run here: `micold-daemon --lib e2e_link` (8), `micold-e2e` (all targets, skips and unit tests), clippy `-D warnings` on `micold-daemon` and `micold-e2e`, `chart-conformance` (80 checks, helm v3.19.0 built from source), `e2e-kind` (33 cases), `classify-change`. `chart-check` (kubeconform) and the full `scripts/gate.sh` were not run: CI is the gate.

Refs #1985
Refs #1981

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvJzDpFKk2nWx3XXKkp
