#!/usr/bin/env bash
# Build the static Linux daemons a client copies to a remote Linux host or a container whose
# platform differs from its own (feature 806 FR-009, delivering 489 T083).
#
#   scripts/remote-daemon-bundle.sh [--out DIR] [--dry-run]
#
# Produces `micold-daemon-linux-x86_64` and `micold-daemon-linux-aarch64` in DIR (default
# `<target dir>/remote-daemon`). Both are musl builds, so they run on any Linux whatever its libc.
# Cross-compiling uses `cargo zigbuild`, which needs `cargo-zigbuild` and `zig` on PATH.
# `--dry-run` prints the plan and builds nothing. Driven by scripts/tests/remote-daemon-bundle.test.sh.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"

# <rust target> <output name>, one per architecture the client can probe (`SystemSsh::bundled`
# reads `micold-daemon-<os>-<arch>`).
TARGETS=(
	"x86_64-unknown-linux-musl micold-daemon-linux-x86_64"
	"aarch64-unknown-linux-musl micold-daemon-linux-aarch64"
)

usage() {
	echo "usage: ${0##*/} [--out DIR] [--dry-run]" >&2
	exit 2
}

out=""
dry_run=0
while [ "$#" -gt 0 ]; do
	case "$1" in
	--out)
		[ "$#" -ge 2 ] || usage
		out="$2"
		shift 2
		;;
	--dry-run)
		dry_run=1
		shift
		;;
	*) usage ;;
	esac
done

target_dir="$("$root/scripts/build-lock.sh" --print-target-dir)"
out="${out:-$target_dir/remote-daemon}"

if [ "$dry_run" -eq 1 ]; then
	for entry in "${TARGETS[@]}"; do
		read -r triple name <<<"$entry"
		echo "cargo zigbuild --release -p micold-daemon --target $triple -> $out/$name"
	done
	exit 0
fi

# `zig` itself may be a PyPI `ziglang` wheel, which has no `zig` on PATH: cargo-zigbuild runs it as
# `python3 -m ziglang`, so that counts too.
have_zig() {
	command -v zig >/dev/null 2>&1 ||
		python3 -m ziglang version >/dev/null 2>&1 ||
		python -m ziglang version >/dev/null 2>&1
}
if ! command -v cargo-zigbuild >/dev/null 2>&1; then
	echo "${0##*/}: cargo-zigbuild is required to build the Linux daemons and was not found on PATH (cargo install cargo-zigbuild --locked)" >&2
	exit 1
fi
if ! have_zig; then
	echo "${0##*/}: zig is required to build the Linux daemons and was not found (pip install ziglang, or https://ziglang.org/download/)" >&2
	exit 1
fi

mkdir -p "$out"
for entry in "${TARGETS[@]}"; do
	read -r triple name <<<"$entry"
	"$root/scripts/build-lock.sh" cargo zigbuild --release --locked -p micold-daemon --bin micold-daemon --target "$triple"
	install -m 0755 "$target_dir/$triple/release/micold-daemon" "$out/$name"
done
echo "Built: $out"
