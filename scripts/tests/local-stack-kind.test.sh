#!/usr/bin/env bash
# scripts/local-stack.sh in kind mode (#1732 M5, FR-019, FR-023, FR-025) with stub docker, kind,
# kubectl and helm on PATH recording their argv: mode selection, the missing-tool refusal, cluster
# create or reuse, both images built and loaded, the chart installed with ci/local.yaml, both
# port-forwards, and teardown on TERM (cluster deleted unless MICOLD_LOCAL_KEEP=1). No cluster is
# needed; the live run is CI's `local-stack-kind` job.

set -uo pipefail
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/local-stack.sh"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

tmp="$(mktemp -d)"
trap 'pkill -f "$tmp/bin" 2>/dev/null || true; rm -rf "$tmp"' EXIT
mkdir "$tmp/bin"
for tool in docker kind kubectl helm; do
  cat >"$tmp/bin/$tool" <<STUB
#!/usr/bin/env bash
echo "$tool \$*" >>"\$STUB_LOG"
case "$tool \$*" in
  "kind get clusters") printf '%s\n' \${STUB_CLUSTERS:-} ;;
  kubectl*port-forward*) exec sleep 300 ;;
esac
exit 0
STUB
  chmod +x "$tmp/bin/$tool"
done
# PATH with the stubs and the base tools only, so a real kind on the host is never reached.
base="$tmp/base"
mkdir "$base"
for t in bash env sleep cat grep mkdir mktemp rm kill dirname git setsid printf pkill timeout; do
  p="$(command -v "$t")" && ln -s "$p" "$base/$t"
done

# run_stack <log> <env...>: start the stack in its own session, wait for both port-forwards, TERM it.
run_stack() {
  local log="$1"; shift
  : >"$log"
  env -i PATH="$tmp/bin:$base" HOME="$tmp" STUB_LOG="$log" "$@" setsid "$SCRIPT" >"$log.out" 2>&1 &
  local pid=$! i
  for i in $(seq 1 100); do
    [ "$(grep -c 'port-forward' "$log")" -ge 2 ] && break
    sleep 0.1
  done
  kill -TERM "$pid" 2>/dev/null
  for i in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
  kill -0 "$pid" 2>/dev/null && { fail "the stack did not stop on TERM"; kill -KILL "$pid"; }
  wait "$pid" 2>/dev/null
  true
}
has() { grep -qF -- "$2" "$1"; }

# Fresh cluster, default mode is kind.
run_stack "$tmp/fresh"
l="$tmp/fresh"
has "$l" "kind create cluster --name micold-local --config $ROOT/deploy/kind/local.yaml" && pass "default mode is kind and creates micold-local" || fail "no kind create: $(cat "$l" "$l.out")"
has "$l" "docker build -f $ROOT/Dockerfile.orchestrator -t micold-orchestrator:local $ROOT" && pass "orchestrator image built" || fail "orchestrator image not built"
has "$l" "docker build -f $ROOT/Dockerfile.web --build-arg EXPO_PUBLIC_ORCHESTRATOR_URL=ws://127.0.0.1:8730/api/ws -t micold-web:local $ROOT" && pass "web image built with the forwarded URL" || fail "web image not built"
has "$l" "docker save -o " && [ "$(grep -c '^kind load image-archive --name micold-local ' "$l")" = 2 ] && pass "both images loaded" || fail "images not loaded"
has "$l" "helm upgrade --install micold $ROOT/deploy/helm/micold --kube-context kind-micold-local --namespace micold --create-namespace -f $ROOT/deploy/helm/micold/ci/local.yaml --wait --timeout 10m" && pass "chart installed with ci/local.yaml" || fail "helm: $(grep helm "$l")"
has "$l" "kubectl --context kind-micold-local -n micold port-forward deployment/micold-orchestrator 8730:8730" && pass "API forwarded to 8730" || fail "no API forward"
has "$l" "kubectl --context kind-micold-local -n micold port-forward service/micold-web 8731:8731" && pass "web forwarded to 8731" || fail "no web forward"
has "$l" "rollout restart" && fail "a fresh cluster was restarted" || pass "a fresh cluster is not restarted"
has "$l" "kind delete cluster --name micold-local" && pass "TERM deletes the cluster" || fail "cluster not deleted"

# Rerun with the cluster present and KEEP: no create, still build, load, upgrade, restart; kept.
run_stack "$tmp/reuse" STUB_CLUSTERS=micold-local MICOLD_LOCAL_KEEP=1
l="$tmp/reuse"
has "$l" "kind create cluster" && fail "reuse created a cluster" || pass "reuse skips kind create"
[ "$(grep -c '^docker build' "$l")" = 2 ] && [ "$(grep -c '^kind load image-archive' "$l")" = 2 ] && pass "reuse rebuilds and reloads both images" || fail "reuse build/load: $(cat "$l")"
has "$l" "helm upgrade --install micold" && pass "reuse upgrades the release" || fail "reuse no helm upgrade"
has "$l" "kubectl --context kind-micold-local -n micold rollout restart deployment" && pass "reuse restarts the deployments" || fail "reuse no restart"
has "$l" "kind delete cluster" && fail "KEEP still deleted the cluster" || pass "KEEP keeps the cluster"

# Ports overridable.
run_stack "$tmp/ports" MICOLD_LOCAL_API_PORT=9730 MICOLD_LOCAL_WEB_PORT=9731
has "$tmp/ports" "port-forward deployment/micold-orchestrator 9730:8730" && has "$tmp/ports" "EXPO_PUBLIC_ORCHESTRATOR_URL=ws://127.0.0.1:9730/api/ws" && has "$tmp/ports" "port-forward service/micold-web 9731:8731" && pass "port overrides reach the build and the forwards" || fail "port overrides"

# A missing tool: named, with the process hint, non-zero, nothing created.
rm "$tmp/bin/kind"
: >"$tmp/missing"
if env -i PATH="$tmp/bin:$base" HOME="$tmp" STUB_LOG="$tmp/missing" "$SCRIPT" >"$tmp/missing.out" 2>&1; then
  fail "a missing kind exited 0"
elif grep -q "kind is not installed" "$tmp/missing.out" && grep -q "MICOLD_LOCAL_MODE=process" "$tmp/missing.out" && [ ! -s "$tmp/missing" ]; then
  pass "a missing tool is named with the process hint and nothing runs"
else fail "missing tool: $(cat "$tmp/missing.out" "$tmp/missing")"; fi

# An unknown mode is refused.
if env -i PATH="$tmp/bin:$base" HOME="$tmp" MICOLD_LOCAL_MODE=bogus "$SCRIPT" >"$tmp/bogus.out" 2>&1; then fail "an unknown mode ran"
elif grep -q "MICOLD_LOCAL_MODE=bogus" "$tmp/bogus.out"; then pass "an unknown mode is refused by name"; else fail "bogus: $(cat "$tmp/bogus.out")"; fi

exit "$failures"
