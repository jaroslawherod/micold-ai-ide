#!/usr/bin/env bash
# Drives `scripts/macos-bundle.sh` over crafted input directories (feature 028).
#
# The contract these cases come from: specs/028-macos-package/contracts/bundle-layout.md.
#
# Every case builds its own throwaway `--bin-dir` and `--out` in a temp dir and deletes them
# afterwards, so the suite touches nothing real and cases cannot leak into one another. The
# binaries are empty files: composition is what is under test, not what the binaries do.
#
# This runs on Linux. `--stage-only` composes and substitutes without invoking `codesign`, which
# is the whole reason that flag exists (research R3) -- the layout rules are the ones that break
# silently, and they are the ones a Mac is not needed to check.
#
# The invariant that carries the feature: **exactly two files in `Contents/MacOS/`**. A missing
# `micold-daemon` makes `daemon_binary()` fall through to a bare name on a `PATH` a packaged
# install does not have, so every session fails to start while the window opens normally. A third
# file is evidence of a glob, and the file it would be is `micold-showcase`.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
BUNDLE="$ROOT/scripts/macos-bundle.sh"
APP="micold-ai-ide.app"

failures=0
cases=0

ok() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  failures=$((failures + 1))
}

# Echo a fresh directory holding both shipped binaries as empty files.
new_bin_dir() {
  local dir
  dir="$(mktemp -d)"
  : > "$dir/micold-ai-ide"
  : > "$dir/micold-daemon"
  chmod 0755 "$dir/micold-ai-ide" "$dir/micold-daemon"
  echo "$dir"
}

# stage <bin_dir> <out_dir> [extra args...] -- run the producer, capture output, return its status.
stage() {
  local bin_dir="$1" out="$2"
  shift 2
  STAGE_OUT="$("$BUNDLE" --bin-dir "$bin_dir" --out "$out" --stage-only "$@" 2>&1)"
  return $?
}

# ---------------------------------------------------------------------------
echo "== composes the layout of contracts/bundle-layout.md =="

bins="$(new_bin_dir)"
out="$(mktemp -d)"
cases=$((cases + 1))
if stage "$bins" "$out"; then
  for part in \
    "Contents/Info.plist" \
    "Contents/PkgInfo" \
    "Contents/MacOS/micold-ai-ide" \
    "Contents/MacOS/micold-daemon" \
    "Contents/Resources/icon.icns"
  do
    cases=$((cases + 1))
    if [ -f "$out/$APP/$part" ]; then
      ok "$part exists"
    else
      fail "$part missing from the staged bundle"
    fi
  done

  cases=$((cases + 1))
  count="$(find "$out/$APP/Contents/MacOS" -mindepth 1 | wc -l | tr -d ' ')"
  if [ "$count" = "2" ]; then
    ok "Contents/MacOS holds exactly two entries"
  else
    fail "Contents/MacOS holds $count entries, want exactly 2:
$(find "$out/$APP/Contents/MacOS" -mindepth 1)"
  fi

  cases=$((cases + 1))
  if [ "$(cat "$out/$APP/Contents/PkgInfo")" = "APPL????" ]; then
    ok "PkgInfo is APPL????"
  else
    fail "PkgInfo is '$(cat "$out/$APP/Contents/PkgInfo")', want APPL????"
  fi

  cases=$((cases + 1))
  if [ -x "$out/$APP/Contents/MacOS/micold-ai-ide" ] && [ -x "$out/$APP/Contents/MacOS/micold-daemon" ]; then
    ok "both binaries are executable"
  else
    fail "a binary in Contents/MacOS is not executable"
  fi
else
  fail "--stage-only over a well-formed --bin-dir exited non-zero:
$STAGE_OUT"
fi
rm -rf "$bins" "$out"

# ---------------------------------------------------------------------------
echo
echo "== never ships the showcase, whatever else is in --bin-dir (FR-006) =="

bins="$(new_bin_dir)"
: > "$bins/micold-showcase"
chmod 0755 "$bins/micold-showcase"
out="$(mktemp -d)"
cases=$((cases + 1))
if stage "$bins" "$out"; then
  if [ -e "$out/$APP/Contents/MacOS/micold-showcase" ]; then
    fail "micold-showcase reached Contents/MacOS -- the copy globbed"
  elif [ "$(find "$out/$APP/Contents/MacOS" -mindepth 1 | wc -l | tr -d ' ')" != "2" ]; then
    fail "a showcase in --bin-dir changed the Contents/MacOS entry count"
  else
    ok "a showcase beside the shipped binaries is left behind"
  fi
