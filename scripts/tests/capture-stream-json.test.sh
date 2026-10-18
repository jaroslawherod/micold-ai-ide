#!/usr/bin/env bash
# Drives the scrub step of scripts/capture-stream-json.sh (feature 938, T001) on canned lines: each
# secret a recorded `claude` stream-json line can carry is replaced by its fixed placeholder, and
# nothing else on the line changes. Recording itself needs a logged-in CLI, so it is not run here.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
# shellcheck source=../capture-stream-json.sh
source "$ROOT/scripts/capture-stream-json.sh"

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
CWD="/tmp/capture.Ab12Cd/repo"

scrubbed() { printf '%s\n' "$1" | scrub_stream_json "$SID" "$CWD" "$HOME_DIR"; }

check "the session uuid becomes <SESSION_ID>" \
  '{"type":"system","subtype":"init","session_id":"<SESSION_ID>"}' \
  "$(scrubbed '{"type":"system","subtype":"init","session_id":"'"$SID"'"}')"

check "the scratch repo path becomes <CWD>, before <HOME> can take its prefix" \
  '{"cwd":"<CWD>","file":"<CWD>/a.txt"}' \
  "$(scrubbed '{"cwd":"'"$CWD"'","file":"'"$CWD"'/a.txt"}')"

check "the home directory becomes <HOME>" \
  '{"path":"<HOME>/.claude/settings.json"}' \
  "$(scrubbed '{"path":"'"$HOME_DIR"'/.claude/settings.json"}')"

check "an e-mail address becomes <ACCOUNT>" \
  '{"text":"signed in as <ACCOUNT>."}' \
  "$(scrubbed '{"text":"signed in as jaro.dev+ci@example.co.uk."}')"

check "account and organisation identifiers become <ACCOUNT>" \
  '{"account_uuid":"<ACCOUNT>","organizationUuid":"<ACCOUNT>","email":"<ACCOUNT>","n":1}' \
  "$(scrubbed '{"account_uuid":"11111111-2222-3333-4444-555555555555","organizationUuid":"org-9","email":"x","n":1}')"

check "a line with no secret is unchanged byte for byte" \
  '{"type":"result","subtype":"success","is_error":false,"result":"pong \"quoted\" é"}' \
  "$(scrubbed '{"type":"result","subtype":"success","is_error":false,"result":"pong \"quoted\" é"}')"

check "a path with regex metacharacters is matched literally" \
  '{"cwd":"<CWD>","other":"/tmp/captureXAb12Cd/repo"}' \
  "$(printf '%s\n' '{"cwd":"/tmp/capture.Ab12Cd/repo","other":"/tmp/captureXAb12Cd/repo"}' \
       | scrub_stream_json "$SID" "/tmp/capture.Ab12Cd/repo" "$HOME_DIR")"

echo "$cases cases, $failures failed"
[ "$failures" -eq 0 ]
