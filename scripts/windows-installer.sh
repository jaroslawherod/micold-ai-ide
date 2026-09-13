#!/usr/bin/env bash
# Build the per-user Windows setup .exe for one architecture (feature 030, FR-016).
#
#   scripts/windows-installer.sh [--arch x64|arm64] [--out-dir DIR]
#   mise run windows-installer
#
# Contract: specs/030-windows-installer/contracts/windows-installer.md, section "Build interface".
# Driven by scripts/tests/windows-installer.test.sh.

set -euo pipefail

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*) ;;
*)
	echo "${0##*/}: the Windows installer is built on Windows (Inno Setup has no other host)" >&2
	exit 1
	;;
esac

usage() {
	echo "usage: ${0##*/} [--arch x64|arm64] [--out-dir DIR]" >&2
	exit 2
}

# The host arch unless told otherwise; each package installs only on its own arch (FR-015).
case "${PROCESSOR_ARCHITECTURE:-}" in
ARM64) arch=arm64 ;;
*) arch=x64 ;;
esac
out_dir=""
while [ "$#" -gt 0 ]; do
	case "$1" in
	--arch)
		[ "$#" -ge 2 ] || usage
		arch="$2"
		shift 2
		;;
	--out-dir)
		[ "$#" -ge 2 ] || usage
		out_dir="$2"
		shift 2
		;;
	*) usage ;;
	esac
done

case "$arch" in
x64) triple=x86_64-pc-windows-msvc ;;
arm64) triple=aarch64-pc-windows-msvc ;;
*)
	echo "${0##*/}: --arch must be x64 or arm64, got '$arch'" >&2
	exit 2
	;;
esac

root="$(cd "$(dirname "$0")/.." && pwd)"

# The one version every package takes (FR-013); the .iss never spells one out.
version="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version *= *"\(.*\)"/\1/p' "$root/Cargo.toml")"
if [ -z "$version" ]; then
	echo "${0##*/}: no [workspace.package] version in $root/Cargo.toml" >&2
	exit 1
fi

target_dir="$("$root/scripts/build-lock.sh" --print-target-dir)"
bin_dir="$target_dir/$triple/release"

# Inno Setup's compiler: $ISCC, then the installer's default location, then PATH. Resolved before the
# build, so a missing one fails in a second rather than after a release compile.
iscc="${ISCC:-}"
program_files_x86="$(printenv 'ProgramFiles(x86)' || true)"
if [ -z "$iscc" ] && [ -n "$program_files_x86" ] && [ -x "$program_files_x86/Inno Setup 6/ISCC.exe" ]; then
	iscc="$program_files_x86/Inno Setup 6/ISCC.exe"
fi
if [ -z "$iscc" ]; then
	iscc="$(command -v iscc || command -v ISCC || true)"
fi
if [ -z "$iscc" ]; then
	echo "${0##*/}: Inno Setup 6 not found. Install it from https://jrsoftware.org/isdl.php, or set ISCC to ISCC.exe" >&2
	exit 1
fi

# iscc is a native Windows program: hand it Windows paths. Off a Git Bash/MSYS2 host (the tests)
# there is no cygpath, and paths pass through unchanged.
winpath() {
	if command -v cygpath >/dev/null 2>&1; then
		cygpath -w "$1"
	else
		printf '%s\n' "$1"
	fi
}

# Exactly the app and the daemon: the client crate also builds the showcase binary (FR-002, FR-012).
"$root/scripts/build-lock.sh" cargo build --release --locked \
	-p micold-client --bin micold-ai-ide -p micold-daemon --target "$triple"

out_dir="${out_dir:-$target_dir/windows-installer}"
mkdir -p "$out_dir"
# MSYS2 would otherwise rewrite every `/D...` switch into a path.
MSYS2_ARG_CONV_EXCL='*' "$iscc" \
	"/DAppVersion=$version" \
	"/DArch=$arch" \
	"/DBinDir=$(winpath "$bin_dir")" \
	"/O$(winpath "$out_dir")" \
	"$(winpath "$root/packaging/windows/micold-ai-ide.iss")"

echo "Built: $out_dir/micold-ai-ide-$version-$arch-setup.exe"
