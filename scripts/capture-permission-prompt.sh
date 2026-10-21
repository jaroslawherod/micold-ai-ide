#!/usr/bin/env bash
# Record one permission-prompt fixture for #942 (FR-020, research R12).
#
# Usage: scripts/capture-permission-prompt.sh <case>
#   cases: tool-request always-allow plan-approval question-multi mode-change
#
# In a fresh scratch git repo it starts the stub tool server scripts/capture/prompt-stub.py and runs
# the real `claude` with feature 938's structured argv
#   claude -p --input-format stream-json --output-format stream-json --verbose
#          --include-partial-messages --session-id <new uuid> [case flags]
#          --permission-prompt-tool mcp__micold__permission_prompt
#          --mcp-config <binding> --allowedTools mcp__micold
# where the binding points the `micold` server at the stub with `"timeout": 2147483647` (R3). The
# prompt goes in on stdin as one stream-json user message; stdin stays open until the turn's
# `result`, so `mode-change` can then send its control requests. It writes
#   crates/micold-core/tests/fixtures/permission-prompt/<case>.json
#     `call` (the recorded tools/call arguments and _meta), `answer` (what the stub returned),
#     `tool_result` (the CLI's resulting tool-result text), `later` (any further calls of the run);
#     for `mode-change`, `runs`: each launch's flags and its control request / response lines
#   crates/micold-core/tests/fixtures/stream-json/prompt-<case>.jsonl (the run's stdout; for
#     `mode-change` also prompt-mode-change-bypass.jsonl, the launch with the bypass flag)
# scrubbed with `scrub_stream_json` of scripts/capture-stream-json.sh (sourced, never copied).
#
# Needs a logged-in `claude`; CI has none and never runs this. Sourced (as
# scripts/tests/capture-permission-prompt.test.sh does), it only defines the functions.

PROMPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=capture-stream-json.sh
source "$PROMPT_ROOT/scripts/capture-stream-json.sh"

PROMPT_TOOL="mcp__micold__permission_prompt"

# Scrub a fixture's JSON from stdin to stdout, pretty-printed: the same placeholders as the
# stream-json fixtures. Compacted first, so the account-identifier pattern sees `"key":"value"`.
scrub_prompt_fixture() {
  jq -c . | scrub_stream_json "$1" "$2" "$3" | jq .
}

# Run `claude` with the argv in "$@" in the current directory: write the prompt line, read stdout
# until the turn's `result`, then send each control request in $PROMPT_CONTROL (one per line) and
# read until its `control_response`, then close stdin and read to the end. Stdout lines go to
# $PROMPT_OUT, the control exchanges to $PROMPT_EXCHANGES as JSON lines {request, response}.
drive_claude() {
  python3 -I -c '
import json, os, subprocess, sys, threading, queue
prompt, control, out_path, exchanges_path, argv = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:]
proc = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
lines = queue.Queue()
def pump():
    for line in proc.stdout:
        lines.put(line)
    lines.put(None)
threading.Thread(target=pump, daemon=True).start()
out = open(out_path, "w", encoding="utf-8")
def read_until(match, seconds):
    while True:
        try:
            line = lines.get(timeout=seconds)
        except queue.Empty:
            sys.exit(f"no matching line within {seconds} s")
        if line is None:
            sys.exit("claude ended before the line it waited for")
        out.write(line)
        try:
            value = json.loads(line)
        except ValueError:
            continue
        if match(value):
            return value
def send(line):
    proc.stdin.write(line + "\n")
    proc.stdin.flush()
send(prompt)
read_until(lambda v: v.get("type") == "result", 600)
with open(exchanges_path, "w", encoding="utf-8") as exchanges:
    for request in filter(None, control.splitlines()):
        request_id = json.loads(request)["request_id"]
        send(request)
        response = read_until(
            lambda v: v.get("type") == "control_response"
            and v.get("response", {}).get("request_id") == request_id, 60)
        exchanges.write(json.dumps({"request": json.loads(request), "response": response}) + "\n")
proc.stdin.close()
while (line := lines.get(timeout=120)) is not None:
    out.write(line)
proc.wait(timeout=60)
' "$@"
}

