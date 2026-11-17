#!/usr/bin/env bash
# Drives scripts/e2e-kind.sh (#1981/#1984, FR-008, FR-013, FR-014) with stub docker, kind, kubectl,
# helm and cargo on PATH: no cluster, no network. Each stub appends its arguments to calls.log;
# STUB_HELM_RC, STUB_CARGO_RC and STUB_PODS steer them. The live run is CI's kube-e2e job.

set -uo pipefail

ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/e2e-kind.sh"

failures=0
cases=0
ok() { cases=$((cases + 1)); printf 'ok    %s\n' "$1"; }
fail() { cases=$((cases + 1)); failures=$((failures + 1)); printf 'FAIL  %s\n' "$1"; }
check() { if eval "$2"; then ok "$1"; else fail "$1 (exit $rc; out: $out; calls: $(tr '\n' '|' <"$calls" 2>/dev/null))"; fi; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
stubs="$work/bin"
mkdir -p "$stubs"
calls="$work/calls.log"

stub() {
  cat >"$stubs/$1" <<EOF
#!/usr/bin/env bash
echo "$1 \$*" >>"$calls"
$2
EOF
  chmod +x "$stubs/$1"
}
stub docker 'exit 0'
stub kind 'exit 0'
# The helm stub records the seeded breakage in force (the state file) and fails an install the way
# STUB_HELM_RC says; the cargo stub fails while a breakage is in force with STUB_BREAK_CARGO_RC
# (default 1: the suites catch it), else with STUB_CARGO_RC.
stub helm 'case "$*" in
  uninstall*) : >"'"$work"'/breakage" ;;
  *"testing.breakage="*) sed -E "s/.*testing.breakage=([A-Za-z]+).*/\1/" <<<"$*" >"'"$work"'/breakage" ;;
esac
exit "${STUB_HELM_RC:-0}"'
stub cargo 'echo "MICOLD_E2E=${MICOLD_E2E:-} REQUIRED=${MICOLD_E2E_REQUIRED:-} NS=${MICOLD_E2E_NAMESPACE:-}" >>"'"$calls"'"
if [ -s "'"$work"'/breakage" ]; then
  b="$(cat "'"$work"'/breakage")"; want=ops; [ "$b" = volume ] && want=restart
  [ -n "${STUB_FAIL_SUITE:-}" ] && want="$STUB_FAIL_SUITE"
  case "$*" in *"--test $want "*) exit "${STUB_BREAK_CARGO_RC:-1}" ;; esac
  exit 0
fi
exit "${STUB_CARGO_RC:-0}"'
stub kubectl 'case "$*" in *"get pods"*) printf "%s\n" "${STUB_PODS:-micold-0 1/1 Running}" ;; *) echo "kubectl-out $*" ;; esac'
stub uname 'case "$1" in -s) echo "${STUB_OS:-Linux}" ;; -m) echo "${STUB_ARCH:-x86_64}" ;; esac'

# run [env...]: runs the harness with only the stubs and the system basics on PATH.
run() {
  : >"$calls"
  : >"$work/breakage"
  out="$(env PATH="$stubs:/usr/bin:/bin" MICOLD_NO_BUILD_LOCK=1 MICOLD_E2E_LOG_DIR="$work/logs" "$@" "$SCRIPT" 2>&1)"
  rc=$?
}

# A missing tool is named before any cluster work.
rm "$stubs/kind"
run
check "missing tool exits 1 naming it" '[ "$rc" -eq 1 ] && grep -q "kind" <<<"$out"'
check "missing tool: no cluster work" '! grep -q "^kubectl\|^helm\|^docker" "$calls"'
stub kind 'exit 0'

run STUB_OS=Darwin STUB_ARCH=arm64
check "not Linux amd64 exits 0 with a skip line" '[ "$rc" -eq 0 ] && grep -qi "skip" <<<"$out"'
check "skip: no cluster work" '! grep -q "^kind\|^helm\|^docker" "$calls"'

run
check "a passing run exits 0" '[ "$rc" -eq 0 ]'
check "cluster deleted before create (default name)" \
  '[ "$(grep -n "^kind delete cluster --name micold-e2e" "$calls" | head -1 | cut -d: -f1)" -lt "$(grep -n "^kind create cluster --name micold-e2e" "$calls" | cut -d: -f1)" ]'
