# Cycle Log: kube-e2e (#1981)

Append only. Newest last.

## M1 (#1982)

### Baseline

- suite: not run in full before the milestone; `cargo test -p micold-daemon --test orchestrator`
  73 passed after the change (the orchestrator's existing integration tests, unchanged).
- helm and kubeconform are not installable from their release URLs in the cloud thread (proxy
  403); for the local runs below they were built from source with `go install
  helm.sh/helm/v3/cmd/helm@v3.19.0` and `github.com/yannh/kubeconform/cmd/kubeconform@v0.7.0` into
  the session scratchpad. CI installs the pinned release tarballs.

### Cycle 1: T001/T002 `OrchestratorConfig::from_args_env`

- tests: `crates/micold-daemon/src/orchestrator/config.rs` `mod tests` (4 table tests:
  precedence flag > env > default incl. `--flag=value`, lock beside an explicit socket, empty env
  as unset; bad arguments named; defaults not resolved when both values are given; an unresolvable
  default is an error).
- red: body stubbed to `unimplemented!("T002")`:
  `scripts/build-lock.sh cargo test -p micold-daemon --lib orchestrator::config`
  -> `test result: FAILED. 0 passed; 4 failed`
- green: real body: same command -> `test result: ok. 4 passed; 0 failed`; `run()`/`main(args)`
  wired, `cargo test -p micold-daemon --test orchestrator` 73 passed.
- refactor: clippy `type_complexity` on the case table -> `PrecedenceCase` alias; the `=` split
  kept UTF-8 only (no `unsafe` encoded-bytes split).

### Cycle 2: T008 `kube_paths`

- test: `scripts/tests/classify-change.test.sh` kube cases (13, then 2 more for the chart scripts).
- red: `scripts/tests/classify-change.test.sh` -> `classify-change: 15 of 81 case(s) failed`
  (`got=<none>`: no `kube_paths=` output).
- green: `micold-kube` attribute in `.gitattributes` and the output in `classify-change.sh` ->
  `classify-change: all 83 cases passed`.

### Cycle 3: T006 `install-kube-tools.sh`

- test: `scripts/tests/install-kube-tools.test.sh` (7 cases: mismatch aborts and installs nothing,
  failed download, unknown tool, pins well formed).
- red: no script -> `install-kube-tools: 4 of 7 case(s) failed` (the 3 "nothing installed" cases
  pass vacuously without the script).
- green: `install-kube-tools: all 7 cases passed`.

### Cycle 4: T004/T005 chart conformance

- test: `scripts/tests/chart-conformance.test.sh` (40 checks).
- red: `CHART=<copy with Chart.yaml and ci/ only, no templates or values>` ->
  `chart-conformance: 33 of 36 check(s) failed`. The chart files had been drafted before the test
  was run, so the red run used a template-less copy rather than the empty tree.
- green: `chart-conformance: all 40 checks passed`; `helm lint --strict` clean; kubeconform
  `Valid: 2, Invalid: 0`.

### Cycle 5: T007 `chart-check.sh`

- test: `scripts/tests/chart-check.test.sh` (good fixture passes; template error fails at
  `helm lint`; a manifest invalid only under `ci/two.yaml` fails at kubeconform naming the file).
- red: no script -> `chart-check: 3 of 3 case(s) failed`.
- green: `chart-check: all 3 cases passed`; `scripts/chart-check.sh` on the real chart ->
  `chart-check: ok (4 values set(s))`.

### Cycle 6: T003 image smoke

- check: `scripts/chart-check.sh --image` written first.
- red: `chart-check: no /home/user/micold-ai-ide/Dockerfile.orchestrator` (exit 1).
- green: not runnable here (no docker daemon in the cloud thread); the `kube-chart` CI job runs
  `scripts/chart-check.sh --image` (build, image user 65532, entrypoint refuses an unknown flag,
  serves on a read-only root with `--data-dir` and `--socket`).

### Review A fixes (code-review high, 10 findings)

- tests added together with each fix (no separate red run): `config.rs` case "a flag where the value goes"
  (`--data-dir --socket=/x`), conformance "an empty image.repository is refused" and an exact
  Service-selector check, classify cases for the image inputs, ci.yml and job-only tests.
- green: `orchestrator::config` 4 passed; `chart-conformance: all 41 checks passed`;
  `classify-change: all 90 cases passed`; `chart-check: all 3 cases passed`.
