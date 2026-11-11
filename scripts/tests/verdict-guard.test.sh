#!/usr/bin/env bash
# Drives record-verdict.sh, verdict-guard.sh and wait-merge.sh against throwaway repositories.
#
# Each case builds a bare `origin` and a clone on branch `wt` holding a ledger. `gh` is a stub on
# PATH that answers from JSON fixtures, so no case touches GitHub:
#   pr-<n>.json          what `gh pr view <n>` sees; a missing file makes `gh pr view` fail
#   pr-<n>.merged.json   becomes pr-<n>.json when `gh pr merge <n>` succeeds
#   merge.log            the arguments of every `gh pr merge`
#   comments.log         the body of every `gh pr comment` (it also lands in pr-<n>.json)
#   comment-fail         when present, `gh pr comment` fails

set -uo pipefail

. "$(git rev-parse --show-toplevel)/scripts/tests/lib/autopilot-fixtures.sh"  # ROOT, S, tmp, check, PATH
# This suite needs more of `gh` (comments, a merge log) and a ledger-bearing repo: its own `gh` stub,
# `new_repo` and `pr_json` below replace the shared ones.
cat > "$tmp/bin/gh" <<'STUB'
#!/usr/bin/env bash
F="$GH_FIXTURES"
q() { while [ $# -gt 0 ]; do [ "$1" = -q ] && { shift; echo "$1"; return; }; shift; done; echo .; }
case "$1 $2" in
  "pr view")
    f="$F/pr-$3.json"
    [ -f "$f" ] || { echo "no pull requests found" >&2; exit 1; }
    jq -r "$(q "$@")" "$f" ;;
  "pr merge")
    echo "$*" >> "$F/merge.log"
    cp "$F/pr-$3.merged.json" "$F/pr-$3.json" ;;
  "pr comment")
    [ -f "$F/comment-fail" ] && exit 1
    b="$(while [ $# -gt 0 ]; do [ "$1" = --body ] && { echo "$2"; break; }; shift; done)"
    echo "$b" >> "$F/comments.log"
    f="$F/pr-$3.json"; jq --arg b "$b" '.comments += [{body:$b}]' "$f" > "$f.new" && mv "$f.new" "$f" ;;
  "run list") echo "" ;;
  *) echo "gh stub: unhandled: $*" >&2; exit 2 ;;
esac
STUB
chmod +x "$tmp/bin/gh"
unset AUTOPILOT_MAX_WAIT
export AUTOPILOT_POLL=0 AUTOPILOT_LOG_DIR="$tmp"

# Fresh origin + clone on branch `wt` holding a code commit and a ledger; echoes the clone path.
# The ledger has the new Scope/Head columns, a Pull requests table and a Milestones table.
new_repo() {
  local d; d="$(mktemp -d "$tmp/case.XXXX")"
  git init -q --bare -b main "$d/origin.git"
  git clone -q "$d/origin.git" "$d/wt" 2>/dev/null
  mkdir -p "$d/fx"
  (
    cd "$d/wt"
    git config user.email t@example.com; git config user.name t; git config commit.gpgsign false
    git switch -q -c main 2>/dev/null || true
    echo seed > seed.txt; git add -A; git commit -qm seed; git push -q origin main
    git switch -q -c wt
    mkdir -p specs/042-x/bugs
    write_ledger specs/042-x/autopilot.md wt
    echo bug > specs/042-x/bugs/b.autopilot.md
    echo code > code.txt; git add -A; git commit -qm code; git push -q origin wt
  )
  echo "$d"
}

write_ledger() { # write_ledger <file> <branch> [legacy]
  {
    echo "# Autopilot ledger"
    echo "- **Worktree branch**: $2"
    echo; echo "## Pull requests"; echo
    echo "| PR | Purpose | Status |"; echo "|---|---|---|"; echo "| #7 | Design | open |"; echo "| #8 | Milestone M1 | open |"
    echo; echo "## Milestones"; echo
    echo "| ID | Tasks | PR |"; echo "|---|---|---|"; echo "| M1 | T001 | #8 |"
    echo; echo "## Review rounds"; echo
    if [ "${3:-}" = legacy ]; then
      echo "| Review | Round | Snapshot | Verdict |"; echo "|---|---|---|---|"; echo "| Spec | 1 | t:h | CLEAN |"
    else
      echo "| Review | Round | Scope | Head | Snapshot | Verdict |"; echo "|---|---|---|---|---|---|"
    fi
    echo; echo "## Declined review findings"; echo; echo "None."
  } > "$1"
}

