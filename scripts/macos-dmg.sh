#!/usr/bin/env bash
# Produce `MicoldAIIDE-<version>-universal.dmg` -- the one macOS download a release publishes.
#
# Normative structure: specs/028-macos-package/contracts/release-artifacts.md and
# specs/028-macos-package/contracts/bundle-layout.md.
#
# Usage:
#   scripts/macos-dmg.sh (--target-dir <dir> | --bin-dir <dir>) --out <dir> [--version <v>]
#
#   --target-dir  a cargo target directory holding a release build for every target in TARGETS,
#                 which `lipo` merges into the universal binaries the bundle ships (FR-020)
#   --bin-dir     a directory that already holds the two shipped binaries, merged or single-arch;
#                 what `mise run dmg` passes for a local build of this Mac only
#   --out         where the .dmg is written; created if absent
#   --version     override the version; defaults to the workspace version from `cargo metadata`
#
# This is the *delivery container*, not a second way to lay out an application: composing and
# signing the bundle belongs to `scripts/macos-bundle.sh` and this script calls it rather than
# repeating it. One producer of the layout means the per-pull-request gate in `ci.yml`, which runs
# the bundle script, is testing the same bytes a release ships.
#
# Needs macOS: `hdiutil` is the only supported way to make a disk image, and the bundle has to be
# signed before it goes in. There is no `--stage-only` equivalent here for that reason -- what a
# Linux machine can check about the layout, it checks through the bundle script.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE_SH="$ROOT/scripts/macos-bundle.sh"

APP="micold-ai-ide.app"
VOLNAME="Micold AI IDE"

# The architectures one download runs on, as a list rather than as two `lipo` arguments.
#
# macOS 26 (Tahoe) is Apple's last Intel release, so this list has a known end: once the supported
# floor moves past it, `x86_64-apple-darwin` is deleted from this line and everything below keeps
# working -- a one-element merge is still a merge, and the artifact does not change name or shape
# (research R8). Two spelled-out paths would have made that a rewrite of the merge instead.
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

# The bundle's executables. Named, not globbed, for the same reason `macos-bundle.sh` names them.
BINARIES=("micold-ai-ide" "micold-daemon")

BIN_DIR=""
TARGET_DIR=""
OUT=""
VERSION=""

die() {
  printf 'macos-dmg: %s\n' "$1" >&2
  exit 1
}

usage() {
  sed -n '2,21p' "${BASH_SOURCE[0]}" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --bin-dir) BIN_DIR="${2:-}"; shift 2 ;;
    --target-dir) TARGET_DIR="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    --version) VERSION="${2:-}"; shift 2 ;;
    -h|--help) usage ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$OUT" ] || die "--out is required"
[ -x "$BUNDLE_SH" ] || die "missing scripts/macos-bundle.sh"

if [ -n "$TARGET_DIR" ] && [ -n "$BIN_DIR" ]; then
  die "--target-dir and --bin-dir are alternatives: pass one"
fi
[ -n "$TARGET_DIR" ] || [ -n "$BIN_DIR" ] || die "one of --target-dir or --bin-dir is required"

if [ -n "$BIN_DIR" ]; then
  [ -d "$BIN_DIR" ] || die "--bin-dir is not a directory: $BIN_DIR"
else
  [ -d "$TARGET_DIR" ] || die "--target-dir is not a directory: $TARGET_DIR"
  # Every missing per-target build is named, with its target, before anything is written. A release
  # that is going to fail because one architecture was never built should say which one -- the
  # message a bare `lipo` failure gives names a path, and the path is the part you can already see.
  missing=()
  for target in "${TARGETS[@]}"; do
    for bin in "${BINARIES[@]}"; do
      [ -f "$TARGET_DIR/$target/release/$bin" ] ||
        missing+=("$bin for $target (looked in $TARGET_DIR/$target/release)")
    done
  done
  if [ "${#missing[@]}" -gt 0 ]; then
    for m in "${missing[@]}"; do
      printf 'macos-dmg: missing binary: %s\n' "$m" >&2
    done
    die "cannot merge a universal binary from an incomplete build"
  fi
fi

