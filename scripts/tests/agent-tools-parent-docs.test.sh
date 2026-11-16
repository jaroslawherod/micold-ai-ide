#!/usr/bin/env bash
# Spec 1945 (FR-010, SC-004; US5): the agent tools guide and the autopilot-supervisor skill tell
# agents about the three things the feature added: the `neighbourhood` tool, the parent message
# route (`send_parent_input`) and the implicit `default` parent, and the guide says `default` is a
# reserved worktree name.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

guide=docs/user-guide/agent-tools.md
skill=.claude/skills/autopilot-supervisor/SKILL.md

want() { # file, fixed text, what
  if grep -qF -- "$2" "$1"; then pass "$1: $3"; else fail "$1: missing $3 ('$2')"; fi
}

want "$guide" '| `neighbourhood` |' "the neighbourhood tool"
want "$guide" '### Messaging the parent' "the parent message route"
want "$guide" 'the implicit `default` for a worktree with no recorded parent' "the implicit parent"
want "$guide" 'The name `default` is reserved' "the reserved worktree name"
want "$guide" 'before the session' "the link recorded before the session starts"
want "$skill" '`send_parent_input`' "the parent route"
want "$skill" '`neighbourhood`' "the neighbourhood tool"

[ "$failures" -eq 0 ]
