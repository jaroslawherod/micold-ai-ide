#!/usr/bin/env bash
# Run the orchestrator end-to-end suites on a throwaway kind cluster (#1981/#1984, FR-008, FR-013,
# FR-014). `mise run e2e-kind` and CI's `kube-e2e` job run this. Needs docker or podman
# (MICOLD_CONTAINER_RUNTIME picks; scripts/container-runtime.sh), kind, kubectl, helm and cargo on PATH (scripts/install-kube-tools.sh pins kind, kubectl and helm). Linux amd64 only:
# anywhere else it prints a skip line and exits 0.
#
# Flow: delete-then-create the cluster, build Dockerfile.orchestrator, `kind load` it, install the
# chart with ci/e2e.yaml and --wait, then one `cargo test -p micold-e2e --test <suite>` per suite
# with MICOLD_E2E=1 MICOLD_E2E_REQUIRED=1, in KNOWN_SUITES order (restart before link: it expects an
# empty registry). Two steps follow in the same cluster (#1985):
#   refusal  after the other suites: `helm upgrade` with ci/corrupt-registry.yaml (no --wait: the pod
#            must never be Ready), then `--test refusal`.
#   matrix   SC-003, with the default selection or MICOLD_E2E_MATRIX=1: for each seeded breakage
#            (command user volume dataDir opsPort) uninstall the release (this deletes its PVC, so a
#            corrupt registry.json cannot leak), install it with `--set testing.breakage=<v>` and
#            run ops, restart and link until one fails; it must be the suite meant to catch it (volume: restart, the rest: ops). Otherwise the run fails.
# On failure the pods, describe output, events and container logs go to the log dir; the cluster
# is deleted at exit unless MICOLD_E2E_KEEP=1.
# Environment: specs/1981-kube-e2e/contracts/harness-env.md, docs/development/kube-e2e.md.
#
#   MICOLD_E2E_CLUSTER    kind cluster name (micold-e2e)
#   MICOLD_E2E_NAMESPACE  namespace of the release (micold-e2e)
#   MICOLD_E2E_SUITES     comma list of suites (every directory under crates/micold-e2e/tests)
#   MICOLD_E2E_MATRIX     1 runs the seeded-breakage matrix with a chosen selection, 0 skips it
#   MICOLD_E2E_KEEP=1     keep the cluster afterwards
#   MICOLD_E2E_LOG_DIR    failure logs (target/e2e-logs/<cluster>)
# Exit status is the suites'.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHART="$ROOT/deploy/helm/micold"
KNOWN_SUITES="ops restart link refusal network ui"
# Placeholder suites: name:blocking issue. Listed as skipped, no cargo test run (FR-013).
PLACEHOLDERS="network:#1732 ui:#1954"
BREAKAGES="command user volume dataDir opsPort"
IMAGE_REPO=ghcr.io/jaroslawherod/micold-orchestrator
IMAGE_TAG=e2e
IMAGE="$IMAGE_REPO:$IMAGE_TAG"

say() { echo "e2e-kind: $*"; }

if [ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]; then
  say "skip: kind e2e runs on Linux amd64 only, this is $(uname -s)-$(uname -m)"
  exit 0
fi

# shellcheck source=container-runtime.sh
. "$ROOT/scripts/container-runtime.sh"
detect_container_runtime || { say "needed: docker or podman, kind, kubectl, helm, cargo" >&2; exit 1; }
for tool in kind kubectl helm cargo; do
  command -v "$tool" >/dev/null 2>&1 || { say "$tool is not installed (needed: docker or podman, kind, kubectl, helm, cargo)" >&2; exit 1; }
done

cluster="${MICOLD_E2E_CLUSTER:-micold-e2e}"
ns="${MICOLD_E2E_NAMESPACE:-micold-e2e}"
log_dir="${MICOLD_E2E_LOG_DIR:-$ROOT/target/e2e-logs/$cluster}"

if [ -n "${MICOLD_E2E_SUITES:-}" ]; then
  IFS=, read -r -a chosen <<<"$MICOLD_E2E_SUITES"
  matrix="${MICOLD_E2E_MATRIX:-0}"
