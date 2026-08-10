#!/usr/bin/env bash
# Decides whether a change is documentation-only (feature 023).
#
#   classify-change.sh <base-ref> <head-ref>
#
# Writes `docs_only=` and `reason=` to stdout in GITHUB_OUTPUT form, and any non-documentation
# paths to stderr so a surprising full run explains itself in the job log.
#
# Contract: specs/023-docs-only-ci-skip/contracts/classify-change.md
# Declaration: .gitattributes, attribute `micold-docs` -- this script never hard-codes a path.
#
# Two rules govern everything below:
#
#   1. A change is documentation-only only when EVERY touched path is declared documentation. One
#      path outside the set makes it code-affecting.
#   2. Every failure path lands on `docs_only=false`. There is no input for which "something went
#      wrong" means "skip the build". A misclassification in that direction costs runner minutes;
#      the other direction ships an untested change.
#
# The caller fetches the base ref. On a `pull_request` run, actions/checkout with fetch-depth: 0
# does NOT create `origin/<base>` -- which is why the `assertions` job fetches explicitly even
# though it also uses fetch-depth: 0. This script deliberately does not fetch on the caller's
# behalf: a script that silently repairs its own inputs cannot tell "base missing" from "base
# empty", and that difference decides whether the run falls back to the full pipeline.

set -uo pipefail

verdict() {
  printf 'docs_only=%s\n' "$1"
  printf 'reason=%s\n' "$2"
  exit 0
}

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

# The escape hatch (FR-021). Checked before anything else so a forced run cannot be defeated by a
# base ref that happens to be broken.
case "${FORCE_FULL_CI:-}" in
  1 | true | TRUE | yes) verdict false "forced by full-ci label" ;;
esac

# An all-zero SHA is git's "no such commit": a brand-new branch's push has it as `before`.
zero='0000000000000000000000000000000000000000'
if [ -z "$base" ] || [ "$base" = "$zero" ]; then
  verdict false "base ref unavailable"
fi
if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
  verdict false "base ref unavailable"
fi
if ! git rev-parse --verify --quiet "$head^{commit}" >/dev/null; then
  verdict false "head ref unavailable"
fi
if ! git merge-base "$base" "$head" >/dev/null 2>&1; then
  # Unrelated histories: a force push can leave the old tip with no common ancestor.
  verdict false "no merge base"
fi

# The NUL-separated streams below never pass through a shell variable: `$(...)` cannot hold a NUL
# byte -- bash drops them silently, which collapses the whole path list into one mangled string.
# Files keep the separators intact, and reading the loop from a file rather than a pipe keeps the
# counters in this shell.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Three dots: the diff from the merge base, i.e. everything this change touches, regardless of
# which commit came last (FR-005). -z and quotePath=false keep a path with a space, a quote or a
# non-ASCII byte intact all the way to check-attr.
if ! git -c core.quotePath=false diff --name-only -z "$base...$head" > "$work/changed" 2>/dev/null
then
  verdict false "could not determine changed files"
fi

if [ ! -s "$work/changed" ]; then
  verdict true "no files changed"
fi

# `check-attr --stdin -z` answers for every path in one call. Its output is NUL-separated triples:
# path, attribute, value. Only `set` is documentation -- `unset` and `unspecified` are both code,
# which is what makes "undeclared means code" the default rather than a rule to remember.
if ! git check-attr --stdin -z micold-docs < "$work/changed" > "$work/attrs" 2>/dev/null; then
  verdict false "could not classify changed files"
fi

total=0
docs=0
offenders=""
while IFS= read -r -d '' path && IFS= read -r -d '' _attr && IFS= read -r -d '' value; do
  total=$((total + 1))
  if [ "$value" = "set" ]; then
    docs=$((docs + 1))
  else
    offenders="${offenders}${path}"$'\n'
  fi
done < "$work/attrs"

if [ "$total" -eq 0 ]; then
  # check-attr answered for nothing while git diff listed something: do not guess.
  verdict false "could not classify changed files"
fi

if [ "$docs" -eq "$total" ]; then
  verdict true "$total documentation path(s)"
fi

{
  echo "non-documentation paths (this is why the full pipeline ran):"
  printf '%s' "$offenders" | sed 's/^/  /'
} >&2

verdict false "$((total - docs)) non-documentation path(s) of $total"
