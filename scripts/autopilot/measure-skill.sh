#!/usr/bin/env bash
# What each autopilot role loads, in estimated tokens (bytes / 4: compare runs, not absolutes).
#
#   measure-skill.sh            this worktree
#   measure-skill.sh <git-ref>  <ref> against this worktree, with the delta
#
# Rows:
#   description     the skill's frontmatter description, in every session's context
#   orchestrator    SKILL.md, read by the orchestrator at start
#   unit:<phase>    unit.md plus that phase file, read by each unit at spawn
#   on-demand       references/ and templates/, read only when a step links them
# README.md and tests/ are for humans and are not counted.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 2
skill=.claude/skills/speckit-autopilot
ref=${1:-}
if [ -n "$ref" ] && ! git rev-parse -q --verify "$ref^{commit}" >/dev/null; then
  echo "measure-skill: unknown ref $ref" >&2; exit 2
fi

size() { # <ref|""> <path>: bytes, 0 when missing
  if [ -z "$1" ]; then cat "$2" 2>/dev/null | wc -c; else git show "$1:$2" 2>/dev/null | wc -c; fi
}
files() { # <ref|""> <dir>: markdown files under <dir>
  if [ -z "$1" ]; then find "$2" -name '*.md' 2>/dev/null; else git ls-tree -r --name-only "$1" -- "$2" | grep '\.md$'; fi
}
report() { # <ref|"">: "role tokens" lines
  local r=$1 f unit od=0
  local d; d=$(if [ -z "$r" ]; then cat "$skill/SKILL.md"; else git show "$r:$skill/SKILL.md"; fi 2>/dev/null |
    awk '/^description:/{sub(/^description: */,""); print; exit}' | wc -c)
  echo "description $((d / 4))"
  echo "orchestrator $(( $(size "$r" "$skill/SKILL.md") / 4 ))"
  unit=$(size "$r" "$skill/unit.md")
  for f in $(files "$r" "$skill/phases" | sort); do
    echo "unit:$(basename "$f" .md) $(( (unit + $(size "$r" "$f")) / 4 ))"
  done
  for f in $(files "$r" "$skill/references") $(files "$r" "$skill/templates"); do
    od=$((od + $(size "$r" "$f")))
  done
  echo "on-demand $((od / 4))"
}

if [ -z "$ref" ]; then
  report "" | awk '{printf "%-20s %7d\n", $1, $2}'
else
  join -a1 -a2 -e 0 -o 0,1.2,2.2 <(report "$ref" | sort) <(report "" | sort) |
    awk -v r="$ref" 'BEGIN{printf "%-20s %8s %8s %8s\n", "role", r, "now", "delta"}
      {printf "%-20s %8d %8d %+8d\n", $1, $2, $3, $3-$2}'
fi
