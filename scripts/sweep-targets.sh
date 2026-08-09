#!/usr/bin/env bash
#
# Run `cargo sweep` against every target directory this repository accumulates:
# the shared one beside the main checkout, plus the private one each worktree
# grows under a bare `cargo`.
#
# Two kinds of target dir exist because there are two ways to reach cargo:
#
#   mise run <task>  -> scripts/build-lock.sh exports CARGO_TARGET_DIR to
#                       <main checkout>/target-shared, so every worktree's
#                       build lands in the one shared directory.
#   bare `cargo`     -> no such export, so cargo resolves `target-dir` from
#                       .cargo/config.toml. That key is relative to the config
#                       file's own directory and the closest config wins --
#                       every worktree has its own checked-in copy, so a bare
#                       cargo builds into target-shared/ beside the *worktree*.
#
# Sweeping only the first kind leaves the second growing unbounded, which is
# how this disk filled. So iterate `git worktree list` and let each checkout
# resolve its own directory, with CARGO_TARGET_DIR unset -- inherited from a
# build-lock.sh parent it would redirect every pass onto the same directory and
# silently skip the per-worktree ones.
#
# Usage:
#   scripts/sweep-targets.sh --time 7
#   scripts/sweep-targets.sh --dry-run --time 7

set -euo pipefail

unset CARGO_TARGET_DIR

if [ "$#" -eq 0 ]; then
	echo "usage: ${0##*/} <cargo-sweep args...>" >&2
	exit 2
fi

if ! git rev-parse --git-common-dir >/dev/null 2>&1; then
	echo "${0##*/}: not a git repository" >&2
	exit 1
fi

status=0
swept=0

while IFS= read -r line; do
	case "$line" in
	worktree\ *) ;;
	*) continue ;;
	esac
	checkout=${line#worktree }

	# cargo-sweep needs a cargo project to resolve target-dir from; a worktree
	# checked out to a branch without a manifest at its root has none.
	if [ ! -f "$checkout/Cargo.toml" ]; then
		echo "sweep: skipping $checkout (no Cargo.toml)" >&2
		continue
	fi

	echo "sweep: cargo sweep $* -- $checkout"
	# One directory failing (mid-cleanup, permissions) should not abandon the
	# rest, so record the failure and carry on.
	if ! (cd "$checkout" && cargo sweep "$@"); then
		echo "sweep: failed in $checkout" >&2
		status=1
	fi
	swept=$((swept + 1))
done < <(git worktree list --porcelain)

if [ "$swept" -eq 0 ]; then
	echo "${0##*/}: no sweepable checkouts found" >&2
	exit 1
fi

exit "$status"