# Deliberately after the input checks: see above. The platform is the last thing to stop this.
[ "$(uname -s)" = "Darwin" ] || die "hdiutil needs macOS"

if [ -z "$VERSION" ]; then
  VERSION="$(cargo metadata --format-version 1 --no-deps --manifest-path "$ROOT/Cargo.toml" |
    python3 -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="micold-client"))')"
  [ -n "$VERSION" ] || die "could not read the workspace version from cargo metadata"
fi

DMG="$OUT/MicoldAIIDE-$VERSION-universal.dmg"

# --- compose and sign the bundle -------------------------------------------
#
# Into a staging directory that becomes the image's contents, so the .app the user drags is
# byte-for-byte the one that was signed and verified a moment ago.

STAGE="$(mktemp -d)"
MERGED="$(mktemp -d)"
cleanup() { rm -rf "$STAGE" "$MERGED"; }
trap cleanup EXIT

# --- one binary per architecture, merged into one -------------------------
#
# `lipo -create` over the target list. The bundle script is given the merged directory and is none
# the wiser: it composes and signs whatever `--bin-dir` holds, so there is exactly one description
# of the bundle's layout whether the binaries are universal or not.

if [ -n "$BIN_DIR" ]; then
  # Already merged, or single-architecture on purpose (`mise run dmg`). `MERGED` stays the empty
  # temp dir and is removed with the rest; the caller's directory is never something this script
  # owns, and `cleanup` must not be pointed at it.
  SOURCE="$BIN_DIR"
else
  SOURCE="$MERGED"
  for bin in "${BINARIES[@]}"; do
    inputs=()
    for target in "${TARGETS[@]}"; do
      inputs+=("$TARGET_DIR/$target/release/$bin")
    done
    lipo -create -output "$MERGED/$bin" "${inputs[@]}" ||
      die "lipo could not merge $bin from: ${inputs[*]}"
    chmod 0755 "$MERGED/$bin"
  done
fi

"$BUNDLE_SH" --bin-dir "$SOURCE" --out "$STAGE" --version "$VERSION"

# --- the install gesture ---------------------------------------------------
#
# A symlink to /Applications beside the app is what makes the window a drag target: the user opens
# the image and moves the icon across, which is the gesture every Mac user already knows. Without
# it the only obvious action is to double-click the app *inside the image*, which half-works and
# then loses their state on eject -- the failure `micold_core::install_location` exists to catch,
# and this symlink is what makes catching it rare rather than routine.

ln -s /Applications "$STAGE/Applications"

# --- the image -------------------------------------------------------------
#
# UDZO: read-only and zlib-compressed, the conventional format for an application download. The
# name carries the version and `universal`, so a file on disk still says what it is once it has
# left the download page (FR-008, FR-009).

mkdir -p "$OUT"
rm -f "$DMG"

hdiutil create \
  -volname "$VOLNAME" \
  -srcfolder "$STAGE" \
  -fs HFS+ \
  -format UDZO \
  -ov \
  "$DMG" >/dev/null

[ -f "$DMG" ] || die "hdiutil reported success but produced no image at $DMG"

# --- verify what is actually inside ----------------------------------------
#
# The image is mounted and the bundle re-verified from inside it, because everything up to here
# checked a directory, and what ships is the image. A signature broken by the copy into the image
# reaches the user as "the application is damaged", which names nothing.

MOUNT="$(mktemp -d)"
hdiutil attach "$DMG" -mountpoint "$MOUNT" -nobrowse -readonly >/dev/null
detach() { hdiutil detach "$MOUNT" -quiet >/dev/null 2>&1 || true; cleanup; }
trap detach EXIT

codesign --verify --strict --verbose=2 "$MOUNT/$APP" ||
  die "the bundle inside the image does not verify"

[ -L "$MOUNT/Applications" ] || die "the image has no Applications symlink to drag onto"

count="$(find "$MOUNT/$APP/Contents/MacOS" -type f | wc -l | tr -d ' ')"
[ "$count" = "2" ] ||
  die "expected exactly two files in Contents/MacOS/, found $count (a third is a glob)"

echo "built and verified: $DMG"
