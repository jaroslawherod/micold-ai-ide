#!/usr/bin/env bash
# The live check of the kind-mode local stack (#1732 M5, T052, SC-008, US5 scenarios 1, 2, 5), run
# by CI's `local-stack-kind` job on a docker host (kind cannot run in a cloud session):
#   1. `local-stack.sh` with MICOLD_LOCAL_KEEP=1 in the background; probe (120 s after helm returns)
#   2. delete the orchestrator pod; probe again (state survives in Postgres)
#   3. TERM it (cluster kept); rerun in the background, still KEEP, to exercise reuse; probe; TERM
#   4. a last run without KEEP; probe; TERM; the cluster must be gone
# Prints each phase's duration, for the ledger.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CTX=kind-micold-local
say() { echo "local-stack-kind-live: $*"; }
stack=""
LOG="$(mktemp)"
started=$SECONDS

# up [env...]: start the stack in its own session and wait for helm to return (the forwards start).
up() {
  : >"$LOG"
  env "$@" setsid "$ROOT/scripts/local-stack.sh" >"$LOG" 2>&1 &
  stack=$!
  local t=$SECONDS
  until grep -q "local-stack: orchestrator ws://" "$LOG"; do
    kill -0 "$stack" 2>/dev/null || { cat "$LOG"; say "the stack exited"; exit 1; }
    sleep 2
  done
  say "up in $((SECONDS - t)) s"
}
down() {
  kill -TERM "-$stack" 2>/dev/null || kill -TERM "$stack"
  wait "$stack" 2>/dev/null || true
}
probe() { local t=$SECONDS; MICOLD_PROBE_TIMEOUT=120 "$ROOT/scripts/local-stack-probe.sh"; say "$1 probe in $((SECONDS - t)) s"; }
trap 'down 2>/dev/null; kind delete cluster --name micold-local >/dev/null 2>&1 || true; rm -f "$LOG"' EXIT

up MICOLD_LOCAL_KEEP=1
probe first
kubectl --context "$CTX" -n micold delete pod -l app.kubernetes.io/component=orchestrator --wait=true
# The port-forward dies with the pod: restart the stack's forwards by rerunning (reuse) below; probe
# the restarted pod through a fresh forward first.
kubectl --context "$CTX" -n micold rollout status deployment/micold-orchestrator --timeout 5m
kubectl --context "$CTX" -n micold port-forward deployment/micold-orchestrator 18730:8730 >/dev/null 2>&1 &
fwd=$!
MICOLD_LOCAL_API_PORT=18730 probe "after restart"
kill "$fwd"
down
kind get clusters | grep -qx micold-local || { say "KEEP did not keep the cluster"; exit 1; }

up MICOLD_LOCAL_KEEP=1
grep -q "reusing cluster micold-local" "$LOG" || { say "the rerun did not reuse the cluster"; exit 1; }
probe reuse
down

up
probe last
down
if kind get clusters | grep -qx micold-local; then say "TERM without KEEP left the cluster"; exit 1; fi
say "ok in $((SECONDS - started)) s"