check "image built and loaded" 'grep -q "^docker build -f .*Dockerfile.orchestrator" "$calls" && grep -q "^kind load docker-image .* --name micold-e2e" "$calls"'
check "chart installed with ci/e2e.yaml and --wait" 'grep -q "^helm upgrade --install micold .*deploy/helm/micold .*-f .*ci/e2e.yaml.*--wait" "$calls"'
check "suites run with MICOLD_E2E=1 and REQUIRED=1" 'grep -q "^cargo test -p micold-e2e --test ops" "$calls" && grep -q "MICOLD_E2E=1 REQUIRED=1 NS=micold-e2e" "$calls"'

# line_of <pattern>: the number of the first line of calls.log matching it.
line_of() { grep -n -- "$1" "$calls" | head -1 | cut -d: -f1; }

check "each suite runs on its own cargo test, restart before link" \
  '[ "$(line_of "^cargo test -p micold-e2e --test ops ")" -lt "$(line_of "^cargo test -p micold-e2e --test restart ")" ] && [ "$(line_of "^cargo test -p micold-e2e --test restart ")" -lt "$(line_of "^cargo test -p micold-e2e --test link ")" ]'
check "refusal runs after the other suites, after a reinstall with ci/corrupt-registry.yaml" \
  '[ "$(line_of "^cargo test -p micold-e2e --test link ")" -lt "$(line_of "^helm upgrade .*ci/corrupt-registry.yaml")" ] && [ "$(line_of "^helm upgrade .*ci/corrupt-registry.yaml")" -lt "$(line_of "^cargo test -p micold-e2e --test refusal")" ]'
check "the corrupt-registry install does not wait for Ready" '! grep "^helm upgrade .*ci/corrupt-registry.yaml" "$calls" | grep -q -- "--wait"'
check "the matrix installs each of the five breakages" \
  '[ "$(grep -c "^helm upgrade .*--set testing.breakage=" "$calls")" -eq 5 ] && for b in command user volume dataDir opsPort; do grep -q "^helm upgrade .*--set testing.breakage=$b " "$calls" || exit 1; done'
check "the matrix runs after refusal" '[ "$(line_of "^cargo test -p micold-e2e --test refusal")" -lt "$(line_of "^helm upgrade .*--set testing.breakage=")" ]'
check "the PVC is deleted before every breakage" \
  '[ "$(grep -c "^helm uninstall micold" "$calls")" -eq 5 ] && [ "$(grep -c "^kubectl .*delete pvc --all" "$calls")" -eq 5 ]'
check "a breakage the suites catch passes the run" '[ "$rc" -eq 0 ] && grep -q "breakage opsPort: caught by suite ops" <<<"$out"'
check "cluster deleted at exit" '[ "$(grep -c "^kind delete cluster --name micold-e2e" "$calls")" -ge 2 ]'

run MICOLD_E2E_CLUSTER=other-e2e
check "a distinct MICOLD_E2E_CLUSTER is honoured" 'grep -q "^kind create cluster --name other-e2e" "$calls" && ! grep -q -- "--name micold-e2e" "$calls"'

run MICOLD_E2E_KEEP=1
check "MICOLD_E2E_KEEP=1 keeps the cluster" '[ "$rc" -eq 0 ] && [ "$(grep -c "^kind delete cluster" "$calls")" -eq 1 ] && grep -qi "kept" <<<"$out"'

rm -rf "$work/logs"
run STUB_CARGO_RC=3
check "exit status is the first failing suite's" '[ "$rc" -eq 3 ]'
check "collect_logs writes into the log dir on failure" '[ -s "$work/logs/pods.txt" ] && [ -s "$work/logs/describe.txt" ]'
check "cluster deleted after a failing suite" '[ "$(grep -c "^kind delete cluster" "$calls")" -ge 2 ]'

rm -rf "$work/logs"
run
check "no logs on success" '[ ! -e "$work/logs" ]'

