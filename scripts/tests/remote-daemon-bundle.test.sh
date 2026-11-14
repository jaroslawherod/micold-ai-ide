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

# --- the packages ---------------------------------------------------------------------------------
cargo_toml="$(cat "$ROOT/crates/micold-client/Cargo.toml")"
for f in "$X86" "$ARM"; do
  expect_has "the deb assets carry $f" "$cargo_toml" "target/remote-daemon/$f\", \"usr/lib/micold-ai-ide/remote/\""
  expect_has "the rpm assets carry $f" "$cargo_toml" "target/remote-daemon/$f\", dest = \"/usr/lib/micold-ai-ide/remote/$f\""
done

# `mise run deb|rpm` package an existing build and read the daemons from the workspace's own
# target/remote-daemon (the asset paths above), so each must build them there first (issue 1001).
mise_toml="$(cat "$ROOT/mise.toml")"
body="$(awk '$0 == "[tasks.remote-daemons]" {on=1; next} on && /^\[/ {exit} on' <<<"$mise_toml")"
expect_has "the remote-daemons task runs the bundle script into target/remote-daemon" "$body" "remote-daemon-bundle.sh --out target/remote-daemon"
for task in deb rpm; do
  body="$(awk -v t="[tasks.$task]" '$0 == t {on=1; next} on && /^\[/ {exit} on' <<<"$mise_toml")"
  expect_has "the $task task builds the remote daemons first" "$body" 'depends = ["remote-daemons"]'
done

release="$(cat "$ROOT/.github/workflows/release.yml")"
ci="$(cat "$ROOT/.github/workflows/ci.yml")"
for job in deb rpm linux-daemons; do
  body="$(awk -v j="  $job:" '$0 == j {on=1; next} on && /^  [a-z-]+:$/ {exit} on' <<<"$release")"
  expect_has "the $job job in release.yml builds the Linux daemons" "$body" "remote-daemon-bundle.sh"
done
body="$(awk '$0 == "  linux-daemons:" {on=1; next} on && /^  [a-z-]+:$/ {exit} on' <<<"$ci")"
expect_has "ci.yml builds the Linux daemons once" "$body" "remote-daemon-bundle.sh"

if [ "$failures" -gt 0 ]; then
  printf '\n%d failure(s)\n' "$failures"
  exit 1
fi