pr_json() { # pr_json <fx> <n> <head-sha> [branch] [state]
  printf '{"state":"%s","mergeable":"MERGEABLE","headRefName":"%s","baseRefName":"main","comments":[],"headRefOid":"%s","mergeCommit":{"oid":"abc123"},"statusCheckRollup":[{"name":"ci complete","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://x/runs/7"}]}\n' \
    "${5:-OPEN}" "${4:-wt}" "$3" > "$1/pr-$2.json"
}
head_of() { git rev-parse HEAD; }
record() { "$S/record-verdict.sh" specs/042-x/autopilot.md "${1:-Review B M1}" "${2:-1}" "${3:-8}" snap "${4:-$(head_of)}" "${5:-CLEAN}"; }
commit_all() { git add -A; git commit -qm "$1"; git push -q origin wt; }
ledger_text() { cat specs/042-x/autopilot.md; }

# --- record-verdict.sh -------------------------------------------------------------------------
d="$(new_repo)"; cd "$d/wt"; H="$(head_of)"
check "record appends a row with scope and the full head" 0 '' record
check "record wrote Scope and 40-char Head" 0 "^\| Review B M1 \| 1 \| 8 \| $H \| snap \| CLEAN \|" grep "Review B M1" specs/042-x/autopilot.md
check "record refuses a short head" 2 'head must be 40 hex' "$S/record-verdict.sh" specs/042-x/autopilot.md R 1 8 snap abc123 CLEAN
check "record refuses an uppercase head" 2 'head must be 40 hex' "$S/record-verdict.sh" specs/042-x/autopilot.md R 1 8 snap "$(tr a-f A-F <<<"$H")" CLEAN
check "record refuses an unknown verdict word" 2 'verdict must start with' "$S/record-verdict.sh" specs/042-x/autopilot.md R 1 8 snap "$H" LGTM
check "record refuses a missing ledger" 2 'no ledger' "$S/record-verdict.sh" nope.md R 1 8 snap "$H" CLEAN
printf '# L\n\n## Other\n' > nosec.md
check "record refuses a ledger without Review rounds" 2 "no '## Review rounds' section" "$S/record-verdict.sh" nosec.md R 1 8 snap "$H" CLEAN
check "record keeps free text after the verdict word" 0 '' record "Review A M1" 1 8 "$H" "MINOR-ONLY: 2 MINOR"
check "record put the free text in the row" 0 'MINOR-ONLY: 2 MINOR \|$' grep "Review A M1" specs/042-x/autopilot.md
check "record keeps the rest of the ledger" 0 '^## Declined review findings' grep Declined specs/042-x/autopilot.md

d="$(new_repo)"; cd "$d/wt"; H="$(head_of)"; write_ledger specs/042-x/autopilot.md wt legacy
check "record upgrades a legacy header" 0 '' record
check "record: new header has Scope and Head" 0 '^\| Review \| Round \| Scope \| Head \| Snapshot \| Verdict \|' grep '^| Review |' specs/042-x/autopilot.md
check "record: old row keeps empty Scope and Head" 0 '^\| Spec \| 1 \|  \|  \| t:h \| CLEAN \|' grep '^| Spec' specs/042-x/autopilot.md

# --- verdict-guard.sh --------------------------------------------------------------------------
guard() { "$S/verdict-guard.sh" "$@"; }
setup_pr() { # setup_pr: repo with an approved verdict for PR 8's head; sets d, H, fx
  d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; H="$(head_of)"
  record >/dev/null; commit_all ledger; P0="$(head_of)"
}
pr_json_at() { pr_json "$d/fx" 8 "$1" "${2:-wt}"; }

setup_pr; pr_json_at "$P0"
check "guard allows the head the verdict was recorded for" 0 "^ALLOWED 8 $P0$" guard 8
# the verdict row names H (the code commit); the ledger commit on top differs only in a ledger file
echo more > code.txt; commit_all "change code"; P1="$(head_of)"; pr_json_at "$P1"
check "guard refuses a stale head naming both SHAs" 7 "^VERDICT-REFUSED 8 reviewed $H current $P1 re-review: " guard 8

