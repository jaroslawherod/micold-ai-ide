#!/usr/bin/env bash
# Drives scripts/chart-check.sh (#1981, FR-005, SC-004) over a chart broken two ways: a template
# helm cannot render, and a manifest Kubernetes would reject. Each must fail the check and name the
# values file it failed with. Skips with the reason without `helm` and `kubeconform` on PATH
# (scripts/install-kube-tools.sh installs both); the `kube-chart` CI job runs it with them.

set -uo pipefail

ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/chart-check.sh"

for tool in helm kubeconform; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "chart-check.test: SKIP: $tool is not on PATH (scripts/install-kube-tools.sh installs it)"
    exit 0
  fi
done

failures=0
cases=0
ok() { cases=$((cases + 1)); printf 'ok    %s\n' "$1"; }
fail() { cases=$((cases + 1)); failures=$((failures + 1)); printf 'FAIL  %s\n' "$1"; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# new_chart <dir> <deployment.yaml body>: a one-template chart with a ci/ values file.
new_chart() {
  mkdir -p "$1/templates" "$1/ci"
  printf 'apiVersion: v2\nname: fixture\nversion: 0.1.0\nappVersion: "1"\n' >"$1/Chart.yaml"
  printf 'replicas: 1\n' >"$1/values.yaml"
  printf '{}\n' >"$1/ci/default.yaml"
  printf 'replicas: 2\n' >"$1/ci/two.yaml"
  printf '%s\n' "$2" >"$1/templates/deployment.yaml"
}

deployment='apiVersion: apps/v1
kind: Deployment
metadata:
  name: fixture
spec:
  replicas: {{ .Values.replicas }}
  selector:
    matchLabels: {app: fixture}
  template:
    metadata:
      labels: {app: fixture}
    spec:
      containers:
        - name: c
          image: example.test/c:1'

# A good chart passes (conformance is the real chart's, so it is switched off here).
new_chart "$work/good" "$deployment"
out="$(CHART="$work/good" CHART_CHECK_CONFORMANCE=0 "$SCRIPT" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ]; then ok "a valid chart passes"; else fail "a valid chart passes (exit $rc: $out)"; fi

# No ci/default.yaml: the defaults would go unchecked, so the check refuses.
new_chart "$work/nodefault" "$deployment"
rm "$work/nodefault/ci/default.yaml"
out="$(CHART="$work/nodefault" CHART_CHECK_CONFORMANCE=0 "$SCRIPT" 2>&1)"
rc=$?
if [ "$rc" -ne 0 ] && grep -q 'ci/default.yaml' <<<"$out"; then ok "a chart without ci/default.yaml is refused"; else fail "a chart without ci/default.yaml is refused (exit $rc: $out)"; fi

# A template error: helm lint fails.
new_chart "$work/syntax" "${deployment/\{\{ .Values.replicas \}\}/\{\{ .Values.replicas }"
out="$(CHART="$work/syntax" CHART_CHECK_CONFORMANCE=0 "$SCRIPT" 2>&1)"
rc=$?
if [ "$rc" -ne 0 ] && grep -q 'helm lint' <<<"$out"; then ok "a template error fails at helm lint"; else fail "a template error fails at helm lint (exit $rc: $out)"; fi

# A schema error only for ci/two.yaml: replicas rendered as a string there.
new_chart "$work/schema" "${deployment/\{\{ .Values.replicas \}\}/\{\{ if eq (int .Values.replicas) 2 \}\}\"two\"\{\{ else \}\}1\{\{ end \}\}}"
out="$(CHART="$work/schema" CHART_CHECK_CONFORMANCE=0 "$SCRIPT" 2>&1)"
rc=$?
if [ "$rc" -ne 0 ] && grep -q 'kubeconform' <<<"$out" && grep -q 'ci/two.yaml' <<<"$out"; then
  ok "an invalid manifest fails at kubeconform, naming the values file"
else
  fail "an invalid manifest fails at kubeconform, naming the values file (exit $rc: $out)"
fi

echo
if [ "$failures" -ne 0 ]; then
  echo "chart-check: $failures of $cases case(s) failed"
  exit 1
fi
echo "chart-check: all $cases cases passed"
