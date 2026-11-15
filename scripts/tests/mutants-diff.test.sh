#!/usr/bin/env bash
# Regression check for the mutants-on-the-diff baseline: cargo-mutants' scratch copy must keep .git,
# or micold-core's tests that run `git check-attr` fail the unmutated baseline (exit 4).
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel)"
if grep -q -- '--copy-vcs true' "$ROOT/scripts/mutants-diff.sh"; then
  echo "ok: mutants-diff.sh passes --copy-vcs true"
else
  echo "FAIL: mutants-diff.sh must pass --copy-vcs true" >&2
  exit 1
fi
