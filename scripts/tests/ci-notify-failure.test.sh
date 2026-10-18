#!/usr/bin/env bash
# Drives scripts/ci-notify-failure.sh (#956) against a stand-in `gh` that records its calls and
# answers the two lookups from files, so what is checked is the decision, not GitHub.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
SCRIPT="$PWD/scripts/ci-notify-failure.sh"
failures=0
pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin"
cat >"$work/bin/gh" <<'STANDIN'
#!/usr/bin/env bash
# Records every call. Lookups: the commit's merged PR ("<number> <login>"), the commit author's
# login, and the open tracking issue's number, each from a file the case may leave absent.
echo "gh $*" >>"$GH_LOG"
case "$*" in
  "api repos/o/r/commits/abc/pulls"*) cat "$GH_PR" 2>/dev/null || true ;;
  "api repos/o/r/commits/abc "*) cat "$GH_AUTHOR" 2>/dev/null || true ;;
  "issue list"*) cat "$GH_ISSUE" 2>/dev/null || true ;;
esac
STANDIN
chmod +x "$work/bin/gh"

run() { # run <event>; resets the log and the lookups
  : >"$work/log"
  EVENT="$1" REPO=o/r SHA=abc RUN_URL=https://x/run/1 PATH="$work/bin:$PATH" \
    GH_LOG="$work/log" GH_PR="$work/pr" GH_AUTHOR="$work/author" GH_ISSUE="$work/issue" \
    "$SCRIPT" >"$work/out" 2>&1
}
reset() { rm -f "$work/pr" "$work/author" "$work/issue"; }

reset
echo "42 alice" >"$work/pr"
run push
if grep -q '^gh pr comment 42 ' "$work/log" && grep -q '@alice' "$work/log" \
  && grep -q 'https://x/run/1' "$work/log" && ! grep -q 'issue create' "$work/log"; then
  pass "push: comments on the merged PR and mentions its author"
else fail "push: comments on the merged PR" "$(cat "$work/log")"; fi

reset
echo "7 dependabot[bot]" >"$work/pr"
run push
if grep -q '^gh pr comment 7 ' "$work/log" && ! grep -q '@dependabot' "$work/log"; then
  pass "push: a bot author is not mentioned"
else fail "push: a bot author is not mentioned" "$(cat "$work/log")"; fi

reset
echo bob >"$work/author"
run push
if grep -q '^gh issue create ' "$work/log" && grep -q '@bob' "$work/log" \
  && ! grep -q 'pr comment' "$work/log"; then
  pass "push with no merged PR: opens a tracking issue mentioning the commit author"
else fail "push with no merged PR" "$(cat "$work/log")"; fi

reset
echo carol >"$work/author"
run schedule
if grep -q '^gh issue create ' "$work/log" && grep -q '@carol' "$work/log" \
  && ! grep -q 'pr comment' "$work/log"; then
  pass "schedule, no open tracking issue: creates one mentioning the commit author"
else fail "schedule, no open tracking issue" "$(cat "$work/log")"; fi

reset
echo 99 >"$work/issue"
echo carol >"$work/author"
run schedule
if grep -q '^gh issue comment 99 ' "$work/log" && grep -q '@carol' "$work/log" \
  && ! grep -q 'issue create' "$work/log"; then
  pass "schedule, open tracking issue: comments on it instead of opening another"
else fail "schedule, open tracking issue" "$(cat "$work/log")"; fi

if ! EVENT=pull_request REPO=o/r SHA=abc RUN_URL=u PATH="$work/bin:$PATH" GH_LOG="$work/log" \
  "$SCRIPT" >/dev/null 2>&1; then
  pass "pull_request: refuses (the workflow never calls it there)"
else fail "pull_request: refuses"; fi

[ "$failures" -eq 0 ] || { echo "$failures failed"; exit 1; }
