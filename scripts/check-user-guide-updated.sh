#!/usr/bin/env bash
# Blocks a user-facing *feature* that does not touch the user guide.
#
#   check-user-guide-updated.sh <base-ref> <head-ref>
#   CHANGE_TITLE=<the pull request's title>   DOCS_NOT_NEEDED=<the docs-not-needed label>
#
# Constitution Principle VII: "Every user-facing feature MUST ship with corresponding user-guide
# documentation in the same change. A feature is not 'done' until its documentation exists." The
# Documentation gate beside it says the same thing about pull requests. Half of that gate was
# already mechanical -- the docs build runs in CI -- and half was not: whether the guide had in
# fact been updated was left to review, which is the half a reviewer skips when the diff is long
# and the feature works.
#
# Declaration: .gitattributes, attributes `micold-user-facing` and `micold-user-guide` -- this
# script never hard-codes a path, for the same reason `classify-change.sh` does not. The set of
# screens is going to change; the rule is not.
#
# **Features, not every edit.** The word in the principle is "feature", and holding the gate to it
# is what makes it worth having. Measured over the sixty merges before this script existed, firing
# on any edit under the declared paths would have blocked ten changes, eight of them bug fixes
# whose guide page says nothing different (BUG-017's attempt count, BUG-020's mutation, a reworded
# message) -- a gate whose label becomes routine is a gate that has stopped being read. Firing on
# `feat` alone would have blocked two, and both were real omissions: naming the AI CLIs the service
# cannot find, and the Settings icon rail. So the title decides, via conventional-commit prefix.
#
# Nothing lints that prefix today, which is the known weakness: writing `fix` gets past this. It is
# a guard against forgetting, not against intent, and the convention is already load-bearing --
# release-please reads the same prefixes to decide version numbers.
#
# Two rules govern everything below:
#
#   1. **Only the user guide satisfies it.** A developer doc is not a substitute. The reader this
#      principle protects is the person using the application.
#   2. **Every failure path blocks.** There is no input for which "something went wrong" means
#      "the docs are fine". The cost of a wrong block is one label; the cost of a wrong pass is a
#      feature nobody can find out how to use, which is the failure this gate exists to prevent.
#
# The escape hatch is the `docs-not-needed` label, passed in as `DOCS_NOT_NEEDED`. It is checked
# before anything else, so a change somebody has deliberately waived cannot then be blocked by a
# base ref that happens to be broken.
#
# The caller fetches the base ref. On a `pull_request` run, actions/checkout with fetch-depth: 0
# does NOT create `origin/<base>`. This script deliberately does not fetch on the caller's behalf:
# a script that silently repairs its own inputs cannot tell "base missing" from "base empty".

set -uo pipefail

# blocked <reason>; passed <reason>
passed() {
  printf 'user guide: ok — %s\n' "$1"
  exit 0
}
blocked() {
  printf 'user guide: BLOCKED — %s\n' "$1"
  exit 1
}

# The escape hatch (checked first, deliberately).
case "${DOCS_NOT_NEEDED:-}" in
  1 | true | TRUE | yes) passed "waived by the docs-not-needed label" ;;
esac

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <base-ref> <head-ref>" >&2
  exit 2
fi
base="$1"
head="$2"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "not inside a git repository" >&2
  exit 2
fi

# Conventional-commit prefixes, as release-please reads them: `feat:`, `feat(scope):`, and either
# with a `!` breaking-change marker. A title that is not a feature is not this gate's business.
case "${CHANGE_TITLE-}" in
  "")
    # Only the caller can know this, and the caller is a workflow step that always has it. An
    # empty title means the wiring broke, which is a failure path like any other.
    blocked "the change's title was not provided, so it could not be examined"
    ;;
esac
if ! printf '%s' "$CHANGE_TITLE" | grep -Eq '^feat(\([^)]*\))?!?:'; then
  passed "not a feature — ${CHANGE_TITLE}"
fi

advice() {
  cat <<'EOF'

Constitution Principle VII: a user-facing feature is not done until its documentation exists, and
the Documentation gate requires the guide to be updated in the same pull request.

Either update the page in docs/user-guide/ that this change makes wrong, or -- if it genuinely
changes nothing a reader of the guide would notice -- apply the `docs-not-needed` label and push
again. The label is a decision somebody signs, not a way to make the check quiet.
EOF
}

# An all-zero SHA is git's "no such commit": a brand-new branch's push has it as `before`.
zero='0000000000000000000000000000000000000000'
if [ -z "$base" ] || [ "$base" = "$zero" ] \
  || ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
  blocked "base ref unavailable, so the change could not be examined"
fi
if ! git rev-parse --verify --quiet "$head^{commit}" >/dev/null; then
  blocked "head ref unavailable, so the change could not be examined"
fi
if ! git merge-base "$base" "$head" >/dev/null 2>&1; then
  # Unrelated histories: a force push can leave the old tip with no common ancestor.
  blocked "no merge base, so the change could not be examined"
fi

# The NUL-separated streams below never pass through a shell variable: `$(...)` cannot hold a NUL
# byte -- bash drops them silently, which collapses the whole path list into one mangled string.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Three dots: the diff from the merge base, i.e. everything this change touches, regardless of
# which commit came last. Splitting the code and the prose across two commits is normal.
if ! git -c core.quotePath=false diff --name-only -z "$base...$head" > "$work/changed" 2>/dev/null
then
  blocked "could not determine the changed files"
fi

if [ ! -s "$work/changed" ]; then
  passed "no files changed"
fi

# `check-attr --stdin -z` answers for every path in one call, and answers for deleted paths too:
# it is pure pattern matching, so removing a screen counts exactly like editing one.
classify() {
  if ! git check-attr --stdin -z "$1" < "$work/changed" > "$work/$1" 2>/dev/null; then
    blocked "could not classify the changed files"
  fi
}
classify micold-user-facing
classify micold-user-guide

# Only `set` counts. Both `unset` and `unspecified` mean "not this", which is what makes the
# declaration a list of exceptions rather than a rule to remember.
set_paths() {
  while IFS= read -r -d '' path && IFS= read -r -d '' _attr && IFS= read -r -d '' value; do
    [ "$value" = "set" ] && printf '%s\n' "$path"
  done < "$work/$1"
}

facing="$(set_paths micold-user-facing)"
guide="$(set_paths micold-user-guide)"

if [ -z "$facing" ]; then
  passed "no user-facing paths in this change"
fi

facing_count="$(printf '%s\n' "$facing" | grep -c .)"

if [ -n "$guide" ]; then
  guide_count="$(printf '%s\n' "$guide" | grep -c .)"
  passed "$facing_count user-facing path(s), documented in $guide_count user-guide page(s)"
fi

{
  printf 'user guide: BLOCKED — %s user-facing path(s) changed and no page under\n' "$facing_count"
  printf 'docs/user-guide/ was touched:\n\n'
  printf '%s\n' "$facing" | sed 's/^/  /'
  advice
} >&2
exit 1
