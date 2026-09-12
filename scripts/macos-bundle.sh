#!/usr/bin/env bash
# Compose `micold-ai-ide.app` from binaries cargo has already built, and (on macOS) sign it.
#
# Normative structure: specs/028-macos-package/contracts/bundle-layout.md.
#
# Usage:
#   scripts/macos-bundle.sh --bin-dir <dir> --out <dir> [--stage-only] [--version <v>]
#
#   --bin-dir     directory holding the two shipped binaries: a cargo profile directory, or the
#                 staging directory `scripts/macos-dmg.sh` fills with `lipo` output
#   --out         where `micold-ai-ide.app` is written; created if absent
#   --stage-only  compose and substitute, run no `codesign`. Runs on any OS, which is what makes
#                 the layout testable without a Mac (research R3)
#   --version     override the version; defaults to the workspace version from `cargo metadata`
#
# The two binaries are named one at a time, on purpose. `cargo build` puts `micold-showcase` in
# the same directory as the two that ship, so any glob into `Contents/MacOS/` ships a
# development-only binary to users (FR-006) -- and a bundle with a third executable is
# indistinguishable, after the fact, from one that globbed. `crates/micold-client/tests/
# packaging_excludes_showcase.rs` asserts this file never learns to glob.
#
# Nothing here needs an Apple account, a certificate, or a repository secret: the signature is
# ad-hoc, so a contributor's build is of equivalent trust status to the released one (FR-016).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE="$ROOT/packaging/macos/Info.plist.in"
ENTITLEMENTS="$ROOT/packaging/macos/entitlements.plist"
ICON_REL="assets/icon/icon.icns"
ICON="$ROOT/$ICON_REL"

# The bundle's executables, and the only ones. Order matters at signing time: the nested binary is
# signed before the enclosing bundle, or the bundle's seal does not cover it.
EXECUTABLE="micold-ai-ide"
BINARIES=("micold-daemon" "$EXECUTABLE")

BIN_DIR=""
OUT=""
STAGE_ONLY=0
VERSION=""

die() {
  printf 'macos-bundle: %s\n' "$1" >&2
  exit 1
}

usage() {
  sed -n '2,20p' "${BASH_SOURCE[0]}" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --bin-dir) BIN_DIR="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    --version) VERSION="${2:-}"; shift 2 ;;
    --stage-only) STAGE_ONLY=1; shift ;;
    -h|--help) usage ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$BIN_DIR" ] || die "--bin-dir is required"
[ -n "$OUT" ] || die "--out is required"
[ -d "$BIN_DIR" ] || die "--bin-dir is not a directory: $BIN_DIR"

# --- inputs ----------------------------------------------------------------
#
# Every missing input is named before anything is written, so the message says what to fix rather
# than leaving a half-composed bundle behind.

for bin in "${BINARIES[@]}"; do
  [ -f "$BIN_DIR/$bin" ] || die "missing binary: $bin (looked in $BIN_DIR)"
done
[ -f "$ICON" ] || die "missing icon: $ICON_REL"
[ -f "$TEMPLATE" ] || die "missing template: packaging/macos/Info.plist.in"

if [ -z "$VERSION" ]; then
  # The resolved version, not a line that looks like one: `cargo metadata` reports what cargo
  # itself would build, so the plist, the release tag, and the About dialog derive from the single
  # `[workspace.package] version` release-please bumps (FR-003).
  VERSION="$(cargo metadata --format-version 1 --no-deps --manifest-path "$ROOT/Cargo.toml" |
    python3 -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="micold-client"))')"
  [ -n "$VERSION" ] || die "could not read the workspace version from cargo metadata"
fi

# --- compose ---------------------------------------------------------------

APP="$OUT/micold-ai-ide.app"
CONTENTS="$APP/Contents"

rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

for bin in "${BINARIES[@]}"; do
  cp "$BIN_DIR/$bin" "$CONTENTS/MacOS/$bin"
  chmod 0755 "$CONTENTS/MacOS/$bin"
done

cp "$ICON" "$CONTENTS/Resources/icon.icns"

# `APPL????` with no trailing newline: the file is eight bytes, and Finder reads it as such.
printf 'APPL????' > "$CONTENTS/PkgInfo"

sed "s/@VERSION@/$VERSION/g" "$TEMPLATE" > "$CONTENTS/Info.plist"

# An unsubstituted token means a key was added to the template without being wired into the
# substitution above. Shipping it would put a literal `@TOKEN@` where the version, or whatever
# else it names, belongs -- so it fails the build. The check is on the token *shape* rather than
# on `@VERSION@`, because the token that regresses is the one nobody thought about.
# `|| true` because no match is the good outcome, and `grep` reports it as a failure.
residue="$(grep -o '@[A-Z][A-Z0-9_]*@' "$CONTENTS/Info.plist" | sort -u | tr '\n' ' ' || true)"
if [ -n "${residue% }" ]; then
  rm -rf "$APP"
  die "unsubstituted token in Info.plist: ${residue% }"
fi

if [ "$(uname -s)" = "Darwin" ]; then
  plutil -lint "$CONTENTS/Info.plist" >/dev/null ||
    die "Info.plist is not a well-formed plist after substitution"
fi

if [ "$STAGE_ONLY" -eq 1 ]; then
  echo "staged (unsigned): $APP"
  exit 0
fi

if [ "$(uname -s)" != "Darwin" ]; then
  die "signing needs macOS; use --stage-only to compose the bundle elsewhere"
fi

# --- sign ------------------------------------------------------------------
#
# Ad-hoc (`--sign -`): no identity, no Apple account, no secret (FR-016). The hardened runtime is
# enabled because it is what a Developer ID signature will require anyway, so the seam FR-017
# leaves open is a change of identity rather than a change of shape.
#
# The nested daemon is signed first. A bundle's seal covers its contents as they were when it was
# sealed; signing the daemon afterwards invalidates the enclosing signature.

codesign --force --sign - --timestamp=none --options runtime \
  --entitlements "$ENTITLEMENTS" "$CONTENTS/MacOS/micold-daemon"

codesign --force --sign - --timestamp=none --options runtime \
  --entitlements "$ENTITLEMENTS" "$APP"

# --- verify ----------------------------------------------------------------
#
# Verification runs wherever the bundle is signed, not only at release time: a bundle modified
# after signing reaches the user as "the application is damaged and can't be opened", which names
# nothing. `--strict` is what makes the check see a file added to `Contents/` after the fact.
#
# `spctl --assess` is deliberately not run. It rejects an ad-hoc-signed app, and that rejection is
# the documented first-launch block (FR-013), not a build failure. It stays a diagnostic in
# docs/development/macos-packaging.md; `crates/micold-core/tests/macos_signature_gate.rs` asserts
# it never becomes a gate here.

codesign --verify --strict --verbose=2 "$APP"

if ! codesign -dvv "$APP" 2>&1 | grep -q 'Signature=adhoc'; then
  die "the bundle is signed, but not ad-hoc — check the signing identity in this script"
fi

echo "signed (ad-hoc) and verified: $APP"
