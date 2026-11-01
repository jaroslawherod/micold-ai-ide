#!/usr/bin/env bash
# PreToolUse(Bash) hook: refuse a bare `cargo build|test|check|clippy|run|bench|nextest`.
# A bare cargo in a worktree builds into a private target-shared/ beside the worktree and skips the
# repo-wide build lock; `mise run <task>` (scripts/build-lock.sh) uses the one shared dir and lock.
# Exit 2 with the reason on stderr. Why: docs/development/shared-build.md.
set -uo pipefail
cmd=$(jq -r '.tool_input.command // empty' 2>/dev/null) || exit 0
re='(^|[;&|(]|$'\n')[[:space:]]*cargo[[:space:]]+((\+|-)[^[:space:]]*[[:space:]]+)*(build|test|check|clippy|run|bench|nextest)([[:space:]]|$)'
[[ $cmd =~ $re ]] || exit 0
[[ $cmd =~ CARGO_TARGET_DIR= ]] && exit 0
echo "bare cargo: it builds into a private target dir and skips the build lock. Use 'mise run <task>' (mise tasks), or 'scripts/build-lock.sh cargo ...' for an ad hoc command." >&2
exit 2
