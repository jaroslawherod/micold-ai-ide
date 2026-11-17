#!/usr/bin/env bash
# Run the orchestrator end-to-end suites on a throwaway kind cluster (#1981/#1984, FR-008, FR-013,
# FR-014). `mise run e2e-kind` and CI's `kube-e2e` job run this. Needs docker, kind, kubectl, helm
# and cargo on PATH (scripts/install-kube-tools.sh pins kind, kubectl and helm). Linux amd64 only:
# anywhere else it prints a skip line and exits 0.
#
# Flow: delete-then-create the cluster, build Dockerfile.orchestrator, `kind load` it, install the
# chart with ci/e2e.yaml and --wait, then `cargo test -p micold-e2e --test <suite>...` with
# MICOLD_E2E=1 MICOLD_E2E_REQUIRED=1. On failure the pods, describe output, events and container
# logs go to the log dir; the cluster is deleted at exit unless MICOLD_E2E_KEEP=1.
# Environment: specs/1981-kube-e2e/contracts/harness-env.md, docs/development/kube-e2e.md.
#
#   MICOLD_E2E_CLUSTER    kind cluster name (micold-e2e)
#   MICOLD_E2E_NAMESPACE  namespace of the release (micold-e2e)
#   MICOLD_E2E_SUITES     comma list of suites (every directory under crates/micold-e2e/tests)
#   MICOLD_E2E_KEEP=1     keep the cluster afterwards
#   MICOLD_E2E_LOG_DIR    failure logs (target/e2e-logs/<cluster>)
# Exit status is the suites'.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHART="$ROOT/deploy/helm/micold"
KNOWN_SUITES="ops link restart refusal network ui"
IMAGE_REPO=ghcr.io/jaroslawherod/micold-orchestrator
IMAGE_TAG=e2e
IMAGE="$IMAGE_REPO:$IMAGE_TAG"

say() { echo "e2e-kind: $*"; }

if [ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]; then
  say "skip: kind e2e runs on Linux amd64 only, this is $(uname -s)-$(uname -m)"
  exit 0
fi

for tool in docker kind kubectl helm cargo; do
  command -v "$tool" >/dev/null 2>&1 || { say "$tool is not installed (needed: docker kind kubectl helm cargo)" >&2; exit 1; }
done

cluster="${MICOLD_E2E_CLUSTER:-micold-e2e}"
ns="${MICOLD_E2E_NAMESPACE:-micold-e2e}"
log_dir="${MICOLD_E2E_LOG_DIR:-$ROOT/target/e2e-logs/$cluster}"

if [ -n "${MICOLD_E2E_SUITES:-}" ]; then
  IFS=, read -r -a suites <<<"$MICOLD_E2E_SUITES"
else
  suites=()
  for d in "$ROOT"/crates/micold-e2e/tests/*/; do
    [ -f "$d/main.rs" ] && suites+=("$(basename "$d")")
  done
fi
test_args=()
for s in "${suites[@]}"; do
  case " $KNOWN_SUITES " in
    *" $s "*) test_args+=(--test "$s") ;;
    *) say "unknown suite '$s' (known: $KNOWN_SUITES)" >&2; exit 2 ;;
  esac
done
[ "${#test_args[@]}" -gt 0 ] || { say "no suite to run" >&2; exit 2; }

collect_logs() {
  mkdir -p "$log_dir"
  kubectl --context "kind-$cluster" -n "$ns" get pods -o wide >"$log_dir/pods.txt" 2>&1 || true
  kubectl --context "kind-$cluster" -n "$ns" describe pods >"$log_dir/describe.txt" 2>&1 || true
  kubectl --context "kind-$cluster" -n "$ns" get events --sort-by=.lastTimestamp >"$log_dir/events.txt" 2>&1 || true
  kubectl --context "kind-$cluster" -n "$ns" logs --all-containers --prefix --tail=500 \
    -l app.kubernetes.io/instance=micold >"$log_dir/logs.txt" 2>&1 || true
  say "failure logs in $log_dir"
}

cleanup() {
  local rc=$?
  [ "$rc" -eq 0 ] || collect_logs
  if [ "${MICOLD_E2E_KEEP:-}" = 1 ]; then
    say "cluster $cluster kept (MICOLD_E2E_KEEP=1); delete it with: kind delete cluster --name $cluster"
  else
    kind delete cluster --name "$cluster" >/dev/null 2>&1 || true
  fi
  exit "$rc"
}

# Delete first: a cluster left by an earlier run under this name is replaced, never doubled.
kind delete cluster --name "$cluster" >/dev/null 2>&1 || true
trap cleanup EXIT
say "creating kind cluster $cluster"
kind create cluster --name "$cluster" --wait 120s

say "building $IMAGE"
"$ROOT/scripts/build-lock.sh" docker build -f "$ROOT/Dockerfile.orchestrator" -t "$IMAGE" "$ROOT"
kind load docker-image "$IMAGE" --name "$cluster"

say "installing the chart into namespace $ns"
if ! helm upgrade --install micold "$CHART" -n "$ns" --create-namespace --kube-context "kind-$cluster" \
  -f "$CHART/ci/e2e.yaml" --set image.tag="$IMAGE_TAG" --wait --timeout 180s; then
  say "the release did not become ready; pods:" >&2
  pods="$(kubectl --context "kind-$cluster" -n "$ns" get pods -o wide 2>&1 || true)"
  echo "$pods" >&2
  if grep -qE 'ImagePullBackOff|ErrImagePull|ErrImageNeverPull' <<<"$pods"; then
    say "ImagePullBackOff: the node could not pull $IMAGE; it should have come from 'kind load docker-image $IMAGE --name $cluster'" >&2
  fi
  kubectl --context "kind-$cluster" -n "$ns" describe pods >&2 || true
  kubectl --context "kind-$cluster" -n "$ns" logs --all-containers --prefix --tail=50 \
    -l app.kubernetes.io/instance=micold >&2 || true
  exit 1
fi

say "running suites: ${suites[*]}"
rc=0
MICOLD_E2E=1 MICOLD_E2E_REQUIRED=1 MICOLD_E2E_NAMESPACE="$ns" MICOLD_E2E_CONTEXT="kind-$cluster" \
  "$ROOT/scripts/build-lock.sh" cargo test -p micold-e2e "${test_args[@]}" -- --test-threads=1 || rc=$?
exit "$rc"
