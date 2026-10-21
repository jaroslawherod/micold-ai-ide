#!/usr/bin/env bash
# Record one `claude` stream-json fixture for feature 938 (FR-024, research R11).
#
# Usage: scripts/capture-stream-json.sh [--image <file>] [--interrupt-after-first-delta] <name> "<prompt>"
#            [--resume <id>] [-- <extra claude args>...]
#
# --image <file> (feature 941, P-5): the user message carries <prompt> as a text block, then the
#   file as a base64 image block, its media type sniffed from the file's bytes (PNG, JPEG, GIF, WebP).
# --interrupt-after-first-delta (feature 941, P-4): stdin stays open; after the first text delta on
#   stdout the capture writes the `control_request` `interrupt` line, and closes stdin at the `result`.
#
# In a fresh scratch git repo, with CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1 (feature 944), it runs the
# structured argv
#   claude -p --input-format stream-json --output-format stream-json --verbose
#          --include-partial-messages (--session-id <new uuid> | --resume <id>) [extra args]
# writes <prompt> to stdin as one stream-json user message, and saves stdout verbatim to
# crates/micold-core/tests/fixtures/stream-json/<name>.jsonl, scrubbed of the session uuid, the
# scratch and home paths and account identifiers (fixed placeholders, see scrub_stream_json).
# Prints the session uuid on stderr, so a later `--resume` capture can continue the conversation.
#
# Needs a logged-in `claude`; CI has none and never runs this. Sourced (as
# scripts/tests/capture-stream-json.test.sh does), it only defines the functions.

# Escape a string for use as a literal in a sed extended regular expression.
sed_literal() {
  printf '%s' "$1" | sed -E 's/[]\/$*.^[|+?(){}]/\\&/g'
}

# Scrub stdin to stdout: <session id> → <SESSION_ID>, <cwd> → <CWD>, <home> → <HOME>, e-mail
# addresses and account / organisation identifier values → <ACCOUNT>. The cwd goes first: the
# scratch repo may lie under the home directory, and its name holds the session uuid.
scrub_stream_json() {
  local sid cwd home
  sid="$(sed_literal "$1")"
  cwd="$(sed_literal "$2")"
  home="$(sed_literal "$3")"
  sed -E -e "s/$cwd/<CWD>/g" \
      -e "s/$home/<HOME>/g" \
      -e "s/$sid/<SESSION_ID>/g" \
      -e 's/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/<ACCOUNT>/g' \
      -e 's/"((account|organization)_?([Uu]uid|[Ii]d)|email|emailAddress|email_address)":"[^"]*"/"\1":"<ACCOUNT>"/g'
}

# Scrub stdin to stdout where a line-by-line sed cannot: the CLI streams a tool call's input as
# `input_json_delta` chunks, which can cut the scratch path or the session uuid in two. Each run of
# consecutive chunks of one block is joined, <cwd>, <home> and <session id> are replaced in the
# joined text, and the text is cut again at the original chunk lengths (the last chunk takes the
# rest), so every line stays one event. Lines it does not change pass through byte for byte.
scrub_split_deltas() {
  python3 -I -c '
import json, sys
secrets = [(sys.argv[1], "<CWD>"), (sys.argv[2], "<HOME>"), (sys.argv[3], "<SESSION_ID>")]

def chunk_of(line):
    try:
        value = json.loads(line)
        delta = value["event"]["delta"]
        if value.get("type") == "stream_event" and delta.get("type") == "input_json_delta":
            return value, value["event"].get("index")
    except (ValueError, KeyError, TypeError, AttributeError):
        pass
    return None, None

def flush(run, out):
    joined = "".join(v["event"]["delta"]["partial_json"] for _, v in run)
    scrubbed = joined
    for secret, placeholder in secrets:
        if secret:
            scrubbed = scrubbed.replace(secret, placeholder)
    if scrubbed == joined:
        out.extend(line for line, _ in run)
        return
    at = 0
    for i, (_, value) in enumerate(run):
        size = len(value["event"]["delta"]["partial_json"])
        piece = scrubbed[at:] if i == len(run) - 1 else scrubbed[at:at + size]
        at += len(piece)
        value["event"]["delta"]["partial_json"] = piece
        out.append(json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n")

out, run, run_index = [], [], None
for line in sys.stdin:
    value, index = chunk_of(line)
    if value is None or (run and index != run_index):
        flush(run, out)
        run = []
    if value is None:
        out.append(line)
    else:
        run.append((line, value))
        run_index = index
flush(run, out)
sys.stdout.write("".join(out))
' "$2" "$3" "$1"
}

# The one stdin line that carries <prompt> as a user message (contracts/claude-stream-json.md).
user_message_line() {
  jq -cn --arg text "$1" '{type:"user",message:{role:"user",content:[{type:"text",text:$text}]}}'
}

# The one stdin line that carries <prompt> and the image file <file> as one user message: the text
# block, then the image block (specs/941-structured-prompt-input/contracts/claude-input.md). Fails
# when <file> is not a PNG, JPEG, GIF or WebP by its first bytes.
image_message_line() {
  local media
  media="$(python3 -I -c '
import sys
head = open(sys.argv[1], "rb").read(12)
if head.startswith(b"\x89PNG\r\n\x1a\n"): print("image/png")
elif head.startswith(b"\xff\xd8\xff"): print("image/jpeg")
elif head[:6] in (b"GIF87a", b"GIF89a"): print("image/gif")
elif head[:4] == b"RIFF" and head[8:12] == b"WEBP": print("image/webp")
else: sys.exit(1)
' "$2")" || { echo "not a PNG, JPEG, GIF or WebP file: $2" >&2; return 1; }
  base64 -w0 < "$2" \
    | jq -cR --arg text "$1" --arg media "$media" \
        '{type:"user",message:{role:"user",content:[{type:"text",text:$text},
          {type:"image",source:{type:"base64",media_type:$media,data:.}}]}}'
}

