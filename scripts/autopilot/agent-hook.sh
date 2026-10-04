#!/usr/bin/env bash
# PreToolUse(Agent) hook: a subagent of an autopilot run spawns the small agent types, not
# general-purpose.
#
# Every subagent carries its system prompt and tool schemas on each of its calls. In four measured
# runs that baseline, about 17k tokens times 266 subagents, was the largest single cost, a fifth of
# the total, and a general-purpose helper carries about 10k more than an autopilot type (the
# Artifact schema alone). rules/delegate.md names the types; this hook holds helpers to them.
#
# A no-op unless the caller is itself a subagent and its directory, or the session's, is on a
# branch an autopilot ledger names. There it blocks, with exit 2 and the reason on stderr, an Agent
# call whose subagent_type is general-purpose, claude or missing. The orchestrator is never
# blocked: it may fall back to general-purpose for a unit.
set -uo pipefail
in=$(cat)
IFS=$'\t' read -r agent type cwd < <(jq -r '[(.agent_id // "-"), (.tool_input.subagent_type // "general-purpose"),
  (.cwd // ".")] | @tsv' <<<"$in") || exit 0
[ "$agent" != - ] || exit 0
case $type in general-purpose|claude) ;; *) exit 0 ;; esac
. "$(dirname "$0")/ledger-of.sh"
[ -n "$(run_ledger "$cwd")" ] || exit 0
[ -f "$(git -C "$cwd" rev-parse --show-toplevel 2>/dev/null)/.claude/agents/autopilot-worker.md" ] || exit 0
echo "autopilot agents: do not spawn '$type' from a subagent: it carries about 10k more tokens on every call. Use subagent_type autopilot-worker (a mechanical task, a multi-step git job, a check of static text), autopilot-reviewer (a review; it cannot edit) or Explore (finding code). See rules/delegate.md." >&2
exit 2