run STUB_HELM_RC=1 STUB_PODS="micold-0 0/1 ImagePullBackOff"
check "a failed install exits non-zero" '[ "$rc" -ne 0 ]'
check "ImagePullBackOff diagnostics name the image" 'grep -q "ImagePullBackOff" <<<"$out" && grep -q "micold-orchestrator:e2e" <<<"$out"'
check "a failed install runs no suite" '! grep -q "^cargo test" "$calls"'

run MICOLD_E2E_SUITES=link,ops
check "MICOLD_E2E_SUITES selects the suites, run in the harness's order" \
  '[ "$(grep -c "^cargo test" "$calls")" -eq 2 ] && [ "$(line_of "^cargo test -p micold-e2e --test ops ")" -lt "$(line_of "^cargo test -p micold-e2e --test link ")" ]'
check "a chosen selection runs neither refusal nor the matrix" '! grep -q "corrupt-registry\|testing.breakage\|--test refusal" "$calls"'

run MICOLD_E2E_SUITES=refusal
check "selecting refusal reinstalls with the corrupt registry and runs only it" \
  '[ "$rc" -eq 0 ] && grep -q "^helm upgrade .*ci/corrupt-registry.yaml" "$calls" && [ "$(grep -c "^cargo test" "$calls")" -eq 1 ] && grep -q "^cargo test -p micold-e2e --test refusal" "$calls"'

run MICOLD_E2E_SUITES=ops MICOLD_E2E_MATRIX=1
check "MICOLD_E2E_MATRIX=1 runs the matrix for a chosen selection" '[ "$(grep -c "testing.breakage=" "$calls")" -eq 5 ]'

run MICOLD_E2E_MATRIX=0
check "MICOLD_E2E_MATRIX=0 skips the matrix" '[ "$rc" -eq 0 ] && ! grep -q "testing.breakage" "$calls"'

run STUB_BREAK_CARGO_RC=0
run STUB_FAIL_SUITE=link
check "a breakage caught by the wrong suite fails the run" '[ "$rc" -ne 0 ] && grep -q "NOT caught by its suite: link failed first, expected ops" <<<"$out"'

run
check "the matrix names the suite that caught each breakage" 'grep -q "breakage volume: caught by suite restart" <<<"$out" && grep -q "breakage command: caught by suite ops" <<<"$out"'

run STUB_BREAK_CARGO_RC=0
check "a seeded breakage no suite catches fails the run, naming it" \
  '[ "$rc" -ne 0 ] && grep -q "breakage command: NOT caught" <<<"$out" && grep -q "breakage opsPort: NOT caught" <<<"$out"'

run MICOLD_E2E_SUITES=network
check "a placeholder suite is listed as skipped naming its issue and runs no cargo test" \
  '[ "$rc" -eq 0 ] && grep -q "suite network: skipped, blocked on #1732" <<<"$out" && ! grep -q "^cargo test" "$calls"'
run MICOLD_E2E_SUITES=ui
check "the ui placeholder names #1954" '[ "$rc" -eq 0 ] && grep -q "suite ui: skipped, blocked on #1954" <<<"$out"'
run MICOLD_E2E_SUITES=ops,ui
check "selecting a suite runs only it; a placeholder beside it is skipped" \
  '[ "$(grep -c "^cargo test" "$calls")" -eq 1 ] && grep -q "^cargo test -p micold-e2e --test ops " "$calls" && grep -q "suite ui: skipped" <<<"$out"'

check "every suite the harness lists is a cargo test target with a main.rs" \
  'for s in $(sed -n "s/^KNOWN_SUITES=\"\(.*\)\"/\1/p" "$SCRIPT"); do [ -f "$ROOT/crates/micold-e2e/tests/$s/main.rs" ] && grep -q "path = \"tests/$s/main.rs\"" "$ROOT/crates/micold-e2e/Cargo.toml" || { echo "no target for $s"; exit 1; }; done'

run MICOLD_E2E_SUITES=nosuch
check "an unknown suite is refused before cluster work" '[ "$rc" -eq 2 ] && grep -q nosuch <<<"$out" && ! grep -q "^kind create" "$calls"'

echo
if [ "$failures" -ne 0 ]; then
  echo "e2e-kind: $failures of $cases case(s) failed"
  exit 1
fi
echo "e2e-kind: all $cases cases passed"
