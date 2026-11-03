#!/usr/bin/env bash
#
# CI's merge gate, locally and in CI's order: fmt, clippy (core, then workspace), workspace tests, then
# every scripts/tests/*.test.sh. On success it records the tree it saw for the autopilot push hook
# (scripts/autopilot/gate-hook.sh). `mise run gate` calls this; it is a script so that it also runs
# where mise is not installed, as in cloud containers.
#
# As root on Linux, the few permission-refusal tests (root_only below) run as a normal user through
# scripts/run-unprivileged.sh, since root reads and removes through any permission bits. Set
# MICOLD_GATE_AS_ROOT=1 to run everything as root anyway.
#
# Issue #1649: `cargo nextest run` replaces `cargo test` for the workspace run (mise.toml pins
# cargo-nextest; see its comment there). nextest does not run doctests at all, so `cargo test --doc
# --workspace` still runs them on its own -- dropping it would silently stop testing every `///
# ```rust` example. The 591-file test-binary consolidation nextest was evaluated for (issue #1635)
# landed separately and first as issue #1631 (docs/development/shared-build.md), so nextest already
# runs over the ~150 consolidated executables, not the original 591.

set -euo pipefail
cd "$(dirname "$0")/.."

# crates/micold-core/tests/github_locate_desktop_launch.rs fails where `gh` is on PATH only through a
# version manager (mise), which a desktop launch cannot see. This opt-out skips it there; it still
# runs wherever a launch finds `gh`, and CI ignores the variable.
export MICOLD_SKIP_GH_LAUNCH_TEST="${MICOLD_SKIP_GH_LAUNCH_TEST:-1}"

# The tree this gate saw; recorded on success.
snap=$(scripts/autopilot/review-snapshot.sh)

# Tests that check a permission refusal (an unreadable file, an undeletable directory). Root reads
# and removes through any permission bits, so as root these run as a normal user instead; every
# other test needs the root-owned build folder and toolchain, so it stays as root. A new test of
# this kind belongs here. Format: package test-target test-name.
root_only=(
	"micold-core main settings_refuses_save_over_failed_read::a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched"
	"micold-core main settings_write_is_logged::a_refused_write_is_logged_as_a_failure_with_the_reason"
	"micold-core worktree_leftovers a_directory_that_cannot_be_emptied_names_what_survived"
	"micold-core worktree_leftovers the_leftover_report_is_capped"
	"micold-daemon sessions mutation_semantics::worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports"
	"micold-daemon settings settings_service_write_refuses_failed_read::a_service_write_over_an_unreadable_settings_file_is_refused"
)

skip=()
split=0
if [ "$(id -u)" = 0 ] && [ "$(uname -s)" = Linux ] && [ -z "${MICOLD_GATE_AS_ROOT:-}" ]; then
	split=1
	for t in "${root_only[@]}"; do
		skip+=(--skip "${t##* }")
	done
fi

scripts/build-lock.sh bash -c '
  cargo fmt --all -- --check &&
  cargo clippy -p micold-core --all-targets -- -D warnings &&
  cargo clippy --workspace --all-targets -- -D warnings || exit 1
  # Both run even if one fails, so a single failing test does not hide a doctest failure (or vice
  # versa) behind a `&&` short-circuit (issue #1649 review finding). --no-fail-fast: unlike
  # `cargo test`, nextest stops the whole run after the first failing test by default, which would
  # hide every later failure; `cargo test --workspace` never did that, so this keeps the gate
  # reporting the same failures it always did.
  nextest_rc=0; cargo nextest run --workspace --no-fail-fast -- "$@" || nextest_rc=$?
  doc_rc=0; cargo test --doc --workspace || doc_rc=$?
  exit $((nextest_rc == 0 ? doc_rc : nextest_rc))' gate "${skip[@]}"

if [ "$split" = 1 ]; then
	host=$(rustc -vV | sed -n 's/^host: //p')
	runner_var="CARGO_TARGET_$(printf '%s' "$host" | tr 'a-z-' 'A-Z_')_RUNNER"
	for t in "${root_only[@]}"; do
		read -r pkg target name <<<"$t"
		echo "== as a normal user: $pkg $target $name"
		out=$(env "$runner_var=$PWD/scripts/run-unprivileged.sh" \
			scripts/build-lock.sh cargo test -p "$pkg" --test "$target" -- --exact "$name" 2>&1) || {
			printf '%s\n' "$out"
			exit 1
		}
		# A renamed test would otherwise pass by matching nothing.
		grep -q 'test result: ok. 1 passed' <<<"$out" || {
			printf '%s\n' "$out"
			echo "gate: $pkg $target $name ran no test; update root_only in scripts/gate.sh" >&2
			exit 1
		}
	done
fi

for t in scripts/tests/*.test.sh; do
	echo "== $t"
	"$t"
done

echo "${snap%%:*}" >>"$(git rev-parse --git-path autopilot-gate-ok)"
