#!/usr/bin/env bash
# Run the whole stack on this machine (#1732, FR-017, research R11/R12): the orchestrator with its
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

# seed <dir>: a git project with a worktree, and a daemon.toml and projects.json that serve it with
# two structured sessions, one in the project root and one in the worktree. No AI CLI is involved.
seed() {
  local dir="$1" repo="$1/seed/demo" g
  g=(git -C "$repo" -c user.name=micold-local -c user.email=local@micold.invalid)
  mkdir -p "$repo" "$dir/daemon" "$dir/orchestrator" "$dir/run"
  chmod 700 "$dir/run"
  "${g[@]}" init -q -b main
  echo "# demo" >"$repo/README.md"
  "${g[@]}" add README.md
  "${g[@]}" commit -q -m "demo"
  "${g[@]}" worktree add -q -b feature "$repo/.claude/worktrees/feature"
  cat >"$dir/daemon/projects.json" <<JSON
{"schema_version":1,"projects":[{"path":"$repo","display_name":"demo","is_git_repo":true,"sessions":[
{"id":"6f1c2a54-0000-4000-8000-000000000001","title":"Fix the login bug","kind":"Structured"},
{"id":"6f1c2a54-0000-4000-8000-000000000002","worktree_dir":"feature","title":"Add dark mode","kind":"Structured"}]}]}
JSON
  cat >"$dir/daemon.toml" <<TOML
schema = 1

[daemon]
name = "local"
id = "local"
kind = "host"
data_dir = "$dir/daemon"

[listen]
socket = "auto"

[orchestrator_link]
enabled = true

[[projects]]
path = "$repo"
TOML
}

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
  wait -n || true
  local i
  for i in "${!pids[@]}"; do
    kill -0 "${pids[$i]}" 2>/dev/null || say "the ${names[$i]} (pid ${pids[$i]}) exited" >&2
  done
  say "a process exited; stopping the others"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then main "$@"; fi
