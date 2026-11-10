#!/usr/bin/env bash
# Sourced by record-verdict.sh and verdict-guard.sh: the ledger's *Review rounds* table and the
# checks both helpers share. bash 3.2, no GNU-only flags.
#
#   vl_vocab_ok <verdict>        first word is CLEAN, MINOR-ONLY, CHANGES or pending
#   vl_approved <verdict>        first word is CLEAN or MINOR-ONLY
#   vl_is_sha <s>                40 lowercase hex characters
#   vl_has_head <ledger>         the Review rounds header has a Head column
#   vl_rounds <ledger>           TSV rows: review round scope head verdict (columns found by name)
#   vl_scopes <ledger> <pr>      the scopes that name <pr>: itself, milestone IDs, `design`
#   vl_required <ledger> <pr>    TSV of the latest round per review whose scope names <pr>
#   vl_find_ledger <branch>      sets VL_LEDGER (readable file) and VL_TMP; returns 0 found,
#                                1 no ledger names the branch, 2 unreadable
#   vl_ledger_diff <a> <b>       0 when the heads differ only in ledger files, 1 when not, 2 error

# The four ledger patterns of ledger-of.sh, as git pathspecs (`*` must not cross `/`).
LEDGER_PATHSPECS=(
  ':(glob)specs/*/autopilot.md'
  ':(glob)specs/*/autopilot-*.md'
  ':(glob)specs/*/bugs/*.autopilot.md'
  ':(glob)specs/quick/*.autopilot.md'
)

vl_first_word() { local w="${1%%[ :]*}"; printf '%s' "$w"; }
vl_vocab_ok() { case "$(vl_first_word "$1")" in CLEAN|MINOR-ONLY|CHANGES|pending) return 0 ;; esac; return 1; }
vl_approved() { case "$(vl_first_word "$1")" in CLEAN|MINOR-ONLY) return 0 ;; esac; return 1; }
vl_is_sha() { [[ "$1" =~ ^[0-9a-f]{40}$ ]]; }

# awk program: prints the rows of the table under `## <section>` as TSV, header row first, cells
# trimmed. A row is a line starting with `|`; the separator row is skipped.
VL_TABLE_AWK='
function trim(s) { gsub(/^[ \t]+|[ \t]+$/, "", s); return s }
/^## / { insec = (substr($0, 4) == sec); next }
insec && /^\|/ {
  line = $0
  if (line ~ /^\|[ \t]*:?-+/) next
  sub(/^\|/, "", line); sub(/\|[ \t]*$/, "", line)
  n = split(line, c, "|"); out = ""
  for (i = 1; i <= n; i++) out = out (i > 1 ? "\t" : "") trim(c[i])
  print out
}'
vl_table() { awk -v sec="$2" "$VL_TABLE_AWK" "$1"; }

vl_has_head() { vl_table "$1" "Review rounds" | head -1 | tr '\t' '\n' | grep -qxF Head; }

vl_rounds() {
  vl_table "$1" "Review rounds" | awk -F '\t' '
    NR == 1 { for (i = 1; i <= NF; i++) col[$i] = i; next }
    { printf "%s\t%s\t%s\t%s\t%s\n", $col["Review"], $col["Round"], $col["Scope"], $col["Head"], $col["Verdict"] }'
}

vl_scopes() {
  local ledger="$1" pr="${2#\#}"
  echo "$pr"
  vl_table "$ledger" Milestones | awk -F '\t' -v pr="$pr" '
    NR == 1 { for (i = 1; i <= NF; i++) col[$i] = i; next }
    { p = $col["PR"]; sub(/^#/, "", p); if (p == pr) print $col["ID"] }'
  vl_table "$ledger" "Pull requests" | awk -F '\t' -v pr="$pr" '
    NR == 1 { for (i = 1; i <= NF; i++) col[$i] = i; next }
    { p = $col["PR"]; sub(/^#/, "", p); if (p == pr && tolower($col["Purpose"]) == "design") print "design" }'
}

vl_required() {
  local scopes; scopes="$(vl_scopes "$1" "$2" | tr '\n' ' ')"
  vl_rounds "$1" | awk -F '\t' -v scopes=" $scopes" '
    index(scopes, " " $3 " ") && $3 != "" {
      if (!($1 in round) || $2 + 0 >= round[$1] + 0) { round[$1] = $2; row[$1] = $0 }
      if (!($1 in seen)) { seen[$1] = 1; order[++k] = $1 }
    }
    END { for (i = 1; i <= k; i++) print row[order[i]] }'
}

vl_cleanup() { [ -n "${VL_TMP:-}" ] && rm -f "$VL_TMP"; return 0; }

vl_find_ledger() {
  local b="$1" line="" path="" wt="" l sha cand
  VL_LEDGER=""; VL_TMP=""
  # The branch's own checkout, when there is one: it may hold rows not pushed yet.
  while IFS= read -r line; do
    case "$line" in
      "worktree "*) path="${line#worktree }" ;;
      "branch refs/heads/$b") wt="$path"; break ;;
    esac
  done < <(git worktree list --porcelain 2>/dev/null)
  if [ -n "$wt" ]; then
    . "$(dirname "${BASH_SOURCE[0]}")/ledger-of.sh"
    l="$(ledger_of "$wt")"
    if [ -n "$l" ] && [ -f "$l" ]; then VL_LEDGER="$l"; return 0; fi
  fi
  git fetch -q origin "$b" 2>/dev/null || return 2
  sha="$(git rev-parse -q --verify FETCH_HEAD 2>/dev/null)" || return 2
  cand="$(git grep -lF -e "- **Worktree branch**: $b" "$sha" -- "${LEDGER_PATHSPECS[@]}" 2>/dev/null)"
  case $? in 0|1) ;; *) return 2 ;; esac
  VL_TMP="$(mktemp "${TMPDIR:-/tmp}/verdict-ledger.XXXXXX")" || return 2
  while IFS= read -r l; do
    [ -n "$l" ] || continue
    git show "$l" >"$VL_TMP" 2>/dev/null || return 2
    if grep -qxF -- "- **Worktree branch**: $b" "$VL_TMP"; then VL_LEDGER="$VL_TMP"; return 0; fi
  done <<<"$cand"
  return 1
}

vl_ledger_diff() {
  local ex=() p
  for p in "${LEDGER_PATHSPECS[@]}"; do ex+=(":(exclude,glob)${p#:(glob)}"); done
  git diff --quiet "$1" "$2" -- . "${ex[@]}" 2>/dev/null
  case $? in 0) return 0 ;; 1) return 1 ;; *) return 2 ;; esac
}
