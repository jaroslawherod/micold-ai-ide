#!/usr/bin/env bash
#
# Run a cargo command against this repository's *shared* target directory and,
# unless --no-lock is given, hold a repo-wide lock so that only one build runs
# at a time across every worktree.
#
# `jobs = 4` in .cargo/config.toml caps one cargo process. It does not compose:
# four agent worktrees building at once means sixteen jobs, which oversubscribes
# RAM, spills to the swap file on the same NVMe as the target dirs, and stalls
# the whole machine on I/O. This wrapper makes the cap compose.
#
# The shared target dir replaces one ~30 GB target/ per worktree with a single
# one, so dependencies are compiled once rather than once per branch.
#
# Usage:
#   scripts/build-lock.sh cargo test --workspace         # shared target + lock
#   scripts/build-lock.sh --no-lock cargo run -p foo     # shared target only
#   scripts/build-lock.sh --print-target-dir             # resolve the path only
#
# Opt out of the lock:          MICOLD_NO_BUILD_LOCK=1
# Opt out of the shared target: set CARGO_TARGET_DIR yourself

set -euo pipefail

want_lock=1
print_target_dir=0
case "${1:-}" in
--no-lock)
	want_lock=0
	shift
	;;
--print-target-dir)
	print_target_dir=1
	shift
	;;
esac
if [ -n "${MICOLD_NO_BUILD_LOCK:-}" ]; then
	want_lock=0
fi

if [ "$#" -eq 0 ] && [ "$print_target_dir" -eq 0 ]; then
	echo "usage: ${0##*/} [--no-lock | --print-target-dir] <command> [args...]" >&2
	exit 2
fi

# The common git dir is shared by the main checkout and every linked worktree,
# so paths derived from it are visible to all of them. It also sits outside the
# working tree, so the lock never shows up in `git status`.
if ! common_dir=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null); then
	if [ "$print_target_dir" -eq 1 ]; then
		echo "${CARGO_TARGET_DIR:-target}"
		exit 0
	fi
	echo "${0##*/}: not a git repository, running unmodified" >&2
	exec "$@"
fi

# This export is what actually makes the target directory shared. `target-dir`
# in .cargo/config.toml does not: it is relative to its own config file, that
# file is checked in, and cargo's closest config wins -- so a bare cargo in a
# worktree resolves it to a target-shared/ beside the *worktree*, not this one.
# Deriving the path from the common git dir instead points every worktree at
# the main checkout's directory. Keep the directory name in sync with
# .cargo/config.toml. An explicit CARGO_TARGET_DIR still wins.
repo_root=$(dirname "$common_dir")
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target-shared}"

if [ "$print_target_dir" -eq 1 ]; then
	echo "$CARGO_TARGET_DIR"
	exit 0
fi

if [ "$want_lock" -eq 0 ]; then
	exec "$@"
fi

lock_file="$common_dir/micold-build.lock"
owner_file="$common_dir/micold-build.owner"

exec 9>"$lock_file"

# Take the lock without blocking first, so the "waiting" note is only printed
# when there is something to wait for.
if ! flock --nonblock 9; then
	holder=$(cat "$owner_file" 2>/dev/null || true)
	echo "${0##*/}: another build holds the lock${holder:+ ($holder)}; waiting..." >&2
	flock 9
fi

printf '%s, pid %s\n' "$PWD" "$$" >"$owner_file"
trap 'rm -f "$owner_file"' EXIT

status=0
"$@" || status=$?
exit "$status"
