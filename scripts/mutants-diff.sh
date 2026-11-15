#!/usr/bin/env bash
# Mutation-test the lines a change touched (#1931): `cargo mutants --in-diff` over the diff against
# <base>, limited to the Rust globs of scripts/guardrails-scope.conf, with each mutant checked by
# the tests of its own crate, run by nextest.
#
#   scripts/mutants-diff.sh [base-ref=origin/main] [out-dir=mutants-diff-out]
#
# --copy-vcs true keeps .git in cargo-mutants' scratch copy: tests that run git (micold-core's
# documentation_is_not_read, via git check-attr) fail the unmutated baseline (exit 4) without it.
#
# Writes <out-dir>/missed.txt and timeout.txt (mutants the tests did not catch) and a markdown
# report to <out-dir>/summary.md, which is also appended to $GITHUB_STEP_SUMMARY when that is set.
# Exit 0 when every mutant was caught or none applies; otherwise cargo-mutants' own code (2 missed,
# 3 timeout, 4 the unmutated tree already fails). CI treats the job as advisory: see
# docs/development/diff-guardrails.md. Run it through `mise run mutants-diff` for the build lock.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT" || exit 2
base="${1:-origin/main}"
out="${2:-mutants-diff-out}"
mkdir -p "$out"

mapfile -t globs < <(python3 scripts/diff-coverage.py --print-globs rust)
[ "${#globs[@]}" -gt 0 ] || { echo "mutants-diff: no [rust] globs in scripts/guardrails-scope.conf" >&2; exit 2; }
git diff --no-color "$(git merge-base "$base" HEAD)" HEAD >"$out/pr.diff" || exit 2

args=(mutants --in-diff "$out/pr.diff" --test-tool nextest --copy-vcs true --output "$out/raw")
for g in "${globs[@]}"; do args+=(--file "$g"); done
cargo "${args[@]}"
rc=$?

found="$out/raw/mutants.out"
for f in missed timeout; do
  if [ -f "$found/$f.txt" ]; then cp "$found/$f.txt" "$out/$f.txt"; else : >"$out/$f.txt"; fi
done
{
  echo "### mutants on the diff (advisory)"
  echo
  case "$rc" in
    0) echo "No missed mutants." ;;
    2|3) echo "Mutants the tests did not catch: $(wc -l <"$out/missed.txt") missed, $(wc -l <"$out/timeout.txt") timed out."
       echo
       echo '```'
       cat "$out/missed.txt" "$out/timeout.txt"
       echo '```' ;;
    *) echo "cargo-mutants stopped with exit code $rc before finishing." ;;
  esac
} | tee "$out/summary.md" >>"${GITHUB_STEP_SUMMARY:-/dev/null}"
rm -rf "$out/raw" "$out/pr.diff"
exit "$rc"
