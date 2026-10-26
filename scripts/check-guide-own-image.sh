#!/usr/bin/env bash
# #1439 quickstart B3 (FR-012, SC-005): the own-image example of the sandbox page's "Building
# projects in the basic image" section builds as written, and `pkg-config --libs libpq` prints
# `-lpq` in the result.
#
#   scripts/check-guide-own-image.sh --print            print the example's Containerfile
#   scripts/check-guide-own-image.sh <runtime> [<base>] build it with <runtime> (docker or podman),
#                                                       its FROM image replaced by <base> when
#                                                       given (CI builds on `micold-basic:dev`),
#                                                       and check pkg-config in the image
#
# MICOLD_GUIDE overrides the page (the test uses it).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUIDE="${MICOLD_GUIDE:-$ROOT/docs/user-guide/sandboxed-daemon.md}"

# The first ```dockerfile block after the section's heading, up to the next heading. Inside the
# block a `#` line is a Dockerfile comment, not a heading.
example() {
  awk '
    /^### Building projects in the basic image$/ { in_section = 1; next }
    in_block && /^```$/ { exit }
    in_block { print; next }
    in_section && /^#+ / { exit }
    in_section && /^```dockerfile$/ { in_block = 1; next }
  ' "$GUIDE"
}

text="$(example)"
if [ -z "$text" ] || ! printf '%s\n' "$text" | grep -q '^FROM '; then
  echo "check-guide-own-image: no dockerfile block with a FROM line in the section of $GUIDE" >&2
  exit 1
fi

if [ "${1:-}" = "--print" ]; then
  printf '%s\n' "$text"
  exit 0
fi

runtime="${1:?usage: check-guide-own-image.sh --print | <runtime> [<base image>]}"
base="${2:-}"
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
if [ -n "$base" ]; then
  # Only the first FROM, also when it is the first line (portable; GNU sed's `0,/re/` is not).
  printf '%s\n' "$text" | awk -v base="$base" '!done && /^FROM / { print "FROM " base; done = 1; next } { print }' >"$dir/Containerfile"
else
  printf '%s\n' "$text" >"$dir/Containerfile"
fi

tag="micold-guide-own-image:check"
(cd "$dir" && "$runtime" build -t "$tag" -f Containerfile .)
libs="$("$runtime" run --rm --entrypoint pkg-config "$tag" --libs libpq)"
"$runtime" rmi "$tag" >/dev/null 2>&1 || true
if [ "$libs" != "-lpq" ]; then
  echo "check-guide-own-image: pkg-config --libs libpq printed '$libs', not -lpq" >&2
  exit 1
fi
echo "the guide's own-image example builds with $runtime; pkg-config --libs libpq: $libs"