else
  fail "--stage-only exited non-zero when --bin-dir also held a showcase:
$STAGE_OUT"
fi
rm -rf "$bins" "$out"

# ---------------------------------------------------------------------------
echo
echo "== fails loudly rather than shipping something wrong =="

# A copy of everything the producer reads, so a case can remove one part of it. `--version` is
# always passed here: the copy has no cargo workspace to read the real one from.
new_fake_root() {
  local dir
  dir="$(mktemp -d)"
  mkdir -p "$dir/scripts" "$dir/packaging/macos" "$dir/assets/icon"
  cp "$BUNDLE" "$dir/scripts/"
  cp "$ROOT/packaging/macos/Info.plist.in" "$dir/packaging/macos/"
  cp "$ROOT/assets/icon/icon.icns" "$dir/assets/icon/"
  echo "$dir"
}

# expect_failure <name> <phrase the message must contain> <bin_dir> <out> [script]
#
# The phrase is a whole message fragment, not just the file name: every one of these paths has the
# file name in it anyway, so matching the name alone would pass on the shell's own "No such file or
# directory" for the producer itself.
expect_failure() {
  local name="$1" want="$2" bin_dir="$3" out="$4" script="${5:-$BUNDLE}"
  cases=$((cases + 1))
  local msg status
  msg="$("$script" --bin-dir "$bin_dir" --out "$out" --stage-only --version 9.9.9 2>&1)"
  status=$?
  if [ "$status" -eq 0 ]; then
    fail "$name: exited 0, want non-zero"
  elif ! printf '%s' "$msg" | grep -qF -- "$want"; then
    fail "$name: message does not name '$want':
$msg"
  else
    ok "$name"
  fi
}

# The substitution actually happened.
bins="$(new_bin_dir)"
out="$(mktemp -d)"
cases=$((cases + 1))
if stage "$bins" "$out" --version 9.9.9; then
  if grep -q '@VERSION@' "$out/$APP/Contents/Info.plist"; then
    fail "@VERSION@ survived into the staged Info.plist"
  elif grep -q '9\.9\.9' "$out/$APP/Contents/Info.plist"; then
    ok "the version is substituted into Info.plist"
  else
    fail "--version 9.9.9 did not reach the staged Info.plist"
  fi
else
  fail "--stage-only --version exited non-zero:
$STAGE_OUT"
fi
rm -rf "$bins" "$out"

# ... and an unsubstituted token fails the build rather than shipping. The guard is on the token
# *shape*, not on `@VERSION@` alone, so a token added to the template but never wired into the
# substitution is caught too -- which is the only way this can actually regress.
fake="$(new_fake_root)"
sed -i.bak 's#<key>CFBundlePackageType</key>#<key>MicoldBuildDate</key><string>@BUILD_DATE@</string><key>CFBundlePackageType</key>#' "$fake/packaging/macos/Info.plist.in"
rm -f "$fake/packaging/macos/Info.plist.in.bak"
bins="$(new_bin_dir)"
out="$(mktemp -d)"
expect_failure "an unsubstituted @TOKEN@ fails the build" "unsubstituted token in Info.plist: @BUILD_DATE@" "$bins" "$out" "$fake/scripts/macos-bundle.sh"
rm -rf "$fake" "$bins" "$out"

# A missing binary is named.
bins="$(new_bin_dir)"
rm -f "$bins/micold-daemon"
out="$(mktemp -d)"
expect_failure "a missing micold-daemon is named" "missing binary: micold-daemon" "$bins" "$out"
rm -rf "$bins" "$out"

bins="$(new_bin_dir)"
rm -f "$bins/micold-ai-ide"
out="$(mktemp -d)"
expect_failure "a missing micold-ai-ide is named" "missing binary: micold-ai-ide" "$bins" "$out"
rm -rf "$bins" "$out"

