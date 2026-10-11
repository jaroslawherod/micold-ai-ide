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

set -euo pipefail
cd "$(dirname "$0")/.."

# The tree this gate saw; recorded on success.
snap=$(scripts/autopilot/review-snapshot.sh)

# Tests that check a permission refusal (an unreadable file, an undeletable directory). Root reads
# and removes through any permission bits, so as root these run as a normal user instead; every
# other test needs the root-owned build folder and toolchain, so it stays as root. A new test of
# this kind belongs here. Format: package test-target test-name.
root_only=(
	"micold-core settings_refuses_save_over_failed_read a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched"
	"micold-core settings_write_is_logged a_refused_write_is_logged_as_a_failure_with_the_reason"
	"micold-core worktree_leftovers a_directory_that_cannot_be_emptied_names_what_survived"
	"micold-core worktree_leftovers the_leftover_report_is_capped"
	"micold-daemon mutation_semantics worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports"
	"micold-daemon settings_service_write_refuses_failed_read a_service_write_over_an_unreadable_settings_file_is_refused"
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
  cargo clippy --workspace --all-targets -- -D warnings &&
  cargo test --workspace -- "$@"' gate "${skip[@]}"

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
