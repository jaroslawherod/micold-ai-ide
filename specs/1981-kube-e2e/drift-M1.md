# Plan drift, M1

Recorded against `scripts/autopilot/plan-drift.sh specs/1981-kube-e2e M1`.

## UNPLANNED (changed, named by no M1 task)

- `.gitattributes`: the `micold-kube` attribute. `classify-change.sh` never hard-codes a path (its
  header: "Declaration: .gitattributes"), so T008's `kube_paths` is declared there like
  `micold-daemon-runtime`. Beyond T008's list it also marks `scripts/chart-check.sh` and
  `scripts/tests/chart-conformance.test.sh`, the job's own check, so editing them runs the job.
- `scripts/tests/chart-check.test.sh`: T007's "shell test with a bad-chart fixture"; the fixtures
  are generated inside the test, so no fixture directory is added.
- `crates/micold-daemon/src/main.rs`: passes the arguments after `orchestrator` to
  `orchestrator::main(args)` (T002's wiring needs the arguments).
- `deploy/helm/micold/templates/*`: the container sets `command: ["/micold-daemon"]` and
  `args: ["orchestrator", ...]`; with the image's `ENTRYPOINT ["/micold-daemon","orchestrator"]`
  the plan's `args: ["orchestrator", ...]` alone would have passed `orchestrator` twice.
- `deploy/helm/micold/values.schema.json`: `state.mode` enum is `file|sqlite|postgres`, not plan's
  `["file"]`. Helm validates the schema before rendering, so `["file"]` would hide the helper's
  #1948/#1740 message behind a generic schema error; the helper still fails every mode but `file`.

### Named by a task, but in a form the script does not parse

- `Dockerfile.orchestrator` and `.dockerignore`: T003 names both; the script only matches paths
  with a known extension.
- `deploy/helm/micold/templates/_helpers.tpl`, `deploy/helm/micold/ci/default.yaml`,
  `deploy/helm/micold/ci/no-persistence.yaml`, `deploy/helm/micold/ci/web-enabled.yaml`: T005 names
  them in brace lists (`templates/{_helpers.tpl,...}`, `ci/{default,web-enabled,no-persistence}.yaml`).

## MISSING (planned, no change in the diff)

- `crates/micold-core/tests/ci_gate_covers_every_job.rs`: T009 requires it to pass, not to change.
  It passes unchanged (`cargo test -p micold-core --test main ci_gate_covers_every_job`: 4 passed);
  the test lives in the `main` target, not its own binary.
- `scripts/e2e-kind.sh`: T008 names it as a `kube_paths` path; the script itself is M3's (T016).
