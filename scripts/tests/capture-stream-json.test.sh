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

delta() { printf '{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"%s"}}}' "$1"; }
joined_input() { jq -rs '[.[] | .event.delta.partial_json // empty] | join("")'; }

split_run="$(delta '{\"file_path\": \"/tmp/capture.Ab'; echo; delta '12Cd/repo/notes.txt\", \"id\": \"0b6f2c1e-4a7d'; echo; delta '-4c55-9f1e-2d3c4b5a6978\"}'; echo)"
check "a path and the session uuid cut across input_json_delta chunks are scrubbed" \
  '{"file_path": "<CWD>/notes.txt", "id": "<SESSION_ID>"}' \
  "$(printf '%s\n' "$split_run" | scrub_split_deltas "$SID" "$CWD" "$HOME_DIR" | joined_input)"

check "a scrubbed run keeps one line per chunk" \
  3 \
  "$(printf '%s\n' "$split_run" | scrub_split_deltas "$SID" "$CWD" "$HOME_DIR" | wc -l | tr -d ' ')"

unchanged="$(delta '{\"x\": 1}'; echo; echo '{"type":"result","n":1.50}')"
check "lines with no split secret pass through byte for byte" \
  "$unchanged" \
  "$(printf '%s\n' "$unchanged" | scrub_split_deltas "$SID" "$CWD" "$HOME_DIR")"

# The CLI the script runs sees CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1 (feature 944, research R5): a
# stand-in `claude` on PATH writes the variable's value as its one stdout line, and the capture,
# run in a throwaway git repo, saves that line as the fixture.
fake_root="$(mktemp -d)"
mkdir -p "$fake_root/bin" "$fake_root/repo"
cat > "$fake_root/bin/claude" <<'FAKE'
#!/usr/bin/env bash
cat >/dev/null
printf '{"forward":"%s"}\n' "${CLAUDE_CODE_FORWARD_SUBAGENT_TEXT:-unset}"
FAKE
chmod +x "$fake_root/bin/claude"
git -C "$fake_root/repo" init -q
(cd "$fake_root/repo" && unset CLAUDE_CODE_FORWARD_SUBAGENT_TEXT \
  && PATH="$fake_root/bin:$PATH" TMPDIR="$fake_root" capture_main probe "ping") 2>/dev/null
check "the recorded CLI runs with CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1" \
  '{"forward":"1"}' \
  "$(cat "$fake_root/repo/crates/micold-core/tests/fixtures/stream-json/probe.jsonl" 2>/dev/null)"
rm -rf "$fake_root"

# `--image <png>` (feature 941, T001): the one stdin line carries the prompt as a text block, then
# the picture as a base64 image block whose media type comes from the file's bytes (P-5). A
# stand-in `claude` echoes its stdin line as its stdout, so the fixture is that line, scrubbed.
fake_root="$(mktemp -d)"
mkdir -p "$fake_root/bin" "$fake_root/repo"
cat > "$fake_root/bin/claude" <<'FAKE'
#!/usr/bin/env bash
IFS= read -r line
printf '%s\n' "$line"
printf '{"cwd":"%s"}\n' "$PWD"
FAKE
chmod +x "$fake_root/bin/claude"
git -C "$fake_root/repo" init -q
printf '\x89PNG\r\n\x1a\nIHDR' > "$fake_root/pic.png"
(cd "$fake_root/repo" && PATH="$fake_root/bin:$PATH" TMPDIR="$fake_root" \
  capture_main --image "$fake_root/pic.png" probe 'what is "this"?') 2>/dev/null
fixture="$fake_root/repo/crates/micold-core/tests/fixtures/stream-json/probe.jsonl"
check "--image writes the text block, then the image block with the file's media type" \
  '{"type":"user","message":{"role":"user","content":[{"type":"text","text":"what is \"this\"?"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"iVBORw0KGgpJSERS"}}]}}' \
  "$(sed -n 1p "$fixture" 2>/dev/null)"
check "--image output is scrubbed as the other modes are" \
  '{"cwd":"<CWD>"}' \
  "$(sed -n 2p "$fixture" 2>/dev/null)"
check "a file that is not a known image is refused" \
  1 \
  "$( (cd "$fake_root/repo" && printf 'plain text' > "$fake_root/a.png" \
       && PATH="$fake_root/bin:$PATH" TMPDIR="$fake_root" \
       capture_main --image "$fake_root/a.png" probe2 hi) >/dev/null 2>&1; echo $?)"
