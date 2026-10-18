#!/usr/bin/env bash
# Decides whether a push to `main` repeats a pull request run that already proved the same tree (#959).
#
#   REPO=owner/name SHA=<pushed tip> pr-proven.sh
#
# Writes `pr_proven=true|false` and `pr_proven_reason=<why>` to stdout in GITHUB_OUTPUT form.
#
# `true` only when ALL hold: exactly one merged pull request carries SHA and it was merged into
# `main`, and that PR's head commit has the same git tree as SHA. A rebase-merge of a branch that
# was up to date with `main` (what the "require branches to be up to date" branch protection
# guarantees) yields the PR head's tree; a stale branch yields a new tree no run has tested. Every
# other outcome -- a direct push, several PRs, a tree mismatch, an API error -- is `false`, which
# keeps the full pipeline: there is no input for which "something went wrong" skips a job.
#
# Needs `gh` with GH_TOKEN. Tested by scripts/tests/pr-proven.test.sh.

set -uo pipefail

: "${REPO:?}" "${SHA:?}"

verdict() { # <true|false> <reason>
  printf 'pr_proven=%s\n' "$1"
  printf 'pr_proven_reason=%s\n' "$2"
  exit 0
}

prs="$(gh api "repos/$REPO/commits/$SHA/pulls" \
  --jq '[.[] | select(.merged_at != null and .base.ref == "main")] | map("\(.number) \(.head.sha)") | .[]' 2>/dev/null)" \
  || verdict false "could not list the pull requests of the commit"
[ -n "$prs" ] || verdict false "no merged pull request carries this commit (direct push)"
[ "$(printf '%s\n' "$prs" | wc -l)" -eq 1 ] || verdict false "more than one merged pull request carries this commit"

number="${prs%% *}"
head_sha="${prs#* }"
[ -n "$head_sha" ] && [ "$head_sha" != "$prs" ] || verdict false "pull request #$number has no head commit"

pushed_tree="$(gh api "repos/$REPO/commits/$SHA" --jq '.commit.tree.sha' 2>/dev/null)" || pushed_tree=""
pr_tree="$(gh api "repos/$REPO/commits/$head_sha" --jq '.commit.tree.sha' 2>/dev/null)" || pr_tree=""
[ -n "$pushed_tree" ] && [ -n "$pr_tree" ] || verdict false "could not read the commit trees"

if [ "$pushed_tree" = "$pr_tree" ]; then
  verdict true "tree $pushed_tree equals the head of pull request #$number"
fi
verdict false "tree differs from the head of pull request #$number (the branch was not up to date)"
