#!/usr/bin/env bash
# The content hash of `packaging/basic/` (feature 806, contracts/basic-image.md *Content hash*).
#
# Prints the first 16 hex of a sha256 over every file's name (relative to the directory) and
# contents, in `LC_ALL=C` name order, so the value depends on the files alone. It is the tag
# `.github/workflows/basic-image.yml` publishes the basic image under, and the value pinned as
# `BASIC_IMAGE_CONTENT_HASH` in `crates/micold-core/src/sandbox/basic_image.rs`.
#
#   scripts/basic-image-hash.sh           print the hash of the tree
#   scripts/basic-image-hash.sh --check   print CURRENT when the pin matches the tree, else
#                                         DRIFT <pinned> <tree>; exit 0 in both cases (drift is
#                                         expected between a change's merge and its pin PR)
#
# MICOLD_BASIC_DIR and MICOLD_BASIC_PIN_FILE override the two paths (the test uses them).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="${MICOLD_BASIC_DIR:-$ROOT/packaging/basic}"
PIN_FILE="${MICOLD_BASIC_PIN_FILE:-$ROOT/crates/micold-core/src/sandbox/basic_image.rs}"

if [ ! -d "$DIR" ]; then
  echo "basic-image-hash: no directory $DIR" >&2
  exit 2
fi

# One `<sha256>  ./<name>` line per file, sorted by name; the hash of that listing covers every
# name and every byte.
tree_hash() {
  (cd "$DIR" && find . -type f -print | LC_ALL=C sort | while IFS= read -r f; do
    sha256sum "$f"
  done) | sha256sum | cut -c1-16
}

tree="$(tree_hash)"

case "${1:-}" in
  "")
    echo "$tree"
    ;;
  --check)
    pinned="$(grep -o 'BASIC_IMAGE_CONTENT_HASH: &str = "[^"]*"' "$PIN_FILE" 2>/dev/null \
      | sed 's/.*"\(.*\)"/\1/' || true)"
    if [ "$pinned" = "$tree" ]; then
      echo "CURRENT"
    else
      echo "DRIFT ${pinned:-<none>} $tree"
    fi
    ;;
  *)
    echo "usage: $0 [--check]" >&2
    exit 2
    ;;
esac
