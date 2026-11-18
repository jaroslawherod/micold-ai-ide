#!/usr/bin/env bash
# Run the whole stack locally (#1732). Two modes, by MICOLD_LOCAL_MODE:
#   kind     (default, M5, FR-019) a kind cluster running it all: scripts/local-stack-kind.sh
#   process  (M4, FR-017) the processes below, directly on this machine
#
# Process mode (research R11/R12): the orchestrator with its
# client API, one daemon that dials it and serves a seeded project (a worktree and two sessions), and
# the web client's export served against it, allowing that client's origin. Ctrl+C stops all three.
#
#   MICOLD_LOCAL_API_PORT   the orchestrator's /api/ws port (8730)
#   MICOLD_LOCAL_WEB_PORT   the web client's port (8731)
#   MICOLD_LOCAL_DIR        the private data directory (a fresh one under /tmp, removed at exit)
# For tests and for a prebuilt tree:
#   MICOLD_LOCAL_DAEMON_BIN the micold-daemon binary (built with cargo when unset)
#   MICOLD_LOCAL_WEB_DIR    an existing web export to serve (built with pnpm when unset)
#
# Both modes check the two ports first and exit, naming the port and its holder, if one is in use.
# Only the host's loopback is used; nothing is reachable from another machine.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
API_PORT="${MICOLD_LOCAL_API_PORT:-8730}"
WEB_PORT="${MICOLD_LOCAL_WEB_PORT:-8731}"

say() { echo "local-stack: $*"; }

# in_use <port>: whether something already listens on 127.0.0.1:<port>.
in_use() { (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null; }

check_ports() {
  local port holder bad=0
  for port in "$API_PORT" "$WEB_PORT"; do
    if in_use "$port"; then
      holder="$(ss -ltnp "sport = :$port" 2>/dev/null | awk 'NR>1 {print $NF}' | head -1 || true)"
      say "port $port is already in use${holder:+ by $holder}: is another local stack running? Stop it, or set MICOLD_LOCAL_API_PORT and MICOLD_LOCAL_WEB_PORT." >&2
      bad=1
    fi
  done
  return "$bad"
}

# seed <dir>: the seeded catalog, shared with the chart's seed init container (kind mode).
seed() { sh "$ROOT/deploy/helm/micold/files/seed-catalog.sh" "$1"; }

# Globals, not locals: the EXIT trap runs `stop` after `main` has returned.
dir="${MICOLD_LOCAL_DIR:-}"
own_dir=0
pids=()
names=()

stop() {
  trap - INT TERM EXIT
  local pid
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  for pid in "${pids[@]}"; do wait "$pid" 2>/dev/null || true; done
  if [ "$own_dir" = 1 ]; then rm -rf "$dir"; fi
  say "stopped"
}

main() {
  check_ports || exit 1
  if [ -z "$dir" ]; then
    dir="$(mktemp -d /tmp/micold-local.XXXXXX)"
    own_dir=1
  fi
  trap 'stop; exit 130' INT TERM
  trap stop EXIT

  local web_dir="${MICOLD_LOCAL_WEB_DIR:-$ROOT/apps/client/dist}"
  if [ -z "${MICOLD_LOCAL_WEB_DIR:-}" ]; then
    say "building the web client for ws://127.0.0.1:$API_PORT/api/ws"
    (cd "$ROOT" && pnpm install --frozen-lockfile >/dev/null &&
      EXPO_PUBLIC_ORCHESTRATOR_URL="ws://127.0.0.1:$API_PORT/api/ws" pnpm --filter @micold/client web:export >/dev/null)
  fi

  local daemon="${MICOLD_LOCAL_DAEMON_BIN:-}"
  if [ -z "$daemon" ]; then
    say "building the daemon"
    "$ROOT/scripts/build-lock.sh" cargo build -p micold-daemon
    daemon="$("$ROOT/scripts/build-lock.sh" --print-target-dir)/debug/micold-daemon"
  fi

  seed "$dir"
  export XDG_RUNTIME_DIR="$dir/run"
  # The build and the seed took a while: another stack may have taken the ports since the first check.
  check_ports || exit 1

  "$daemon" orchestrator --data-dir "$dir/orchestrator" --api-addr "127.0.0.1:$API_PORT" \
    --allow-origin "http://127.0.0.1:$WEB_PORT" --allow-origin "http://localhost:$WEB_PORT" &
  pids+=("$!")
  names+=("orchestrator")
  # The daemon dials the orchestrator's socket and keeps dialling until it is there.
  "$daemon" --config "$dir/daemon.toml" &
  pids+=("$!")
  names+=("daemon")
  node "$ROOT/apps/client/e2e/serve.mjs" "$web_dir" "$WEB_PORT" &
  pids+=("$!")
  names+=("web server")

  say "orchestrator ws://127.0.0.1:$API_PORT/api/ws"
  say "web client    http://127.0.0.1:$WEB_PORT  (Ctrl+C stops all)"
  # One process going away ends the stack: a half stack is worse than none.
  # `-p` names the process wait -n reaped; a kill -0 probe would see an unreaped zombie as alive.
  local gone="" i
  wait -n -p gone || true
  for i in "${!pids[@]}"; do
    [ "${pids[$i]}" = "$gone" ] && say "the ${names[$i]} (pid $gone) exited" >&2
  done
  say "a process exited; stopping the others"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  case "${MICOLD_LOCAL_MODE:-kind}" in
    # The kind mode forwards the same two ports: refuse a held one before creating anything.
    kind) check_ports || exit 1; exec "$ROOT/scripts/local-stack-kind.sh" "$@" ;;
    process) main "$@" ;;
    *) say "unknown MICOLD_LOCAL_MODE=${MICOLD_LOCAL_MODE} (kind or process)" >&2; exit 2 ;;
  esac
fi
