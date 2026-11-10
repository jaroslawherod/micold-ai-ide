#!/usr/bin/env bash
# Shared fixtures of the autopilot shell suites (autopilot.test.sh, run-status.test.sh): a `gh` stub
# on PATH, throwaway repositories, ledger writers and the `check` runner. Source it; it sets
# ROOT, S, failures, cases and tmp, and removes tmp on exit.
#
# `gh` answers from JSON fixtures, so no case touches GitHub:
#   pr-<n>.json          what `gh pr view <n>` sees; pr-<n>.json.<k> replaces it from the k-th view on
#   pr-<n>.merged.json   becomes pr-<n>.json when `gh pr merge <n>` succeeds
#   merge-<n>.fail       `gh pr merge <n>` prints it and fails
#   runs.json            what `gh run list` sees

ROOT="$(git rev-parse --show-toplevel)"
S="$ROOT/scripts/autopilot"
failures=0
cases=0
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/bin"
cat > "$tmp/bin/gh" <<'STUB'
#!/usr/bin/env bash
F="$GH_FIXTURES"
q() { local next=0; while [ $# -gt 0 ]; do [ "$1" = -q ] && { shift; echo "$1"; return; }; shift; done; echo .; }
case "$1 $2" in
  "pr view")
    n="$3"; c=$(( $(cat "$F/count-$n" 2>/dev/null || echo 0) + 1 )); echo "$c" > "$F/count-$n"
    f="$F/pr-$n.json"
    for k in $(seq 1 "$c"); do [ -f "$F/pr-$n.json.$k" ] && f="$F/pr-$n.json.$k"; done
    [ -f "$f" ] || { echo "no pull requests found" >&2; exit 1; }
    jq -r "$(q "$@")" "$f" ;;
  "pr merge")
    n="$3"
    if [ -f "$F/merge-$n.fail" ]; then cat "$F/merge-$n.fail" >&2; exit 1; fi
    for k in "$F/pr-$n.json".*; do [ -f "$k" ] && rm -f "$k"; done
    cp "$F/pr-$n.merged.json" "$F/pr-$n.json" ;;
  "pr list") [ -f "$F/pr-list.json" ] && jq -r "$(q "$@")" "$F/pr-list.json" || echo "" ;;
  "run list") jq -r "$(q "$@")" "$F/runs.json" ;;
  "run view") echo "error[E0308]: mismatched types" ;;
  "api user") echo me ;;
  "issue view") jq -r "$(q "$@")" "$F/issue-$3.json" ;;
  "issue create") echo "$*" >> "$F/gh.log"; echo "https://github.com/o/r/issues/42" ;;
  "issue "*|"label create") echo "$*" >> "$F/gh.log" ;;
  *) echo "gh stub: unhandled: $*" >&2; exit 2 ;;
esac
STUB
chmod +x "$tmp/bin/gh"
export PATH="$tmp/bin:$PATH"
unset CLAUDE_PROJECT_DIR  # the hooks fall back to it; a run inside an autopilot session would leak in

# Fresh origin + clone on branch `wt`; echoes the clone path. GH_FIXTURES is per case.
new_repo() {
  local d; d="$(mktemp -d "$tmp/case.XXXX")"
  git init -q --bare -b main "$d/origin.git"
  git clone -q "$d/origin.git" "$d/wt" 2>/dev/null
  (
    cd "$d/wt"
    git config user.email t@example.com; git config user.name t; git config commit.gpgsign false
    git switch -q -c main 2>/dev/null || true
    echo seed > seed.txt; git add -A; git commit -qm seed; git push -q origin main
    git switch -q -c wt
  )
  mkdir -p "$d/fx"
  echo "$d"
}

ledger() {  # ledger <file> <branch> <phase> <pr...>
  local f="$1" b="$2" ph="$3"; shift 3
  mkdir -p "$(dirname "$f")"
  {
    echo "# Autopilot ledger"
    echo "- **Worktree branch**: $b"
    echo "- **Phase**: $ph"
    echo "- **Next step**: open PR 2"
    echo; echo "## Pull requests"; echo "| PR | Purpose |"
    for p in "$@"; do echo "| #$p | x |"; done
    echo; echo "## Decisions"; echo "| D1 | see #999 |"
    echo; echo "## Open escalation"; echo; echo "None."
  } > "$f"
}

pr_json() { # pr_json <fx> <n> <state> [ci-status ci-conclusion] [mergeable]
  local fx="$1" n="$2" st="$3" s="${4:-}" c="${5:-}" m="${6:-MERGEABLE}" roll='[]'
  [ -n "$s" ] && roll="[{\"name\":\"ci complete\",\"status\":\"$s\",\"conclusion\":\"$c\",\"detailsUrl\":\"https://x/actions/runs/77/job/1\"}]"
  [ "$c" = FAILURE ] && roll="[{\"name\":\"build + test\",\"status\":\"COMPLETED\",\"conclusion\":\"FAILURE\",\"detailsUrl\":\"https://x/actions/runs/77/job/2\"},${roll#[}"
  printf '{"state":"%s","mergeable":"%s","headRefName":"wt","headRefOid":"h1","mergeCommit":{"oid":"abc123"},"statusCheckRollup":%s}\n' \
    "$st" "$m" "$roll"
}

# check <name> <want-exit> <want-output-regex> <cmd...>   (runs in $PWD)
check() {
  local name="$1" want="$2" re="$3"; shift 3
  cases=$((cases + 1))
  local out code
  out="$("$@" 2>&1)"; code=$?
  if [ "$code" != "$want" ] || ! grep -qE -- "$re" <<<"$out"; then
    failures=$((failures + 1))
    echo "FAIL $name: exit $code (want $want), output:"; sed 's/^/    /' <<<"$out"
  fi
}