rm -rf "$fake_root"

# `--interrupt-after-first-delta` (feature 941, T001): the capture writes P-4's interrupt request
# after the first text delta it reads, and keeps stdin open until the `result`. The stand-in
# streams one text delta, then echoes the next stdin line it reads inside a marker line, then ends
# the turn; a thinking delta before it must not trigger the interrupt.
fake_root="$(mktemp -d)"
mkdir -p "$fake_root/bin" "$fake_root/repo"
cat > "$fake_root/bin/claude" <<'FAKE'
#!/usr/bin/env bash
IFS= read -r first
for arg in "$@"; do [ "$prev" = --session-id ] && sid="$arg"; prev="$arg"; done
printf '{"type":"system","subtype":"init","session_id":"%s","cwd":"%s"}\n' "$sid" "$PWD"
printf '{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hm"}}}\n'
printf '{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Once"}}}\n'
IFS= read -r second
printf '{"stdin":%s}\n' "$second"
printf '{"type":"result","subtype":"error_during_execution","is_error":true}\n'
IFS= read -r rest && printf '{"unexpected":"%s"}\n' "$rest"
exit 0
FAKE
chmod +x "$fake_root/bin/claude"
git -C "$fake_root/repo" init -q
(cd "$fake_root/repo" && PATH="$fake_root/bin:$PATH" TMPDIR="$fake_root" \
  timeout 20 bash -c "source '$ROOT/scripts/capture-stream-json.sh'; capture_main --interrupt-after-first-delta probe 'tell a long story'") 2>/dev/null
fixture="$fake_root/repo/crates/micold-core/tests/fixtures/stream-json/probe.jsonl"
check "--interrupt-after-first-delta writes the interrupt request after the first text delta" \
  '{"stdin":{"type":"control_request","request_id":"micold-1","request":{"subtype":"interrupt"}}}' \
  "$(sed -n 4p "$fixture" 2>/dev/null)"
check "--interrupt-after-first-delta closes stdin after the result and keeps every line" \
  5 \
  "$(wc -l < "$fixture" 2>/dev/null | tr -d ' ')"
check "--interrupt-after-first-delta output is scrubbed (session id, cwd)" \
  '{"type":"system","subtype":"init","session_id":"<SESSION_ID>","cwd":"<CWD>"}' \
  "$(sed -n 1p "$fixture" 2>/dev/null)"
rm -rf "$fake_root"

# `--control <json>` (feature 943, T001): each control line goes to the CLI's stdin, in the order
# given, before the user message; the scratch repo holds the project skill `greet` and the custom
# command `hello` the command fixtures run. The stand-in `claude` echoes its stdin, then one line
# per scratch file it finds.
fake_root="$(mktemp -d)"
mkdir -p "$fake_root/bin" "$fake_root/repo"
cat > "$fake_root/bin/claude" <<'FAKE'
#!/usr/bin/env bash
cat
[ -s .claude/skills/greet/SKILL.md ] && echo '{"file":"greet"}'
[ -s .claude/commands/hello.md ] && echo '{"file":"hello"}'
exit 0
FAKE
chmod +x "$fake_root/bin/claude"
git -C "$fake_root/repo" init -q
(cd "$fake_root/repo" \
  && PATH="$fake_root/bin:$PATH" TMPDIR="$fake_root" capture_main probe "ping" \
       --control '{"type":"control_request","request_id":"a","request":{"subtype":"initialize"}}' \
       --control '{"type":"control_request","request_id":"b","request":{"subtype":"set_model","model":"haiku"}}' \
       -- --model haiku) 2>/dev/null
check "--control lines reach stdin in order, before the user message, and the scratch repo holds greet and hello" \
  "$(printf '%s\n' \
      '{"type":"control_request","request_id":"a","request":{"subtype":"initialize"}}' \
      '{"type":"control_request","request_id":"b","request":{"subtype":"set_model","model":"haiku"}}' \
      '{"type":"user","message":{"role":"user","content":[{"type":"text","text":"ping"}]}}' \
      '{"file":"greet"}' '{"file":"hello"}')" \
  "$(cat "$fake_root/repo/crates/micold-core/tests/fixtures/stream-json/probe.jsonl" 2>/dev/null)"
rm -rf "$fake_root"

echo "$cases cases, $failures failed"
[ "$failures" -eq 0 ]
