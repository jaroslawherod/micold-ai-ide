#!/usr/bin/env bash
# Drives the scrub step of scripts/capture-permission-prompt.sh (#942, T001) on a sample fixture:
# the scratch path, the home path and the session uuid a recorded prompt call can carry are replaced
# by their placeholders, the result is still valid JSON, and nothing else changes. Recording itself
# needs a logged-in CLI, so it is not run here.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
# shellcheck source=../capture-permission-prompt.sh
source "$ROOT/scripts/capture-permission-prompt.sh"

failures=0
cases=0

check() {
  local name="$1" expected="$2" actual="$3"
  cases=$((cases + 1))
  if [ "$expected" = "$actual" ]; then
    echo "ok   $name"
  else
    failures=$((failures + 1))
    echo "FAIL $name"
    echo "     expected: $expected"
    echo "     actual:   $actual"
  fi
}

SID="0b6f2c1e-4a7d-4c55-9f1e-2d3c4b5a6978"
HOME_DIR="/home/jaro"
CWD="/tmp/micold-capture-$SID"

sample="$(jq -cn --arg cwd "$CWD" --arg home "$HOME_DIR" --arg sid "$SID" '{
  case: "tool-request",
  call: {
    arguments: {tool_name: "Bash", input: {command: ("ls " + $cwd + " " + $home + "/.claude"),
                description: "List"}, tool_use_id: "toolu_01"},
    _meta: {"claudecode/toolUseId": "toolu_01", session: $sid}
  },
  answer: {behavior: "allow", updatedInput: {command: ("ls " + $cwd), description: "List"}},
  tool_result: ("total 0 in " + $cwd + " é \"quoted\"")
}')"

if ! command -v scrub_prompt_fixture >/dev/null 2>&1; then
  echo "FAIL scrub_prompt_fixture is not defined by scripts/capture-permission-prompt.sh"
  exit 1
fi

scrubbed="$(printf '%s\n' "$sample" | scrub_prompt_fixture "$SID" "$CWD" "$HOME_DIR")"

check "the scrubbed fixture is valid JSON" \
  ok \
  "$(printf '%s' "$scrubbed" | jq -e . >/dev/null 2>&1 && echo ok || echo invalid)"

check "the scratch path becomes <CWD>, before <SESSION_ID> can take the uuid inside it" \
  'ls <CWD> <HOME>/.claude' \
  "$(printf '%s' "$scrubbed" | jq -r '.call.arguments.input.command')"

check "the session uuid becomes <SESSION_ID>" \
  '<SESSION_ID>' \
  "$(printf '%s' "$scrubbed" | jq -r '.call._meta.session')"

check "the tool result keeps everything but the path" \
  'total 0 in <CWD> é "quoted"' \
  "$(printf '%s' "$scrubbed" | jq -r '.tool_result')"

check "no secret is left anywhere" \
  0 \
  "$(printf '%s' "$scrubbed" | grep -c -e "$SID" -e "$HOME_DIR" -e "/tmp/micold-capture")"

check "fields without a secret are unchanged" \
  "$(printf '%s' "$sample" | jq -c '.call.arguments.tool_use_id, .answer.behavior, .case')" \
  "$(printf '%s' "$scrubbed" | jq -c '.call.arguments.tool_use_id, .answer.behavior, .case')"

echo "$cases cases, $failures failed"
[ "$failures" -eq 0 ]
