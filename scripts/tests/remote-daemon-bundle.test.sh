#!/usr/bin/env bash
# The static Linux daemons every package carries for remote hosts and containers (feature 806
# FR-009, delivering 489 T082-T084): what the build script plans, where each package stages them,
# and that the package metadata and the workflows list them. Nothing here compiles or needs zig.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/remote-daemon-bundle.sh"
failures=0

ok() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n      %s\n' "$1" "$2"
  failures=$((failures + 1))
}

# expect_has <name> <haystack> <needle>
expect_has() {
  if grep -qF -- "$3" <<<"$2"; then ok "$1"; else fail "$1" "want \`$3\` in: ${2:0:200}"; fi
}

X86=micold-daemon-linux-x86_64
ARM=micold-daemon-linux-aarch64

# --- the build script ----------------------------------------------------------------------------
plan="$("$SCRIPT" --dry-run 2>&1)"
for want in x86_64-unknown-linux-musl aarch64-unknown-linux-musl "$X86" "$ARM"; do
  expect_has "--dry-run names $want" "$plan" "$want"
done

# Without cargo-zigbuild on PATH the real run stops before building, and says what to install.
if PATH=/usr/bin:/bin command -v cargo-zigbuild >/dev/null 2>&1; then
  ok "skipped: cargo-zigbuild is installed in /usr/bin"
else
  msg="$(PATH=/usr/bin:/bin "$SCRIPT" --out "$(mktemp -d)" 2>&1)"
  status=$?
  if [ "$status" -ne 0 ]; then ok "fails when cargo-zigbuild is missing"; else fail "fails when cargo-zigbuild is missing" "exit 0"; fi
  expect_has "names the missing tool" "$msg" "cargo-zigbuild is required"
fi

# --- the macOS bundle ----------------------------------------------------------------------------
remote="$(mktemp -d)"
bins="$(mktemp -d)"
out="$(mktemp -d)"
trap 'rm -rf "$remote" "$bins" "$out"' EXIT
: >"$remote/$X86"
: >"$remote/$ARM"
: >"$bins/micold-ai-ide"
: >"$bins/micold-daemon"
chmod 0755 "$bins/micold-ai-ide" "$bins/micold-daemon"
if res="$("$ROOT/scripts/macos-bundle.sh" --bin-dir "$bins" --out "$out" --stage-only --remote-dir "$remote" 2>&1)"; then
  app="$out/micold-ai-ide.app/Contents"
  for f in "$X86" "$ARM"; do
    if [ -x "$app/Resources/remote/$f" ]; then ok "the macOS bundle carries $f in Resources/remote"; else fail "the macOS bundle carries $f" "missing"; fi
  done
  n="$(find "$app/MacOS" -mindepth 1 | wc -l | tr -d ' ')"
  if [ "$n" = 2 ]; then ok "Contents/MacOS still holds exactly the two executables"; else fail "Contents/MacOS entries" "$n"; fi
else
  fail "the macOS bundle composes with --remote-dir" "$res"
fi
rm "$remote/$ARM"
res="$("$ROOT/scripts/macos-bundle.sh" --bin-dir "$bins" --out "$out" --stage-only --remote-dir "$remote" 2>&1)" &&
  fail "a missing remote daemon fails the bundle" "exit 0" || expect_has "a missing remote daemon is named" "$res" "missing remote daemon: $ARM"

# --- the Windows installer and the packages -------------------------------------------------------
iss="$(cat "$ROOT/packaging/windows/micold-ai-ide.iss")"
expect_has "the Windows setup installs $X86 to {app}\\remote" "$iss" "{#RemoteDir}\\$X86\"; DestDir: \"{app}\\remote\""
expect_has "the Windows setup installs $ARM to {app}\\remote" "$iss" "{#RemoteDir}\\$ARM\"; DestDir: \"{app}\\remote\""
expect_has "windows-installer.sh passes /DRemoteDir" "$(cat "$ROOT/scripts/windows-installer.sh")" '"/DRemoteDir='

cargo_toml="$(cat "$ROOT/crates/micold-client/Cargo.toml")"
for f in "$X86" "$ARM"; do
  expect_has "the deb assets carry $f" "$cargo_toml" "target/remote-daemon/$f\", \"usr/lib/micold-ai-ide/remote/\""
  expect_has "the rpm assets carry $f" "$cargo_toml" "target/remote-daemon/$f\", dest = \"/usr/lib/micold-ai-ide/remote/$f\""
done

release="$(cat "$ROOT/.github/workflows/release.yml")"
ci="$(cat "$ROOT/.github/workflows/ci.yml")"
for job in deb rpm linux-daemons; do
  body="$(awk -v j="  $job:" '$0 == j {on=1; next} on && /^  [a-z-]+:$/ {exit} on' <<<"$release")"
  expect_has "the $job job in release.yml builds the Linux daemons" "$body" "remote-daemon-bundle.sh"
done
# macOS and Windows never cross-link Linux themselves (zig crashes on the Windows ARM64 runner): they
# download what `linux-daemons` built.
for job in macos windows; do
  body="$(awk -v j="  $job:" '$0 == j {on=1; next} on && /^  [a-z-]+:$/ {exit} on' <<<"$release")"
  expect_has "the $job job in release.yml needs linux-daemons" "$body" "linux-daemons"
  expect_has "the $job job in release.yml downloads the Linux daemons" "$body" "download-artifact"
done
body="$(awk '$0 == "  linux-daemons:" {on=1; next} on && /^  [a-z-]+:$/ {exit} on' <<<"$ci")"
expect_has "ci.yml builds the Linux daemons once" "$body" "remote-daemon-bundle.sh"
expect_has "ci.yml hands the macOS bundle the downloaded daemons" "$ci" "--remote-dir target/remote-daemon"
expect_has "ci.yml hands the Windows installer the downloaded daemons" "$ci" "MICOLD_REMOTE_DIR: target/remote-daemon"

if [ "$failures" -gt 0 ]; then
  printf '\n%d failure(s)\n' "$failures"
  exit 1
fi