# P-4's interrupt request, exactly as the service writes it (contracts/claude-input.md).
INTERRUPT_LINE='{"type":"control_request","request_id":"micold-1","request":{"subtype":"interrupt"}}'

# Run <command...> with the user line in <line file> on its stdin, kept open: copy its stdout to
# ours line by line, write INTERRUPT_LINE after the first `content_block_delta` text delta, close
# its stdin after the first `result` line, and exit with its status. The line goes through a file,
# not an argument: with a picture it can pass the kernel's limit on one argument.
drive_interrupt() {
  python3 -I -c '
import json, subprocess, sys
line_file, interrupt_line, argv = sys.argv[1], sys.argv[2], sys.argv[3:]
child = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE)

def write(line):
    try:
        child.stdin.write((line + "\n").encode()); child.stdin.flush()
    except BrokenPipeError:
        print("the CLI stopped reading its stdin", file=sys.stderr)

with open(line_file) as f:
    write(f.read().rstrip("\n"))
asked = False
for raw in child.stdout:
    sys.stdout.buffer.write(raw); sys.stdout.buffer.flush()
    try:
        value = json.loads(raw)
    except ValueError:
        continue
    if not isinstance(value, dict):
        continue
    event = value.get("event") if value.get("type") == "stream_event" else None
    delta = event.get("delta") if isinstance(event, dict) else None
    if not asked and isinstance(delta, dict) and event.get("type") == "content_block_delta" \
            and delta.get("type") == "text_delta":
        write(interrupt_line)
        asked = True
    elif value.get("type") == "result" and not child.stdin.closed:
        try:
            child.stdin.close()
        except BrokenPipeError:
            pass
sys.exit(child.wait())
' "$1" "$INTERRUPT_LINE" "${@:2}"
}

capture_main() {
  set -euo pipefail
  local image="" interrupt=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --image) image="${2:?--image needs a file}"; shift 2 ;;
      --interrupt-after-first-delta) interrupt=1; shift ;;
      *) break ;;
    esac
  done
  [ $# -ge 2 ] || { echo "usage: $0 [--image <file>] [--interrupt-after-first-delta] <name> \"<prompt>\" [--resume <id>] [-- <claude args>...]" >&2; exit 2; }
  local name="$1" prompt="$2" line
  if [ -n "$image" ]; then
    line="$(image_message_line "$prompt" "$image")" || exit 1
  else
    line="$(user_message_line "$prompt")"
  fi
  shift 2
  local session_flag="--session-id" sid
  sid="$(cat /proc/sys/kernel/random/uuid 2>/dev/null || uuidgen | tr 'A-Z' 'a-z')"
  if [ "${1:-}" = --resume ]; then
    session_flag="--resume"
    sid="$2"
    shift 2
  fi
  [ "${1:-}" = -- ] && shift

  local root out repo
  root="$(git rev-parse --show-toplevel)"
  out="$root/crates/micold-core/tests/fixtures/stream-json/$name.jsonl"
  # A resumed conversation is looked up by its cwd, so a `--resume` capture reuses the scratch repo
  # the first capture of that session made.
  repo="${TMPDIR:-/tmp}/micold-capture-$sid"
  mkdir -p "$repo"
  repo="$(cd "$repo" && pwd -P)"
  [ -d "$repo/.git" ] || git -C "$repo" init -q
  [ -f "$repo/notes.txt" ] || printf 'The answer is 42.\n' > "$repo/notes.txt"

  mkdir -p "$(dirname "$out")"
  # Forward subagent text and thinking, as the service's structured launch does (feature 944, R5).
  export CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1
  local argv=(claude -p --input-format stream-json --output-format stream-json --verbose
              --include-partial-messages "$session_flag" "$sid" "$@")
  if [ -n "$interrupt" ]; then
    # mktemp makes it 0600; it is removed whether the capture succeeds or not.
    local line_file rc=0
    line_file=$(mktemp "${TMPDIR:-/tmp}/micold-capture-line.XXXXXX")
    printf '%s\n' "$line" > "$line_file"
    (cd "$repo" && drive_interrupt "$line_file" "${argv[@]}") || rc=$?
    rm -f "$line_file"
    [ "$rc" -eq 0 ] || return "$rc"
  else
    printf '%s\n' "$line" | (cd "$repo" && "${argv[@]}")
  fi \
    | scrub_split_deltas "$sid" "$repo" "$HOME" \
    | scrub_stream_json "$sid" "$repo" "$HOME" > "$out" \
    || echo "claude exited non-zero (kept: an error result is a fixture too)" >&2
  echo "session $sid -> $out ($(wc -l < "$out") lines)" >&2
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  capture_main "$@"
fi
