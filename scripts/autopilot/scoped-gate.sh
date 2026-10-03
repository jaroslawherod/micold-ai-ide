#!/usr/bin/env bash
# The gate for the crates this branch changed, between review rounds of a milestone.
#
# Usage: scoped-gate.sh [--dry-run]
#
# A milestone used to run the full `mise run gate` four or five times: after implementing, and again
# after each round of review fixes. Only the last run decides what may be pushed. This one checks
# what the change can break, in CI's order: fmt over the whole workspace, then clippy and tests of
# the changed crates (all of them when micold-core or the workspace's own files changed, since
# every crate builds on those), then the shell suites when scripts/ or mise.toml changed. It does
# not record a green gate: `git push` still needs a green `mise run gate` on the final tree.
#
# Changed = differs from the merge base with origin/main: commits, staged, unstaged and untracked.
# Prints `SCOPE <crate>...`, `SCOPE workspace` or `SCOPE none`, then runs. --dry-run prints the
# commands instead. Exit: the first failing step's status, or 0.
set -uo pipefail
dry=false
[ "${1:-}" = --dry-run ] && dry=true
root="$(git rev-parse --show-toplevel)" || exit 2
cd "$root" || exit 2

base="$(git merge-base origin/main HEAD 2>/dev/null)" || { echo "NO-BASE: fetch origin first"; exit 2; }
changed="$( { git diff --name-only "$base"; git ls-files --others --exclude-standard; } | sort -u)"

workspace=false scripts=false crates=()
while read -r f; do
  case "$f" in
    "") ;;
    crates/micold-core/*|Cargo.toml|Cargo.lock|.cargo/*|rust-toolchain.toml|clippy.toml|rustfmt.toml)
      workspace=true ;;
    crates/*/*)
      c="${f#crates/}"; c="${c%%/*}"
      [[ " ${crates[*]-} " == *" $c "* ]] || crates+=("$c") ;;
    scripts/*|mise.toml) scripts=true ;;
  esac
done <<<"$changed"

if $workspace; then echo "SCOPE workspace"
elif [ "${#crates[@]}" -gt 0 ]; then echo "SCOPE ${crates[*]}"
else echo "SCOPE none"
fi

run() { if $dry; then echo "RUN $*"; else "$@" || exit $?; fi; }

if $workspace; then
  run scripts/build-lock.sh bash -c 'cargo fmt --all -- --check &&
    cargo clippy -p micold-core --all-targets -- -D warnings &&
    cargo clippy --workspace --all-targets -- -D warnings &&
    cargo test --workspace'
elif [ "${#crates[@]}" -gt 0 ]; then
  pkgs="$(printf -- ' -p %s' "${crates[@]}")"
  run scripts/build-lock.sh bash -c "cargo fmt --all -- --check &&
    cargo clippy$pkgs --all-targets -- -D warnings &&
    cargo test$pkgs"
else
  run cargo fmt --all -- --check
fi
if $scripts; then run mise run test-scripts; fi
exit 0