setup_pr; echo "x" >> specs/042-x/autopilot.md; echo bug2 >> specs/042-x/bugs/b.autopilot.md; commit_all "ledger only"; pr_json_at "$(head_of)"
check "guard allows a ledger-only difference (autopilot and bugs ledgers)" 0 "^ALLOWED 8 $(head_of)$" guard 8

setup_pr; echo "# note" >> specs/042-x/autopilot.md; echo "x" > specs/042-x/notes.md; commit_all "non-ledger file under specs"; pr_json_at "$(head_of)"
check "guard refuses a changed non-ledger file under specs/" 7 're-review: .*head changed' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; pr_json_at "$(head_of)"
check "guard refuses a PR with no recorded verdict" 7 "reviewed none current $(head_of) re-review: no verdict" guard 8

for v in CHANGES pending; do
  d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; record "Review B M1" 1 8 "$(head_of)" "$v" >/dev/null; pr_json_at "$(head_of)"
  check "guard refuses a $v verdict on the same head" 7 "re-review: Review B M1 round 1 is not approved \($v\)" guard 8
done

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
record "Review B M1" 1 8 "$(head_of)" CHANGES >/dev/null; record "Review B M1" 2 8 "$(head_of)" CLEAN >/dev/null; pr_json_at "$(head_of)"
check "guard takes the latest round per review" 0 '^ALLOWED 8 ' guard 8
record "Review A M1" 1 8 "$(head_of)" CHANGES >/dev/null
check "guard refuses when any required review is not approved" 7 'Review A M1 round 1 is not approved' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; write_ledger specs/042-x/autopilot.md wt legacy; commit_all legacy; pr_json_at "$(head_of)"
check "guard refuses a legacy ledger without a Head column" 7 're-review: ledger has no Head column' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; git push -q origin wt:other; pr_json_at "$(head_of)" other
check "guard leaves a PR whose branch no ledger names unguarded" 0 '^UNGUARDED 8 no ledger$' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
check "guard refuses when gh fails" 7 'reviewed none current unknown re-review: PR head unreadable' guard 99

setup_pr; pr_json_at "not-a-sha"
check "guard refuses a head that is not a SHA" 7 '^VERDICT-REFUSED 8 reviewed none current unknown re-review: PR head unreadable$' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; record >/dev/null; commit_all ledger; pr_json_at "$(head_of)"
git switch -q main; git remote set-url origin "$d/missing.git"
check "guard refuses when the ledger cannot be fetched" 7 're-review: ledger unreadable' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; record >/dev/null; commit_all ledger; pr_json_at "$(head_of)"
git switch -q main
check "guard reads the pushed ledger when the branch has no checkout" 0 "^ALLOWED 8 " guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; record "Design review" 1 design "$(head_of)" CLEAN >/dev/null; commit_all ledger; pr_json "$d/fx" 7 "$(head_of)"
check "guard maps the design scope to the design PR" 0 '^ALLOWED 7 ' guard 7
pr_json "$d/fx" 8 "$(head_of)"
check "guard does not take the design verdict for the milestone PR" 7 'no verdict recorded' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; record "Review B M1" 1 M1 "$(head_of)" CLEAN >/dev/null; commit_all ledger; pr_json "$d/fx" 8 "$(head_of)"
check "guard maps a milestone ID through the Milestones table" 0 '^ALLOWED 8 ' guard 8
pr_json "$d/fx" 7 "$(head_of)"
check "guard does not take a milestone verdict for another PR" 7 'no verdict recorded' guard 7

# --- carry-forward -------------------------------------------------------------------------------
# replay <fx-head-var> : advance main by a commit on `origin`, rebase wt onto it, force-push.
move_base() {
  git switch -q main; echo "base$RANDOM" > "base$RANDOM.txt"; git add -A; git commit -qm base; git push -q origin main
  git switch -q wt; git rebase -q main >/dev/null 2>&1; git push -q -f origin wt
}
comments() { [ -f "$d/fx/comments.log" ] && wc -l < "$d/fx/comments.log" || echo 0; }

