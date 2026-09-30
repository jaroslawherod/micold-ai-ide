#!/usr/bin/env bash
# Snapshot the working tree for a review round, and later diff from that snapshot.
#
#   review-snapshot.sh                      print <tree>:<head>: a tree of the working tree (tracked,
#                                           changed and untracked files, .gitignore honoured) and the
#                                           commit it sat on. Nothing is committed.
#   review-snapshot.sh diff <snap> [args…]  the diff from that snapshot to the working tree now;
#                                           extra args go to `git diff` (e.g. --stat, -- <path>)
#
# Works on a private copy of the index, so the real index, HEAD and stash are untouched.
# Exit 2 with UNKNOWN-SNAPSHOT when <snap> is not a snapshot of this repo, or STALE-SNAPSHOT when
# its commit is no longer under HEAD (a rebase): the diff would show upstream changes as fixes.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

idx="$(mktemp)"
trap 'rm -f "$idx"' EXIT
real="$(git rev-parse --path-format=absolute --git-path index)"
if [ -f "$real" ]; then cp "$real" "$idx"; else rm -f "$idx"; fi
GIT_INDEX_FILE="$idx" git add -A

if [ "${1:-}" = diff ]; then
  snap="${2:?usage: review-snapshot.sh diff <tree>:<head> [git diff args]}"
  shift 2
  tree="${snap%%:*}" head="${snap#*:}"
  if [ "$tree" = "$snap" ] || ! git cat-file -e "$tree^{tree}" 2>/dev/null \
    || ! git cat-file -e "$head^{commit}" 2>/dev/null; then
    echo "UNKNOWN-SNAPSHOT $snap" >&2; exit 2
  fi
  git merge-base --is-ancestor "$head" HEAD || { echo "STALE-SNAPSHOT $snap" >&2; exit 2; }
  GIT_INDEX_FILE="$idx" git diff --cached "$tree" "$@"
else
  echo "$(GIT_INDEX_FILE="$idx" git write-tree):$(git rev-parse HEAD)"
fi
