#!/usr/bin/env bash
# The local stack in a kind cluster (#1732 M5, FR-019..FR-023): `mise run local-stack` runs this
# (scripts/local-stack.sh, MICOLD_LOCAL_MODE=kind, the default). Creates or reuses the kind cluster
# micold-local (deploy/kind/local.yaml), builds micold-orchestrator:local (Dockerfile.orchestrator)
# and micold-web:local (Dockerfile.web, for the forwarded API address), `kind load`s both, installs
# the chart with ci/local.yaml (Postgres state, a seeded daemon sidecar, the web client), and
# port-forwards the API and the web client to loopback. Ctrl+C stops the forwards and deletes the
# cluster, unless MICOLD_LOCAL_KEEP=1. A rerun with the cluster present skips `kind create`, still
# builds, loads and upgrades, then restarts the Deployments so the new images run.
#
#   MICOLD_LOCAL_API_PORT  the API's loopback port (8730)
#   MICOLD_LOCAL_WEB_PORT  the web client's loopback port (8731)
#   MICOLD_LOCAL_KEEP=1    keep the cluster at exit
#
# Needs docker, kind, kubectl and helm (scripts/install-kube-tools.sh pins kind, kubectl and helm).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLUSTER=micold-local
CONTEXT="kind-$CLUSTER"
NS=micold
RELEASE=micold
CHART="$ROOT/deploy/helm/micold"
API_PORT="${MICOLD_LOCAL_API_PORT:-8730}"
WEB_PORT="${MICOLD_LOCAL_WEB_PORT:-8731}"

say() { echo "local-stack: $*"; }

for tool in docker kind kubectl helm; do
  command -v "$tool" >/dev/null 2>&1 || {
    say "$tool is not installed (kind mode needs docker, kind, kubectl and helm; scripts/install-kube-tools.sh installs the last three). MICOLD_LOCAL_MODE=process runs the stack as local processes instead." >&2
    exit 1
  }
done

forwards=()
stop() {
  trap - INT TERM EXIT
  local pid
  for pid in "${forwards[@]}"; do kill "$pid" 2>/dev/null || true; done
  for pid in "${forwards[@]}"; do wait "$pid" 2>/dev/null || true; done
  if [ "${MICOLD_LOCAL_KEEP:-0}" = 1 ]; then
    say "kept cluster $CLUSTER (kind delete cluster --name $CLUSTER removes it)"
  else
    kind delete cluster --name "$CLUSTER" || true
  fi
  say "stopped"
}
trap 'stop; exit 130' INT TERM
trap stop EXIT

reuse=0
if kind get clusters | grep -qx "$CLUSTER"; then
  reuse=1
  say "reusing cluster $CLUSTER"
else
  kind create cluster --name "$CLUSTER" --config "$ROOT/deploy/kind/local.yaml"
fi

docker build -f "$ROOT/Dockerfile.orchestrator" -t micold-orchestrator:local "$ROOT"
docker build -f "$ROOT/Dockerfile.web" --build-arg "EXPO_PUBLIC_ORCHESTRATOR_URL=ws://127.0.0.1:$API_PORT/api/ws" -t micold-web:local "$ROOT"
kind load docker-image --name "$CLUSTER" micold-orchestrator:local
kind load docker-image --name "$CLUSTER" micold-web:local

helm upgrade --install "$RELEASE" "$CHART" --kube-context "$CONTEXT" --namespace "$NS" --create-namespace \
  -f "$CHART/ci/local.yaml" --wait --timeout 10m
if [ "$reuse" = 1 ]; then
  # Same tags as before: only a restart makes the pods run the images just loaded.
  kubectl --context "$CONTEXT" -n "$NS" rollout restart deployment
  kubectl --context "$CONTEXT" -n "$NS" rollout status deployment --timeout 5m
fi

# The API listens on the pod's loopback only (no TLS until #1733): a port-forward reaches it there.
kubectl --context "$CONTEXT" -n "$NS" port-forward "deployment/$RELEASE-orchestrator" "$API_PORT:8730" &
forwards+=("$!")
kubectl --context "$CONTEXT" -n "$NS" port-forward "service/$RELEASE-web" "$WEB_PORT:8731" &
forwards+=("$!")

say "orchestrator ws://127.0.0.1:$API_PORT/api/ws"
say "web client    http://127.0.0.1:$WEB_PORT  (Ctrl+C stops the forwards and deletes the cluster unless MICOLD_LOCAL_KEEP=1)"
wait -n || true
say "a port-forward exited; stopping"