setup_pr; move_base; P1="$(head_of)"; pr_json_at "$P1"
check "carry: clean replay onto a moved base is carried" 0 "^CARRIED 8 $H $P1$" guard 8
check "carry: posts one PR comment naming H and P" 0 "^verdict carried $H -> $P1$" cat "$d/fx/comments.log"
check "carry: second run does not post the comment again" 0 "^CARRIED 8 $H $P1$" guard 8
check "carry: one comment after two runs" 0 '^1$' comments
echo extra > extra.txt; commit_all extra; P2="$(head_of)"; pr_json_at "$P2"
check "carry: a changed patch after a carry is refused" 7 "^VERDICT-REFUSED 8 reviewed $H current $P2 re-review: .*head changed" guard 8

setup_pr; move_base; move_base; P1="$(head_of)"; pr_json_at "$P1"
check "carry: chained replay carries from the original H" 0 "^CARRIED 8 $H $P1$" guard 8

setup_pr; git commit -q --amend -m "reworded"; git push -q -f origin wt; pr_json_at "$(head_of)"
check "carry: amended message only is allowed (same tree, no carry)" 0 "^ALLOWED 8 $(head_of)$" guard 8
check "carry: an amended message posts no comment" 0 '^0$' comments

# A rename is one change of two paths; the replay must keep it (F3).
setup_pr; seq 1 40 > old.txt; commit_all "add old"; git mv old.txt new.txt; commit_all "rename"; H="$(head_of)"
record "Review B M1" 2 8 "$H" CLEAN >/dev/null; commit_all ledger2; move_base; pr_json_at "$(head_of)"
check "carry: a pure rename replayed onto a moved base is carried" 0 '^CARRIED 8 ' guard 8

# vl_identity itself: a rename is one status line (old and new path, no score) and the patch-ids are
# git's stable ones (F3). Each case
# prints the identity of the work on `wt` over `main`.
identity() { ( . "$S/verdict-lib.sh"; vl_identity "$1" "$2" ); }
export -f identity; export S
d="$(new_repo)"; cd "$d/wt"; git config diff.renames false  # so only an explicit -M detects the rename
seq 1 40 > old.txt; printf 'a\n' > a.txt; printf 'b\n' > b.txt; git add -A; git commit -qm base2; git push -q origin wt; git branch -f main wt
git mv old.txt new.txt; sed -i '5s/.*/five/' new.txt; echo a2 >> a.txt; echo b2 >> b.txt; git add -A; git commit -qm "rename with an edit, touch a and b"
check "identity lists a rename as one status line with its two paths, score stripped" 0 '^R'$'\t''old\.txt'$'\t''new\.txt$' identity wt main
check "identity has no delete/add pair for a rename" 0 '^none$' bash -c '! identity wt main | grep -qE "^(D|A)\s(old|new)\.txt$" && echo none'
stable_id="$(git diff-tree -p --root --no-commit-id wt | git patch-id --stable | cut -d' ' -f1)"
check "identity's patch-id is git's stable patch-id" 0 "^$stable_id\$" identity wt main
cd "$ROOT"

setup_pr; git commit -q --allow-empty -m "empty"; git push -q origin wt; move_base; pr_json_at "$(head_of)"
check "carry: an empty commit in the replay is invisible" 0 '^CARRIED 8 ' guard 8

setup_pr; move_base; echo "code" > code.txt; echo changed >> code.txt; commit_all "alter"; pr_json_at "$(head_of)"
check "carry: a changed hunk is refused" 7 're-review: .*head changed' guard 8
check "carry: a changed hunk posts no comment" 0 '^0$' comments

setup_pr; git switch -q main; echo other > code.txt; git add -A; git commit -qm "main code"; git push -q origin main; git switch -q wt
git rebase main >/dev/null 2>&1; echo merged > code.txt; git add code.txt; GIT_EDITOR=true git rebase --continue >/dev/null 2>&1; git push -q -f origin wt; pr_json_at "$(head_of)"
check "carry: a conflict resolution is refused" 7 're-review: .*head changed' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; H="$(head_of)"; record "Review B M1" 1 8 "$H" CHANGES >/dev/null; commit_all ledger; move_base; pr_json_at "$(head_of)"
check "carry: a CHANGES verdict is not carried" 7 're-review: Review B M1 round 1 is not approved \(CHANGES\)' guard 8

setup_pr; git switch -q main; echo b > b.txt; git add -A; git commit -qm base; git push -q origin main; git switch -q wt
git merge -q --no-ff main -m "merge main" >/dev/null 2>&1; git push -q origin wt; pr_json_at "$(head_of)"
check "carry: a merge commit in the range is refused" 7 're-review: .*merge commit' guard 8

