# Quickstart (#1760)

## Part A: automated
1. Core and client units: `scripts/build-lock.sh cargo test -p micold-core kube::settings` and `-p micold-client daemon_config_delivery`; `mise run test-core`.
2. Live: `mise run test-kube` (docker, kind, kubectl; CI job `kube-cluster`). `kube_real_scoped_token` must pass; without the tools it prints `SKIP` and passes (`MICOLD_KUBE_TEST_REQUIRED` unset).
3. Revert check: comment out the token keys in `kube_attempt`; the live test fails at the push (US3.2).
4. `mise run gate` before pushing.

## Part B: visual pass (form row only)
Open Settings -> Daemons -> a Kubernetes daemon: the **GitHub tokens** tick is off by default, its label names the exposure, ticking and saving shows "applies on next start".

## Part C: manual
Step 7 against real GitHub stays manual (spec out of scope). Update row 11 of `specs/1002-sandbox-scoped-token/quickstart.md` to COVERED by `kube_real_scoped_token`.
