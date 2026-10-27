#!/usr/bin/env bash
# Runs every hook command in .claude/settings.json with a PATH that has every tool but `python3`, and expects a silent exit 0: a missing interpreter must make a hook a no-op, not an
# error after every tool call.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
settings="$ROOT/.claude/settings.json"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/bin"
# A PATH of symlinks to every tool on the current PATH except python3.
IFS=: read -ra dirs <<<"$PATH"
for d in "${dirs[@]}"; do
  for f in "$d"/*; do
    n="${f##*/}"
    [ -x "$f" ] && [ ! -d "$f" ] && [ "$n" != python3 ] && [ ! -e "$tmp/bin/$n" ] && ln -s "$f" "$tmp/bin/$n"
  done
done

failures=0
cases=0
while IFS= read -r cmd; do
  cases=$((cases + 1))
  err="$tmp/err"
  PATH="$tmp/bin" CLAUDE_PROJECT_DIR="$ROOT" bash -c "$cmd" </dev/null >/dev/null 2>"$err"
  rc=$?
  if [ "$rc" -ne 0 ] || [ -s "$err" ]; then
    failures=$((failures + 1))
    echo "FAIL: exit $rc, stderr: $(cat "$err")"
    echo "  hook: $cmd"
  fi
done < <(jq -r '.hooks[][].hooks[] | select(.type == "command") | .command' "$settings")

if [ "$cases" -eq 0 ]; then echo "FAIL: no hook commands found"; exit 1; fi
[ "$failures" -eq 0 ] && echo "ok: $cases hook commands are no-ops without python3"
exit "$failures"