# The fixture of one prompt case: the stub's log ($1), the run's stdout ($2), the case name ($3)
# and the case flags (the rest), as one compact JSON object.
prompt_fixture_json() {
  python3 -I -c '
import json, sys
log_path, out_path, case, version, flags = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:]
results = {}
for line in open(out_path, encoding="utf-8"):
    try:
        value = json.loads(line)
    except ValueError:
        continue
    if value.get("type") != "user":
        continue
    content = value.get("message", {}).get("content")
    for block in content if isinstance(content, list) else []:
        if block.get("type") == "tool_result":
            text = block.get("content")
            if isinstance(text, list):
                text = "".join(part.get("text", "") for part in text if isinstance(part, dict))
            results.setdefault(block.get("tool_use_id"), text or "")
exchanges = []
for line in open(log_path, encoding="utf-8"):
    record = json.loads(line)
    tool_use_id = record["arguments"].get("tool_use_id") or (record["_meta"] or {}).get("claudecode/toolUseId")
    exchanges.append({
        "call": {"arguments": record["arguments"], "_meta": record["_meta"]},
        "answer": record["answer"],
        "tool_result": results.get(tool_use_id),
    })
if not exchanges:
    sys.exit(f"{case}: the CLI made no prompt call; nothing recorded")
fixture = {"case": case, "cli_version": version, "flags": flags}
fixture.update(exchanges[0])
fixture["later"] = exchanges[1:]
print(json.dumps(fixture, ensure_ascii=False))
' "$@"
}

# The `mode-change` fixture: each run's flags and control exchanges, from files named
# <dir>/<run>.flags and <dir>/<run>.exchanges ($1 the dir, then the run names).
mode_change_json() {
  python3 -I -c '
import json, sys
directory, version, runs = sys.argv[1], sys.argv[2], sys.argv[3:]
fixture = {"case": "mode-change", "cli_version": version, "runs": []}
for run in runs:
    flags = [f for f in open(f"{directory}/{run}.flags", encoding="utf-8").read().splitlines() if f]
    control = [json.loads(l) for l in open(f"{directory}/{run}.exchanges", encoding="utf-8") if l.strip()]
    fixture["runs"].append({"run": run, "flags": flags, "control": control})
print(json.dumps(fixture, ensure_ascii=False))
' "$@"
}

# One stream-json control request line setting the mode (contracts/claude-permission.md).
set_mode_line() {
  jq -cn --arg id "$1" --arg mode "$2" \
    '{type:"control_request",request_id:$id,request:{subtype:"set_permission_mode",mode:$mode}}'
}

# Start the stub with the answers in "$@"; sets STUB_PID, STUB_PORT and STUB_LOG.
start_stub() {
  local dir="$1"
  shift
  local answers=()
  for answer in "$@"; do answers+=(--answer "$answer"); done
  STUB_LOG="$dir/stub.log"
  : > "$STUB_LOG"
  rm -f "$dir/port"
  python3 -I "$PROMPT_ROOT/scripts/capture/prompt-stub.py" \
    --port-file "$dir/port" --log "$STUB_LOG" "${answers[@]}" &
  STUB_PID=$!
  for _ in $(seq 50); do [ -s "$dir/port" ] && break; sleep 0.1; done
  STUB_PORT="$(cat "$dir/port")"
}

# Run one launch of a case: $1 work dir, $2 run name, $3 prompt, $4 control lines, the rest the case
# flags. Leaves <dir>/<run>.out (stdout), <run>.exchanges and <run>.flags.
run_case() {
  local dir="$1" run="$2" prompt="$3" control="$4"
  shift 4
  local binding="$dir/binding.json"
  jq -n --arg url "http://127.0.0.1:$STUB_PORT/mcp" \
    '{mcpServers:{micold:{type:"http",url:$url,headers:{Authorization:"Bearer capture"},timeout:2147483647}}}' \
    > "$binding"
  printf '%s\n' "$@" > "$dir/$run.flags"
  (cd "$REPO" && drive_claude "$(user_message_line "$prompt")" "$control" \
     "$dir/$run.out" "$dir/$run.exchanges" \
     claude -p --input-format stream-json --output-format stream-json --verbose \
       --include-partial-messages --session-id "$SID" "$@" \
       --permission-prompt-tool "$PROMPT_TOOL" --mcp-config "$binding" --allowedTools mcp__micold)
}