else
  chosen=()
  for d in "$ROOT"/crates/micold-e2e/tests/*/; do
    [ -f "$d/main.rs" ] && chosen+=("$(basename "$d")")
  done
  matrix="${MICOLD_E2E_MATRIX:-1}"
fi
for s in "${chosen[@]}"; do
  case " $KNOWN_SUITES " in
    *" $s "*) ;;
    *) say "unknown suite '$s' (known: $KNOWN_SUITES)" >&2; exit 2 ;;
  esac
done
# Run in KNOWN_SUITES order whatever the order given.
suites=()
for k in $KNOWN_SUITES; do
  case " ${chosen[*]} " in *" $k "*) suites+=("$k") ;; esac
done
[ "${#suites[@]}" -gt 0 ] || { say "no suite to run" >&2; exit 2; }

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
"$ROOT/scripts/build-lock.sh" "$CONTAINER_RT" build -f "$ROOT/Dockerfile.orchestrator" -t "$IMAGE" "$ROOT"
kind_load_image "$cluster" "$IMAGE"

# install_chart <wait|nowait> <timeout> [helm args...]: the e2e values, then the extra arguments.
install_chart() {
  local mode="$1" timeout="$2"
  shift 2
  local wait_args=()
  [ "$mode" = wait ] && wait_args=(--wait)
  helm upgrade --install micold "$CHART" -n "$ns" --create-namespace --kube-context "kind-$cluster" \
    -f "$CHART/ci/e2e.yaml" --set image.tag="$IMAGE_TAG" "$@" ${wait_args[@]+"${wait_args[@]}"} --timeout "$timeout"
}

# run_suite <suite>: the suite's test target against the current release.
run_suite() {
  for p in $PLACEHOLDERS; do
    [ "${p%%:*}" = "$1" ] && { say "suite $1: skipped, blocked on ${p#*:}"; return 0; }
  done
  say "suite $1"
  MICOLD_E2E=1 MICOLD_E2E_REQUIRED=1 MICOLD_E2E_NAMESPACE="$ns" MICOLD_E2E_CONTEXT="kind-$cluster" \
    "$ROOT/scripts/build-lock.sh" cargo test -p micold-e2e --test "$1" -- --test-threads=1
}

say "installing the chart into namespace $ns"
if ! install_chart wait 180s; then
  say "the release did not become ready; pods:" >&2
  pods="$(kubectl --context "kind-$cluster" -n "$ns" get pods -o wide 2>&1 || true)"
  echo "$pods" >&2
  if grep -qE 'ImagePullBackOff|ErrImagePull|ErrImageNeverPull' <<<"$pods"; then
    say "ImagePullBackOff: the node could not pull $IMAGE; it should have been loaded with 'kind load image-archive' (scripts/container-runtime.sh)" >&2
  fi
  kubectl --context "kind-$cluster" -n "$ns" describe pods >&2 || true
  kubectl --context "kind-$cluster" -n "$ns" logs --all-containers --prefix --tail=50 \
    -l app.kubernetes.io/instance=micold >&2 || true
  exit 1
fi

say "running suites: ${suites[*]}"
rc=0
# note_rc <status>: the first failure is the run's exit status; the later steps still run, so one
# CI round reports everything.
note_rc() { [ "$rc" -ne 0 ] || rc=$1; }

for s in "${suites[@]}"; do
  [ "$s" = refusal ] && continue
  run_suite "$s" || note_rc $?
done

case " ${suites[*]} " in
  *" refusal "*)
    say "reinstalling with ci/corrupt-registry.yaml: the pod must never become Ready"
    install_chart nowait 60s -f "$CHART/ci/corrupt-registry.yaml" || note_rc $?
    run_suite refusal || note_rc $?
    ;;
esac

if [ "$matrix" = 1 ]; then
  say "seeded-breakage matrix: $BREAKAGES"
  for breakage in $BREAKAGES; do
    # Uninstall removes the release's PVC; deleting every claim left in the namespace as well keeps
    # a corrupt registry.json from reaching the next run.
    helm uninstall micold -n "$ns" --kube-context "kind-$cluster" --wait >/dev/null 2>&1 || true
    kubectl --context "kind-$cluster" -n "$ns" delete pvc --all --wait=true >/dev/null 2>&1 || true
    say "breakage $breakage: install; the suites must fail"
    install_chart wait 60s --set "testing.breakage=$breakage" || say "breakage $breakage: not Ready (expected for most)"
    # Each breakage has the suite that must catch it: the first suite to fail has to be that one,
    # so an infrastructure failure elsewhere cannot pass for a catch.
    case "$breakage" in volume) expected=restart ;; *) expected=ops ;; esac
    failed=""
    for s in ops restart link; do
      run_suite "$s" || { failed="$s"; break; }
    done
    if [ "$failed" = "$expected" ]; then
      say "breakage $breakage: caught by suite $failed"
    elif [ -n "$failed" ]; then
      say "breakage $breakage: NOT caught by its suite: $failed failed first, expected $expected" >&2
      note_rc 1
    else
      say "breakage $breakage: NOT caught: ops, restart and link all passed against a deliberately wrong chart" >&2
      note_rc 1
    fi
  done
fi
exit "$rc"