# A missing icon is named.
fake="$(new_fake_root)"
rm -f "$fake/assets/icon/icon.icns"
bins="$(new_bin_dir)"
out="$(mktemp -d)"
expect_failure "a missing icon is named" "missing icon: assets/icon/icon.icns" "$bins" "$out" "$fake/scripts/macos-bundle.sh"
rm -rf "$fake" "$bins" "$out"

# ---------------------------------------------------------------------------
echo
echo "== the universal merge is a list, and names the target it is missing (R8) =="

# `scripts/macos-dmg.sh` needs macOS for `hdiutil`, so what runs here is everything it does
# *before* that: reading the per-target binaries and merging them. That ordering is deliberate --
# a release that is going to fail for a missing arm64 build should say so rather than say
# "hdiutil needs macOS", and putting the input checks first is what lets this suite see it.
#
# Why the list matters (research R8): macOS 26 is Apple's last Intel release, so the `x86_64` half
# has a known end. When the floor moves past it the merge becomes a one-element list and `lipo`
# goes away without the artifact changing name or shape. A hard-coded pair of paths would make
# that a rewrite; an array makes it a deletion.

DMG="$ROOT/scripts/macos-dmg.sh"

# Echo a cargo-style target directory holding a release build for each named target.
new_target_dir() {
  local dir
  dir="$(mktemp -d)"
  local triple
  for triple in "$@"; do
    mkdir -p "$dir/$triple/release"
    : > "$dir/$triple/release/micold-ai-ide"
    : > "$dir/$triple/release/micold-daemon"
    chmod 0755 "$dir/$triple/release/micold-ai-ide" "$dir/$triple/release/micold-daemon"
  done
  echo "$dir"
}

# dmg_expect <name> <phrase the message must contain> <target_dir> <out>
dmg_expect() {
  local name="$1" want="$2" target_dir="$3" out="$4"
  cases=$((cases + 1))
  local msg status
  msg="$("$DMG" --target-dir "$target_dir" --out "$out" --version 9.9.9 2>&1)"
  status=$?
  if [ "$status" -eq 0 ]; then
    fail "$name: exited 0, want non-zero"
  elif ! printf '%s' "$msg" | grep -qF -- "$want"; then
    fail "$name: message does not name '$want':
$msg"
  else
    ok "$name"
  fi
}

# The target list is a variable, not two spelled-out `lipo` arguments.
cases=$((cases + 1))
if grep -qE '^TARGETS=\(' "$DMG"; then
  ok "the merge is driven by a TARGETS list"
else
  fail "scripts/macos-dmg.sh does not define a TARGETS=( ) list -- dropping the Intel half would
be a rewrite of the merge instead of a deletion from a list (research R8)"
fi

# Both architectures present: validation passes, and the only thing left to stop it here is the
# platform. Anything else in the message means an input check misfired.
target_dir="$(new_target_dir aarch64-apple-darwin x86_64-apple-darwin)"
out="$(mktemp -d)"
if [ "$(uname -s)" = "Darwin" ]; then
  cases=$((cases + 1))
  ok "skipped on macOS: the platform stop this case looks for does not apply"
else
  dmg_expect "a complete target dir gets past the input checks" "hdiutil needs macOS" "$target_dir" "$out"
fi
rm -rf "$target_dir" "$out"

# One architecture missing: the message names the target, not just the file.
target_dir="$(new_target_dir x86_64-apple-darwin)"
out="$(mktemp -d)"
dmg_expect "a missing aarch64 build names its target" "aarch64-apple-darwin" "$target_dir" "$out"
rm -rf "$target_dir" "$out"

target_dir="$(new_target_dir aarch64-apple-darwin)"
out="$(mktemp -d)"
dmg_expect "a missing x86_64 build names its target" "x86_64-apple-darwin" "$target_dir" "$out"
rm -rf "$target_dir" "$out"

# One binary missing from an otherwise complete target: the message names both which and where.
target_dir="$(new_target_dir aarch64-apple-darwin x86_64-apple-darwin)"
rm -f "$target_dir/aarch64-apple-darwin/release/micold-daemon"
out="$(mktemp -d)"
dmg_expect "a missing micold-daemon names the binary and the target" "micold-daemon" "$target_dir" "$out"
rm -rf "$target_dir" "$out"

# ---------------------------------------------------------------------------
printf '\n%s cases, %s failures\n' "$cases" "$failures"
[ "$failures" -eq 0 ]
