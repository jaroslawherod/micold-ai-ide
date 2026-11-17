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
stub helm 'exit "${STUB_HELM_RC:-0}"'
stub cargo 'echo "MICOLD_E2E=${MICOLD_E2E:-} REQUIRED=${MICOLD_E2E_REQUIRED:-} NS=${MICOLD_E2E_NAMESPACE:-}" >>"'"$calls"'"; exit "${STUB_CARGO_RC:-0}"'
stub kubectl 'case "$*" in *"get pods"*) printf "%s\n" "${STUB_PODS:-micold-0 1/1 Running}" ;; *) echo "kubectl-out $*" ;; esac'
stub uname 'case "$1" in -s) echo "${STUB_OS:-Linux}" ;; -m) echo "${STUB_ARCH:-x86_64}" ;; esac'

# run [env...]: runs the harness with only the stubs and the system basics on PATH.
run() {
  : >"$calls"
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
check "cluster deleted at exit" '[ "$(grep -c "^kind delete cluster --name micold-e2e" "$calls")" -ge 2 ]'

run MICOLD_E2E_CLUSTER=other-e2e
check "a distinct MICOLD_E2E_CLUSTER is honoured" 'grep -q "^kind create cluster --name other-e2e" "$calls" && ! grep -q -- "--name micold-e2e" "$calls"'

run MICOLD_E2E_KEEP=1
check "MICOLD_E2E_KEEP=1 keeps the cluster" '[ "$rc" -eq 0 ] && [ "$(grep -c "^kind delete cluster" "$calls")" -eq 1 ] && grep -qi "kept" <<<"$out"'

rm -rf "$work/logs"
run STUB_CARGO_RC=3
check "exit status is the suite's" '[ "$rc" -eq 3 ]'
check "collect_logs writes into the log dir on failure" '[ -s "$work/logs/pods.txt" ] && [ -s "$work/logs/describe.txt" ]'
check "cluster deleted after a failing suite" '[ "$(grep -c "^kind delete cluster" "$calls")" -ge 2 ]'

rm -rf "$work/logs"
run
check "no logs on success" '[ ! -e "$work/logs" ]'

run STUB_HELM_RC=1 STUB_PODS="micold-0 0/1 ImagePullBackOff"
check "a failed install exits non-zero" '[ "$rc" -ne 0 ]'
check "ImagePullBackOff diagnostics name the image" 'grep -q "ImagePullBackOff" <<<"$out" && grep -q "micold-orchestrator:e2e" <<<"$out"'
check "a failed install runs no suite" '! grep -q "^cargo test" "$calls"'

run MICOLD_E2E_SUITES=ops,link
check "MICOLD_E2E_SUITES selects --test per suite" 'grep -q "^cargo test -p micold-e2e --test ops --test link" "$calls"'

run MICOLD_E2E_SUITES=nosuch
check "an unknown suite is refused before cluster work" '[ "$rc" -eq 2 ] && grep -q nosuch <<<"$out" && ! grep -q "^kind create" "$calls"'

echo
if [ "$failures" -ne 0 ]; then
  echo "e2e-kind: $failures of $cases case(s) failed"
  exit 1
fi
echo "e2e-kind: all $cases cases passed"
