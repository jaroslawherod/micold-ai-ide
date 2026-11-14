#!/usr/bin/env bash
# Classify a PR's diff: autopilot tooling only, or anything else.
#
# Usage: diff-class.sh <pr> [<head-sha>]
# With <head-sha> (fetched locally) the list also includes `git diff --no-renames origin/main...<head>`,
# so the old path of a rename (e.g. crates/ moved into docs/) counts; an unreadable local diff is PRODUCT.
# Tooling paths: .claude/skills/**, .claude/agents/**, scripts/autopilot/**, the autopilot shell
# suites (scripts/tests/autopilot*, scripts/tests/run-status*, scripts/tests/lib/autopilot*),
# docs/**, specs/**. Everything else (crates/, product code, CI config, other scripts) is product.
# Fails closed: an unreadable, empty or truncated (100+ files) file list is PRODUCT.
#
# Last line, and exit code:
#   TOOLING <pr>                 0  every changed file is autopilot tooling
#   PRODUCT <pr> <reason>        1  keeps the full CI requirement
set -uo pipefail
pr="${1:?usage: diff-class.sh <pr> [<head-sha>]}"; pr="${pr#\#}"; head="${2:-}"

files="$(gh pr view "$pr" --json files -q '.files[].path' 2>/dev/null)" || { echo "PRODUCT $pr file list unreadable"; exit 1; }
if [ -n "$head" ]; then
  local_files="$(git diff --name-only --no-renames "origin/main...$head" 2>/dev/null)" || { echo "PRODUCT $pr local diff unreadable"; exit 1; }
  files="$(printf '%s\n%s\n' "$files" "$local_files" | sed '/^$/d' | sort -u)"
fi
[ -n "$files" ] || { echo "PRODUCT $pr no changed files listed"; exit 1; }
[ "$(wc -l <<<"$files")" -lt 100 ] || { echo "PRODUCT $pr 100 or more files (list may be truncated)"; exit 1; }

while IFS= read -r f; do
  case "$f" in
    *..*) echo "PRODUCT $pr $f"; exit 1 ;;
    .claude/skills/*|.claude/agents/*|scripts/autopilot/*|docs/*|specs/*) ;;
    scripts/tests/autopilot*|scripts/tests/run-status*|scripts/tests/lib/autopilot*) ;;
    *) echo "PRODUCT $pr $f"; exit 1 ;;
  esac
done <<<"$files"
echo "TOOLING $pr"
