#!/usr/bin/env bash
# Feature 552 (FR-012, FR-013): the user guide describes the shipped behaviour, and every spec whose
# rules 552 removed starts with a note naming 552.
#
# Two assertions. (1) No `Show agent worktrees`, `show_agent_worktrees`, `worktree-agent-` or
# `worktree provenance` in docs/user-guide: the guide names no toggle and no name-based hiding.
# (2) Every spec that mentions the toggle, plus the three the feature names, carries a note naming
# 552-native-agent-session-signals in its first lines.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

hits="$(grep -rniE 'show agent worktrees|show_agent_worktrees|worktree-agent-|worktree provenance' docs/user-guide || true)"
if [ -n "$hits" ]; then
  fail "docs/user-guide names the removed toggle or provenance:"
  printf '%s\n' "$hits"
else
  pass "docs/user-guide names no toggle, name-based hiding or provenance"
fi

dirs="$(grep -rlE 'Show agent worktrees|show_agent_worktrees' specs \
  | sed 's|^\(specs/[^/]*\)/.*|\1|' | sort -u | grep -v '^specs/552-' || true)"
dirs="$(printf '%s\nspecs/014-hide-agent-worktrees\nspecs/029-worktree-provenance\nspecs/582-attach-provider-worktrees-sessions\n' "$dirs" | sort -u | sed '/^$/d')"

for d in $dirs; do
  if head -8 "$d/spec.md" | grep -q '552-native-agent-session-signals'; then
    pass "$d/spec.md names 552"
  else
    fail "$d/spec.md has no superseded note naming 552-native-agent-session-signals"
  fi
done

[ "$failures" -eq 0 ]
