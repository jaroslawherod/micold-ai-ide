#!/usr/bin/env bash
# Drives `scripts/basic-image-hash.sh` (feature 806, FR-002, FR-013).
#
# The basic image is published under the content hash of `packaging/basic/`, and the client pins
# that hash in `crates/micold-core/src/sandbox/basic_image.rs`. So the hash has to be a function of
# the files alone (not of the order they were written in), has to move when any byte moves, and
# `--check` has to tell a maintainer whether the pin matches the tree. Every case that edits files
# works on a temporary copy, through `MICOLD_BASIC_DIR` and `MICOLD_BASIC_PIN_FILE`.
#
# The last case reads the real Containerfile: the image is the same for every user, so nothing that
# identifies one (a key, a token, the daemon, an installed AI CLI) may be built into it.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/basic-image-hash.sh"
BASIC="$ROOT/packaging/basic"
PIN_SOURCE="$ROOT/crates/micold-core/src/sandbox/basic_image.rs"

failures=0
cases=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n      %s\n' "$1" "$2"
  failures=$((failures + 1))
}

# hash_of <dir>: the script's output for that tree.
hash_of() { MICOLD_BASIC_DIR="$1" "$SCRIPT" 2>&1; }

# copy_basic: a temporary copy of packaging/basic. Echoes its path.
copy_basic() {
  local d
  d="$(mktemp -d)"
  cp -R "$BASIC/." "$d/"
  echo "$d"
}

# pin_file <hash>: a temporary Rust file pinning <hash> the way basic_image.rs does.
pin_file() {
  local f
  f="$(mktemp)"
  printf 'pub const BASIC_IMAGE_CONTENT_HASH: &str = "%s";\n' "$1" >"$f"
  echo "$f"
}

# --- the hash ----------------------------------------------------------------------------------

cases=$((cases + 1))
first="$(hash_of "$BASIC")"
second="$(hash_of "$BASIC")"
if ! printf '%s' "$first" | grep -Eqx '[0-9a-f]{16}'; then
  fail "prints 16 lowercase hex" "got: $first"
elif [ "$first" != "$second" ]; then
  fail "prints 16 lowercase hex" "two runs disagree: $first then $second"
else
  pass "prints 16 lowercase hex, the same twice in a row"
fi

cases=$((cases + 1))
reordered="$(mktemp -d)"
# The same files, written in reverse name order, so creation order and directory order differ.
for f in $(cd "$BASIC" && find . -type f | LC_ALL=C sort -r); do
  mkdir -p "$reordered/$(dirname "$f")"
  cp "$BASIC/$f" "$reordered/$f"
done
got="$(hash_of "$reordered")"
if [ "$got" != "$first" ]; then
  fail "does not depend on the order the files were written in" "want $first, got $got"
else
  pass "does not depend on the order the files were written in"
fi
rm -rf "$reordered"

cases=$((cases + 1))
changed="$(copy_basic)"
printf ' ' >>"$changed/sshd_config"
got="$(hash_of "$changed")"
if [ "$got" = "$first" ] || ! printf '%s' "$got" | grep -Eqx '[0-9a-f]{16}'; then
  fail "changes when one byte of one file changes" "unchanged tree $first, one byte added: $got"
else
  pass "changes when one byte of one file changes"
fi

cases=$((cases + 1))
renamed="$(copy_basic)"
mv "$renamed/README.md" "$renamed/README.txt"
got="$(hash_of "$renamed")"
if [ "$got" = "$first" ]; then
  fail "changes when a file is renamed" "both $got"
else
  pass "changes when a file is renamed"
fi
rm -rf "$renamed"

# --- --check -----------------------------------------------------------------------------------

cases=$((cases + 1))
pin="$(pin_file "$first")"
output="$(MICOLD_BASIC_DIR="$BASIC" MICOLD_BASIC_PIN_FILE="$pin" "$SCRIPT" --check 2>&1)"
status=$?
if [ "$status" -ne 0 ] || [ "$output" != "CURRENT" ]; then
  fail "--check prints CURRENT when the pin matches the tree" "exit $status, output: $output"
else
  pass "--check prints CURRENT when the pin matches the tree"
fi

cases=$((cases + 1))
drifted="$(hash_of "$changed")"
output="$(MICOLD_BASIC_DIR="$changed" MICOLD_BASIC_PIN_FILE="$pin" "$SCRIPT" --check 2>&1)"
status=$?
if [ "$status" -ne 0 ] || [ "$output" != "DRIFT $first $drifted" ]; then
  fail "--check prints DRIFT <pinned> <tree>, exit 0, when they differ" \
    "want 'DRIFT $first $drifted' exit 0, got exit $status: $output"
else
  pass "--check prints DRIFT <pinned> <tree>, exit 0, when they differ"
fi
rm -f "$pin"
rm -rf "$changed"

cases=$((cases + 1))
output="$("$SCRIPT" --check 2>&1)"
status=$?
pinned="$(grep -o 'BASIC_IMAGE_CONTENT_HASH: &str = "[0-9a-f]*"' "$PIN_SOURCE" 2>/dev/null | grep -o '"[0-9a-f]*"' | tr -d '"')"
if [ "$status" -ne 0 ] || [ "$output" != "CURRENT" ]; then
  fail "the repository's pin matches packaging/basic" \
    "exit $status, output: $output (pinned '$pinned' in $PIN_SOURCE; set it to the tree's hash)"
else
  pass "the repository's pin matches packaging/basic"
fi

# --- the Containerfile carries no credential, daemon or AI CLI ---------------------------------

cases=$((cases + 1))
containerfile="$BASIC/Containerfile"
if [ ! -f "$containerfile" ]; then
  fail "Containerfile copies in no key, credential, daemon or token, and installs no AI CLI" \
    "$containerfile does not exist"
else
  # Instructions only: comments may name what is deliberately absent.
  instructions="$(grep -v '^[[:space:]]*#' "$containerfile")"
  copies="$(printf '%s\n' "$instructions" | grep -Ei '^[[:space:]]*(COPY|ADD)[[:space:]]' || true)"
  bad_copy="$(printf '%s\n' "$copies" | grep -Ei 'key|credential|\.ssh|micold-daemon|token' || true)"
  npm="$(printf '%s\n' "$instructions" | grep -Ei 'npm[[:space:]]+(install|i)[[:space:]]' || true)"
  if [ -n "$bad_copy" ]; then
    fail "Containerfile copies in no key, credential, daemon or token, and installs no AI CLI" \
      "copies: $bad_copy"
  elif [ -n "$npm" ]; then
    fail "Containerfile copies in no key, credential, daemon or token, and installs no AI CLI" \
      "installs: $npm"
  else
    pass "Containerfile copies in no key, credential, daemon or token, and installs no AI CLI"
  fi
fi

printf '\n%d case(s), %d failure(s)\n' "$cases" "$failures"
[ "$failures" -eq 0 ]
