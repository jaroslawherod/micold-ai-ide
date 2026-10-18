#!/usr/bin/env bash
# Drives scripts/pr-proven.sh (#959) against a stand-in `gh` that answers from files, so what is
# checked is the decision, not GitHub. The stand-in applies no --jq: the PR list file already holds
# the "<number> <head sha>" lines the script's filter would produce.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
SCRIPT="$PWD/scripts/pr-proven.sh"
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
# PRS: lines "<number> <head sha>"; tree-<sha> files hold a commit's tree; GH_FAIL makes it exit 1.
[ -z "${GH_FAIL:-}" ] || exit 1
case "$*" in
  "api repos/o/r/commits/tip/pulls"*) cat "$GH_DIR/prs" 2>/dev/null || true ;;
  "api repos/o/r/commits/"*) sha="${2##*/}"; cat "$GH_DIR/tree-$sha" 2>/dev/null || exit 1 ;;
esac
STANDIN
chmod +x "$work/bin/gh"

run() { PATH="$work/bin:$PATH" GH_DIR="$work" REPO=o/r SHA=tip "$SCRIPT"; }
reset() { rm -f "$work"/prs "$work"/tree-*; }
expect() { # <name> <wanted pr_proven>
  local out
  out="$(run)"
  if printf '%s\n' "$out" | grep -qx "pr_proven=$2"; then pass "$1"; else fail "$1" "$out"; fi
}

reset
printf '42 head1\n' >"$work/prs"; echo T1 >"$work/tree-tip"; echo T1 >"$work/tree-head1"
expect "one merged PR whose head tree equals the pushed tree: proven" true

reset
printf '42 head1\n' >"$work/prs"; echo T1 >"$work/tree-tip"; echo T2 >"$work/tree-head1"
expect "tree differs (branch was not up to date): not proven" false

reset
echo T1 >"$work/tree-tip"
expect "no merged PR (direct push): not proven" false

reset
printf '42 head1\n43 head2\n' >"$work/prs"
echo T1 >"$work/tree-tip"; echo T1 >"$work/tree-head1"; echo T1 >"$work/tree-head2"
expect "two PRs carry the commit: not proven" false

reset
printf '42 head1\n' >"$work/prs"; echo T1 >"$work/tree-tip"
expect "the PR head's tree cannot be read: not proven" false

reset
printf '42 head1\n' >"$work/prs"; echo T1 >"$work/tree-head1"
expect "the pushed tree cannot be read: not proven" false

reset
printf '42 head1\n' >"$work/prs"; echo T1 >"$work/tree-tip"; echo T1 >"$work/tree-head1"
if out="$(GH_FAIL=1 run)" && printf '%s\n' "$out" | grep -qx 'pr_proven=false'; then
  pass "the API fails: not proven"
else fail "the API fails" "$out"; fi

[ "$failures" -eq 0 ] || { echo "$failures failed"; exit 1; }
