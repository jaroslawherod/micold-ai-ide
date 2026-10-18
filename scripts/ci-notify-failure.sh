#!/usr/bin/env bash
# Tells somebody that `main` or the nightly run went red (#956).
#
# The macOS and Windows `Test (daemon)` steps run only after the merge, so a platform break
# surfaces on `main`. CI's `ci-complete` job calls this from a step that runs when the gate failed
# on a push or schedule event. Needs `gh` with GH_TOKEN, and `pull-requests: write` plus
# `issues: write` on the calling job.
#
# push:     comment on the merged pull request that carries the commit, mentioning its author.
#           No merged PR (a direct push): fall through to the tracking issue.
# schedule: open the tracking issue, or comment on it when one is already open, mentioning the
#           author of the commit the run tested.
#
# Env: EVENT (push|schedule), REPO (owner/name), SHA, RUN_URL.
# Tested by scripts/tests/ci-notify-failure.test.sh. Auto-revert is deliberately not done here.

set -euo pipefail

: "${EVENT:?}" "${REPO:?}" "${SHA:?}" "${RUN_URL:?}"
case "$EVENT" in
  push | schedule) ;;
  *) echo "ci-notify-failure: not for event '$EVENT'" >&2; exit 2 ;;
esac

TITLE="CI is red on main"

# "@login", or the bare login for a bot, which is not a person to notify.
mention() {
  case "$1" in
    "" | *"[bot]") printf '%s' "${1:-the author}" ;;
    *) printf '@%s' "$1" ;;
  esac
}

if [ "$EVENT" = push ]; then
  pr="$(gh api "repos/$REPO/commits/$SHA/pulls" \
    --jq '[.[] | select(.merged_at != null)] | .[0] | select(. != null) | "\(.number) \(.user.login)"' || true)"
  if [ -n "$pr" ]; then
    number="${pr%% *}"
    author="${pr#* }"
    gh pr comment "$number" --repo "$REPO" --body "$(mention "$author"): CI went red on \`main\` after this merged (commit $SHA). The macOS and Windows daemon suites run only after the merge, so a platform break shows up here. Run: $RUN_URL"
    exit 0
  fi
fi
# The commit's author: a direct push's, or the head of `main` the nightly run tested (#959).
who="$(mention "$(gh api "repos/$REPO/commits/$SHA" --jq '.author.login // empty' || true)")"

body="CI is red on \`main\` ($EVENT, commit $SHA). Notifying $who. Run: $RUN_URL"
existing="$(gh issue list --repo "$REPO" --state open --search "\"$TITLE\" in:title" \
  --json number --jq '.[0].number // empty' || true)"
if [ -n "$existing" ]; then
  gh issue comment "$existing" --repo "$REPO" --body "$body"
else
  gh issue create --repo "$REPO" --title "$TITLE" --body "$body"
fi
