#!/usr/bin/env bash
# Record one `claude` stream-json fixture for feature 938 (FR-024, research R11).
#
# Usage: scripts/capture-stream-json.sh <name> "<prompt>" [--resume <id>] [-- <extra claude args>...]
#
# In a fresh scratch git repo it runs the structured argv
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

# The one stdin line that carries <prompt> as a user message (contracts/claude-stream-json.md).
user_message_line() {
  jq -cn --arg text "$1" '{type:"user",message:{role:"user",content:[{type:"text",text:$text}]}}'
}

capture_main() {
  set -euo pipefail
  [ $# -ge 2 ] || { echo "usage: $0 <name> \"<prompt>\" [--resume <id>] [-- <claude args>...]" >&2; exit 2; }
  local name="$1" prompt="$2"
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
  user_message_line "$prompt" \
    | (cd "$repo" && claude -p --input-format stream-json --output-format stream-json --verbose \
         --include-partial-messages "$session_flag" "$sid" "$@") \
    | scrub_stream_json "$sid" "$repo" "$HOME" > "$out" \
    || echo "claude exited non-zero (kept: an error result is a fixture too)" >&2
  echo "session $sid -> $out ($(wc -l < "$out") lines)" >&2
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  capture_main "$@"
fi