setup_pr; cp specs/042-x/autopilot.md "$d/led.md"; git reset -q --hard main; mkdir -p specs/042-x; cp "$d/led.md" specs/042-x/autopilot.md; git add -A; git commit -qm "ledger only"; git push -q -f origin wt; pr_json_at "$(head_of)"
check "carry: an empty (ledger-only) range is refused" 7 're-review: .*no change' guard 8

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; record "Review B M1" 1 8 "$(printf '%040d' 1)" CLEAN >/dev/null; commit_all ledger; pr_json_at "$(head_of)"
check "carry: an unreachable reviewed head is refused" 7 're-review: .*unreachable' guard 8

setup_pr; move_base; pr_json_at "$(head_of)"; sed -i 's/"baseRefName":"main"/"baseRefName":"no-such-base"/' "$d/fx/pr-8.json"
check "carry: refuses when the base cannot be fetched" 7 're-review: .*base unreadable' guard 8

setup_pr; move_base; pr_json_at "$(head_of)"; touch "$d/fx/comment-fail"
check "carry: refuses when the comment cannot be posted" 7 're-review: .*comment not posted' guard 8

# --- wait-merge.sh -----------------------------------------------------------------------------
wm() { "$S/wait-merge.sh" "$@"; }
merged_json() { sed 's/"state":"OPEN"/"state":"MERGED"/' "$d/fx/pr-8.json" > "$d/fx/pr-8.merged.json"; }

setup_pr; echo more > code.txt; commit_all "change code"; pr_json_at "$(head_of)"; merged_json
check "wait-merge refuses a stale verdict with exit 7" 7 '^VERDICT-REFUSED 8 reviewed .* re-review: ' wm 8
check "wait-merge issued no merge for a stale verdict" 1 '' test -f "$d/fx/merge.log"
check "wait-merge --squash refuses a stale verdict too" 7 '^VERDICT-REFUSED 8 ' wm 8 --squash
check "wait-merge --no-merge skips the guard" 0 '^GREEN 8$' wm 8 --no-merge

setup_pr; pr_json_at "$P0"; merged_json
check "wait-merge --squash merges a verified head" 0 '^MERGED 8 abc123$' wm 8 --squash
check "wait-merge --squash passes --squash and the head" 0 "pr merge 8 --squash --match-head-commit $P0\$" cat "$d/fx/merge.log"

setup_pr; pr_json_at "$P0"; merged_json
check "wait-merge rebases a verified head" 0 '^MERGED 8 abc123$' wm 8
check "wait-merge rebase passes --match-head-commit" 0 "pr merge 8 --rebase --match-head-commit $P0\$" cat "$d/fx/merge.log"

setup_pr; move_base; pr_json_at "$(head_of)"; merged_json
check "wait-merge merges a carried verdict pinned to the new head" 0 '^MERGED 8 abc123$' wm 8
check "wait-merge pins a carried merge to the new head" 0 "--match-head-commit $(head_of)\$" cat "$d/fx/merge.log"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"; git push -q origin wt:other; pr_json_at "$(head_of)" other; merged_json
check "wait-merge merges an unguarded PR pinned to its head" 0 '^MERGED 8 abc123$' wm 8
check "wait-merge pins an unguarded merge to the head" 0 "--match-head-commit $(head_of)\$" cat "$d/fx/merge.log"

# A guard that dies, or answers with no head, must refuse the merge: run wait-merge beside a stub guard.
setup_pr; pr_json_at "$P0"; merged_json; cp -R "$S" "$d/s2"
printf '#!/usr/bin/env bash\necho boom\nexit 3\n' > "$d/s2/verdict-guard.sh"
check "wait-merge refuses with 'guard failed' when the guard dies" 7 '^VERDICT-REFUSED 8 reviewed none current unknown re-review: guard failed: boom$' "$d/s2/wait-merge.sh" 8
printf '#!/usr/bin/env bash\necho WEIRD\n' > "$d/s2/verdict-guard.sh"
check "wait-merge refuses when the guard names no head" 7 'guard gave no head \(WEIRD\)$' "$d/s2/wait-merge.sh" 8
check "wait-merge issued no merge when the guard failed" 1 '' test -f "$d/fx/merge.log"

echo "verdict-guard.test.sh: $cases cases, $failures failed"
[ "$failures" -eq 0 ]
