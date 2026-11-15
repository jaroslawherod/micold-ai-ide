#!/usr/bin/env bash
# Check the Helm chart deploy/helm/micold with no cluster (#1981, FR-005, SC-004).
#
#   chart-check.sh            helm lint and kubeconform over the default values and every
#                             ci/*.yaml, then scripts/tests/chart-conformance.test.sh
#   chart-check.sh --image    build Dockerfile.orchestrator and smoke-run it as UID 65532 on a
#                             read-only root (FR-001); needs docker
#
# Needs `helm` and `kubeconform` on PATH (scripts/install-kube-tools.sh pins both). kubeconform
# fetches its Kubernetes schemas from the network. CHART overrides the chart directory,
# KUBECONFORM_K8S_VERSION the Kubernetes version validated against (default the kubectl pin's
# 1.34.0), CHART_CHECK_CONFORMANCE=0 skips the conformance test (the script's own shell test does,
# on fixture charts). `mise run chart-check` runs this; scripts/gate.sh runs it when both tools
# exist; CI runs it in the `kube-chart` job.

set -euo pipefail

ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

image_smoke() {
  local tag="${MICOLD_ORCHESTRATOR_IMAGE:-ghcr.io/jaroslawherod/micold-orchestrator:dev}" id
  command -v docker >/dev/null 2>&1 || { echo "chart-check: --image needs docker" >&2; exit 1; }
  [ -f "$ROOT/Dockerfile.orchestrator" ] || { echo "chart-check: no $ROOT/Dockerfile.orchestrator" >&2; exit 1; }
  echo "== docker build -f Dockerfile.orchestrator -t $tag"
  docker build -f "$ROOT/Dockerfile.orchestrator" -t "$tag" "$ROOT"

  local user
  user="$(docker image inspect -f '{{.Config.User}}' "$tag")"
  [ "$user" = 65532 ] || [ "$user" = 65532:65532 ] || { echo "chart-check: image user is '$user', want 65532" >&2; exit 1; }
  echo "ok    image user is $user"

  # The entrypoint is `micold-daemon orchestrator`: an unknown flag is refused by name.
  local out rc=0
  out="$(docker run --rm --read-only --user 65532:65532 "$tag" --no-such-flag 2>&1)" || rc=$?
  if [ "$rc" -eq 0 ] || ! grep -q 'unknown orchestrator argument "--no-such-flag"' <<<"$out"; then
    echo "chart-check: the entrypoint did not refuse an unknown orchestrator flag (exit $rc): $out" >&2
    exit 1
  fi
  echo "ok    entrypoint is micold-daemon orchestrator"

  # As the chart runs it: read-only root, the data and socket paths on writable mounts only.
  id="$(docker run -d --read-only --user 65532:65532 --cap-drop ALL --security-opt no-new-privileges \
    --tmpfs /var/lib/micold:uid=65532,gid=65532 --tmpfs /run/micold:uid=65532,gid=65532 --tmpfs /tmp \
    "$tag" --data-dir=/var/lib/micold --socket=/run/micold/orchestrator.sock)"
  trap 'docker rm -f "$id" >/dev/null 2>&1 || true' EXIT
  sleep 3
  local running
  running="$(docker inspect -f '{{.State.Running}}' "$id" || echo unknown)"
  if [ "$running" != true ]; then
    docker logs "$id" >&2 || true
    echo "chart-check: the orchestrator exited instead of serving as UID 65532 on a read-only root" >&2
    exit 1
  fi
  echo "ok    orchestrator runs as 65532 on a read-only root with --data-dir and --socket"
}

if [ "${1:-}" = --image ]; then
  [ "$#" -eq 1 ] || { echo "usage: $0 [--image]" >&2; exit 2; }
  image_smoke
  exit 0
fi
[ "$#" -eq 0 ] || { echo "usage: $0 [--image]" >&2; exit 2; }

CHART="${CHART:-$ROOT/deploy/helm/micold}"
K8S_VERSION="${KUBECONFORM_K8S_VERSION:-1.34.0}"
for tool in helm kubeconform; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "chart-check: $tool is not on PATH (scripts/install-kube-tools.sh $tool)" >&2
    exit 1
  }
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# One run per values set: the defaults (unless ci/default.yaml stands for them), then each ci/*.yaml.
sets=()
[ -e "$CHART/ci/default.yaml" ] || sets=(default)
for f in "$CHART"/ci/*.yaml; do
  [ -e "$f" ] && sets+=("ci/$(basename "$f")")
done

failed=0
for set in "${sets[@]}"; do
  args=()
  [ "$set" = default ] || args=(-f "$CHART/$set")
  echo "== $set"
  if ! helm lint --strict "$CHART" "${args[@]}"; then
    echo "chart-check: helm lint failed with $set" >&2
    failed=1
    continue
  fi
  if ! helm template micold "$CHART" --namespace micold "${args[@]}" >"$work/rendered.yaml"; then
    echo "chart-check: helm template failed with $set" >&2
    failed=1
    continue
  fi
  if ! kubeconform -strict -summary -kubernetes-version "$K8S_VERSION" "$work/rendered.yaml"; then
    echo "chart-check: kubeconform rejected the manifests rendered with $set" >&2
    failed=1
  fi
done
[ "$failed" = 0 ] || exit 1

if [ "${CHART_CHECK_CONFORMANCE:-1}" != 0 ]; then
  echo "== conformance"
  MICOLD_CHART_REQUIRED=1 CHART="$CHART" "$ROOT/scripts/tests/chart-conformance.test.sh"
fi
echo "chart-check: ok (${#sets[@]} values set(s))"