# Scrub a run's stdout ($1) into a fixture file ($2); $3 is the run's session uuid.
save_stdout() {
  scrub_split_deltas "$3" "$REPO" "$HOME" < "$1" | scrub_stream_json "$3" "$REPO" "$HOME" > "$2"
  echo "stdout -> $2 ($(wc -l < "$2") lines)" >&2
}

prompt_main() {
  set -euo pipefail
  [ $# -eq 1 ] || { echo "usage: $0 tool-request|always-allow|plan-approval|question-multi|mode-change" >&2; exit 2; }
  local case="$1" prompt flags=() answers=()
  case "$case" in
    tool-request)
      prompt='Use the Bash tool once to run exactly this command: touch micold-probe.txt. Then reply with the word done.'
      answers=(allow) ;;
    always-allow)
      prompt='Use the Bash tool to run exactly this command: touch micold-always.txt. When it has run, run exactly the same command a second time with the Bash tool, as a separate call. Then reply with the word done.'
      answers=(always-allow allow) ;;
    plan-approval)
      prompt='Plan how to create a file named hello.txt in the current directory containing the single word hi. Keep the plan to three short lines and present it with the ExitPlanMode tool. Once it is approved, create the file with the Write tool and reply with the word done.'
      flags=(--permission-mode plan)
      answers=(allow allow) ;;
    question-multi)
      prompt='Use the AskUserQuestion tool exactly once, with two questions. First: question "Which color?", header "Color", single choice, options "red", "green" and "blue". Second: question "Which fruits?", header "Fruits", multiSelect true, options "apple", "pear" and "plum". Then reply with the answers you got and nothing else.'
      answers=(answer-first) ;;
    mode-change) ;;
    *) echo "unknown case: $case" >&2; exit 2 ;;
  esac

  local root fixtures streams version
  root="$(git rev-parse --show-toplevel)"
  fixtures="$root/crates/micold-core/tests/fixtures/permission-prompt"
  streams="$root/crates/micold-core/tests/fixtures/stream-json"
  mkdir -p "$fixtures" "$streams"
  version="$(claude --version | awk '{print $1}')"
  WORK="$(mktemp -d)"
  trap '[ -n "${STUB_PID:-}" ] && kill "$STUB_PID" 2>/dev/null; rm -rf "$WORK"' EXIT
  local work="$WORK"

  SID="$(cat /proc/sys/kernel/random/uuid 2>/dev/null || uuidgen | tr 'A-Z' 'a-z')"
  REPO="${TMPDIR:-/tmp}/micold-capture-$SID"
  mkdir -p "$REPO"
  REPO="$(cd "$REPO" && pwd -P)"
  git -C "$REPO" init -q
  printf 'The answer is 42.\n' > "$REPO/notes.txt"

  if [ "$case" = mode-change ]; then
    local control first="$SID"
    control="$(set_mode_line micold-1 acceptEdits; set_mode_line micold-2 bypassPermissions)"
    start_stub "$work"
    run_case "$work" not-launched 'Reply with the single word ready.' "$control"
    SID="$(cat /proc/sys/kernel/random/uuid 2>/dev/null || uuidgen | tr 'A-Z' 'a-z')"
    run_case "$work" launched 'Reply with the single word ready.' \
      "$(set_mode_line micold-1 bypassPermissions)" --allow-dangerously-skip-permissions
    save_stdout "$work/not-launched.out" "$streams/prompt-mode-change.jsonl" "$first"
    save_stdout "$work/launched.out" "$streams/prompt-mode-change-bypass.jsonl" "$SID"
    mode_change_json "$work" "$version" not-launched launched \
      | scrub_prompt_fixture "$first" "$REPO" "$HOME" \
      | scrub_prompt_fixture "$SID" "$REPO" "$HOME" > "$fixtures/mode-change.json"
  else
    start_stub "$work" "${answers[@]}"
    run_case "$work" run "$prompt" "" "${flags[@]}"
    save_stdout "$work/run.out" "$streams/prompt-$case.jsonl" "$SID"
    prompt_fixture_json "$STUB_LOG" "$work/run.out" "$case" "$version" "${flags[@]}" \
      | scrub_prompt_fixture "$SID" "$REPO" "$HOME" > "$fixtures/$case.json"
  fi
  echo "fixture -> $fixtures/$case.json" >&2
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  prompt_main "$@"
fi
