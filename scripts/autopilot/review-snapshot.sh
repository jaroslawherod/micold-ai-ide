#!/usr/bin/env bash
# Snapshot the working tree for a review round, and later diff from that snapshot.
#
#   review-snapshot.sh                     print a tree SHA of the working tree: tracked, changed and
#                                          untracked files (.gitignore honoured). Nothing is committed.
#   review-snapshot.sh diff <sha> [args…]  the diff from that snapshot to the working tree now;
#                                          extra args go to `git diff` (e.g. --stat, -- <path>)
#
# Works on a private copy of the index, so the real index, HEAD and stash are untouched.
# Exit 2 with UNKNOWN-SNAPSHOT when <sha> is not a tree in this repo.
set -euo pipefail

idx="$(mktemp)"
trap 'rm -f "$idx"' EXIT
cp "$(git rev-parse --git-path index)" "$idx"
GIT_INDEX_FILE="$idx" git add -A

if [ "${1:-}" = diff ]; then
  sha="${2:?usage: review-snapshot.sh diff <sha> [git diff args]}"
  shift 2
  git cat-file -e "$sha^{tree}" 2>/dev/null || { echo "UNKNOWN-SNAPSHOT $sha" >&2; exit 2; }
  GIT_INDEX_FILE="$idx" git diff --cached "$sha" "$@"
else
  GIT_INDEX_FILE="$idx" git write-tree
fi
