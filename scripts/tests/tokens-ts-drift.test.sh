#!/usr/bin/env bash
# Fails when the committed `apps/client/src/theme/tokens.generated.ts` differs from what
# `micold-tokens-ts` generates, or is missing (feature 1950, FR-007). A self-test then proves the
# check itself: an edited copy and a missing file must both fail, naming the file and the
# regeneration command.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

committed=apps/client/src/theme/tokens.generated.ts
regen="cargo run -p micold-core --bin micold-tokens-ts -- $committed"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cargo run -q -p micold-core --bin micold-tokens-ts -- "$tmp/fresh.ts"

# check <file>: succeeds when <file> equals the freshly generated module; otherwise prints the
# file and the command that regenerates it, and fails.
check() {
  local file="$1"
  if [ ! -f "$file" ]; then
    echo "DRIFT: $file is missing. Regenerate it: $regen"
    return 1
  fi
  if ! cmp -s "$tmp/fresh.ts" "$file"; then
    echo "DRIFT: $file differs from the generator. Regenerate it: $regen"
    return 1
  fi
}

check "$committed"
echo "ok    $committed matches the generator"

# Self-test: the check must catch an edit and a missing file.
cp "$committed" "$tmp/edited.ts"
echo "// hand edit" >>"$tmp/edited.ts"
if out="$(check "$tmp/edited.ts")"; then
  echo "FAIL  an edited copy passed the drift check"
  exit 1
fi
case "$out" in
  *"$tmp/edited.ts"*"$regen"*) echo "ok    an edited copy fails, naming the file and the command" ;;
  *) echo "FAIL  edited-copy message lacks file or command: $out"; exit 1 ;;
esac

if out="$(check "$tmp/missing.ts")"; then
  echo "FAIL  a missing file passed the drift check"
  exit 1
fi
case "$out" in
  *"$tmp/missing.ts"*"$regen"*) echo "ok    a missing file fails, naming the file and the command" ;;
  *) echo "FAIL  missing-file message lacks file or command: $out"; exit 1 ;;
esac
