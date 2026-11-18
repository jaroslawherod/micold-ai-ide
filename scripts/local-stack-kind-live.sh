#!/usr/bin/env bash
# The live check of the kind-mode local stack (#1732 M5, T052, SC-008, US5 scenarios 1, 2, 5), run
# by CI's `local-stack-kind` job on a docker host (kind cannot run in a cloud session):
#   1. `local-stack.sh` with MICOLD_LOCAL_KEEP=1 in the background; probe (120 s after helm returns)
#   2. write a marker row in Postgres; delete the orchestrator pod (its port-forward dies, so the
#      stack must exit; the cluster is kept), then the Postgres pod; the marker and the registered
#      daemons must survive; probe through fresh forwards on 18730/18731
#   3. rerun in the background, still KEEP, to exercise reuse; probe; TERM
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
# down: TERM the stack if it still runs; a stack that already exited is fine.
down() {
  kill -TERM "-$stack" 2>/dev/null || kill -TERM "$stack" 2>/dev/null || true
  wait "$stack" 2>/dev/null || true
}
probe() { local t=$SECONDS; env MICOLD_PROBE_TIMEOUT=120 "$ROOT/scripts/local-stack-probe.sh"; say "$1 probe in $((SECONDS - t)) s"; }
trap 'down 2>/dev/null; kind delete cluster --name micold-local >/dev/null 2>&1 || true; rm -f "$LOG"' EXIT

up MICOLD_LOCAL_KEEP=1
probe first

# State in Postgres (SC-008, US5.5). The client API is read-only (no rename), so the probe of state
# is SQL: a marker row, and the daemons the orchestrator registered, both read again after the
# orchestrator and Postgres pods are deleted (the seed re-runs, so the API listing alone proves
# nothing here).
psql() { kubectl --context "$CTX" -n micold exec micold-postgres-0 -- psql -U micold -d micold -tAc "$1"; }
psql "CREATE TABLE IF NOT EXISTS local_stack_marker (v text); INSERT INTO local_stack_marker VALUES ('kept')"
daemons_before="$(psql "SELECT count(*) FROM daemons")"
[ "$daemons_before" -ge 1 ] || { say "no daemon registered in Postgres"; exit 1; }
kubectl --context "$CTX" -n micold delete pod -l app.kubernetes.io/component=orchestrator --wait=true
# The stack's API forward points at the deleted pod. kubectl only notices on the next connection
# (and may then log and carry on), so stop the stack here rather than wait for it to end itself.
down
kind get clusters | grep -qx micold-local || { say "KEEP did not keep the cluster"; exit 1; }
kubectl --context "$CTX" -n micold delete pod micold-postgres-0 --wait=true
kubectl --context "$CTX" -n micold rollout status statefulset/micold-postgres --timeout 5m
kubectl --context "$CTX" -n micold rollout status deployment/micold-orchestrator --timeout 5m
[ "$(psql "SELECT v FROM local_stack_marker")" = kept ] || { say "the marker row did not survive"; exit 1; }
[ "$(psql "SELECT count(*) FROM daemons")" -ge "$daemons_before" ] || { say "registered daemons were lost"; exit 1; }
say "state survived the orchestrator and Postgres restarts"
# Fresh forwards on their own ports for both, so nothing from the stopped stack is reused.
kubectl --context "$CTX" -n micold port-forward deployment/micold-orchestrator 18730:8730 >/dev/null 2>&1 &
fwd_api=$!
kubectl --context "$CTX" -n micold port-forward service/micold-web 18731:8731 >/dev/null 2>&1 &
fwd_web=$!
MICOLD_LOCAL_API_PORT=18730 MICOLD_LOCAL_WEB_PORT=18731 probe "after restart"
kill "$fwd_api" "$fwd_web" 2>/dev/null || true

up MICOLD_LOCAL_KEEP=1
grep -q "reusing cluster micold-local" "$LOG" || { say "the rerun did not reuse the cluster"; exit 1; }
probe reuse
down

up
probe last
down
if kind get clusters | grep -qx micold-local; then say "TERM without KEEP left the cluster"; exit 1; fi
say "ok in $((SECONDS - started)) s"
