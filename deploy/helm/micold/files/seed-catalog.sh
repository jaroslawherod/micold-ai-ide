#!/bin/sh
# Seed one daemon's catalog (#1732, FR-017, FR-021): a git project `demo` with a worktree `feature`,
# and a daemon.toml and projects.json that serve it with two structured sessions, one in the
# project root and one in the worktree. No AI CLI is involved. POSIX sh and git only, so both
# scripts/local-stack.sh (process mode) and the chart's seed init container (kind mode) run it.
#
#   seed-catalog.sh <dir>
#   SEED_NAME                 the daemon's name and id (local)
#   SEED_KIND                 [daemon] kind (host)
#   SEED_LISTEN               [listen] socket: "auto" or a path (auto)
#   SEED_ORCHESTRATOR_SOCKET  [orchestrator_link] socket; unset derives it from the daemon's endpoint

set -eu

dir="$1"
name="${SEED_NAME:-local}"
repo="$dir/seed/demo"
g() { git -C "$repo" -c user.name=micold-local -c user.email=local@micold.invalid "$@"; }

mkdir -p "$repo" "$dir/daemon" "$dir/orchestrator" "$dir/run"
chmod 700 "$dir/run"
if [ ! -d "$repo/.git" ]; then
  g init -q -b main
  echo "# demo" >"$repo/README.md"
  g add README.md
  g commit -q -m "demo"
  g worktree add -q -b feature "$repo/.claude/worktrees/feature"
fi
cat >"$dir/daemon/projects.json" <<JSON
{"schema_version":1,"projects":[{"path":"$repo","display_name":"demo","is_git_repo":true,"sessions":[
{"id":"6f1c2a54-0000-4000-8000-000000000001","title":"Fix the login bug","kind":"Structured"},
{"id":"6f1c2a54-0000-4000-8000-000000000002","worktree_dir":"feature","title":"Add dark mode","kind":"Structured"}]}]}
JSON
link_socket=""
[ -z "${SEED_ORCHESTRATOR_SOCKET:-}" ] || link_socket="socket = \"$SEED_ORCHESTRATOR_SOCKET\""
cat >"$dir/daemon.toml" <<TOML
schema = 1

[daemon]
name = "$name"
id = "$name"
kind = "${SEED_KIND:-host}"
data_dir = "$dir/daemon"

[listen]
socket = "${SEED_LISTEN:-auto}"

[orchestrator_link]
enabled = true
$link_socket

[[projects]]
path = "$repo"
TOML
