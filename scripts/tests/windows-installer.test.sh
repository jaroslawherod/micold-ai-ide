#!/usr/bin/env bash
# Drives `scripts/windows-installer.sh` with `uname`, `cargo` and `iscc` stubbed (feature 030, FR-016).
#
# The real script only runs on a Windows runner, where a mistake costs a CI round trip and shows up
# as an installer with the wrong version or the wrong binaries. So its decisions are pinned here, on
# any host: every case gets a fresh temp dir of stubs that record their arguments, and deletes it
# afterwards. Nothing is built and nothing is installed.
#
# The contract these cases come from: specs/030-windows-installer/contracts/windows-installer.md,
# section "Build interface".

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/windows-installer.sh"

failures=0
cases=0

# new_stubs <uname -s output>: a temp dir whose bin/ holds `uname`, `cargo` and `iscc` stubs. Each
# tool stub appends its arguments, one invocation per line, to <dir>/<tool>.args. Echoes the dir.
new_stubs() {
  local dir
  dir="$(mktemp -d)"
  mkdir -p "$dir/bin"
  printf '#!/usr/bin/env bash\necho %q\n' "$1" >"$dir/bin/uname"
  local tool
  for tool in cargo iscc; do
    printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >>%q\n' "$dir/$tool.args" >"$dir/bin/$tool"
  done
  chmod +x "$dir/bin/"*
  echo "$dir"
}

# run_script <dir> [env assignments...] -- [script args...]: runs the script with the stubs first on
# PATH and the build lock off. Sets `status` and `output` (stdout and stderr together).
run_script() {
  local dir="$1"
  shift
  local envs=()
  while [ "$#" -gt 0 ] && [ "$1" != "--" ]; do
    envs+=("$1")
    shift
  done
  [ "$#" -gt 0 ] && shift
  output="$(cd "$ROOT" && env PATH="$dir/bin:$PATH" MICOLD_NO_BUILD_LOCK=1 ISCC="$dir/bin/iscc" \
    ${envs[@]+"${envs[@]}"} "$SCRIPT" "$@" 2>&1)"
  status=$?
}

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n      %s\n' "$1" "$2"
  failures=$((failures + 1))
}

# expect_refusal <name> <want output substring>: the last run_script exited non-zero and said so.
expect_refusal() {
  cases=$((cases + 1))
  if [ "$status" -eq 0 ]; then
    fail "$1" "want a non-zero exit, got 0; output: $output"
  elif ! printf '%s' "$output" | grep -qF -- "$2"; then
    fail "$1" "want output containing \`$2\`, got: $output"
  else
    pass "$1"
  fi
}

# --- host --------------------------------------------------------------------------------------

d="$(new_stubs Linux)"
run_script "$d" -- --arch x64
expect_refusal "refuses to run off Windows" 'the Windows installer is built on Windows'
rm -rf "$d"

# --- arguments ---------------------------------------------------------------------------------

# A Windows host from here on.
WINDOWS_UNAME=MINGW64_NT-10.0-26100

for arch in x86 aarch64 ARM64 ""; do
  d="$(new_stubs "$WINDOWS_UNAME")"
  run_script "$d" -- --arch "$arch"
  expect_refusal "rejects --arch '$arch'" '--arch must be x64 or arm64'
  if [ -s "$d/cargo.args" ]; then
    fail "rejects --arch '$arch' before building" "cargo ran: $(cat "$d/cargo.args")"
  fi
  rm -rf "$d"
done

# --- the build ---------------------------------------------------------------------------------

# expect_args <name> <tool> <want substring>: the last run_script succeeded and <tool> was invoked
# with arguments containing <want>.
expect_args() {
  cases=$((cases + 1))
  local got
  got="$(cat "$d/$2.args" 2>/dev/null)"
  if [ "$status" -ne 0 ]; then
    fail "$1" "want exit 0, got $status; output: $output"
  elif ! printf '%s' "$got" | grep -qF -- "$3"; then
    fail "$1" "want $2 arguments containing \`$3\`, got: ${got:-<not invoked>}"
  else
    pass "$1"
  fi
}

# The version every package takes, read independently of the script under test.
WORKSPACE_VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version *= *"\(.*\)"/\1/p' "$ROOT/Cargo.toml")"
if [ -z "$WORKSPACE_VERSION" ]; then
  fail "reads [workspace.package] version from Cargo.toml" "found none; this suite's own parse is broken"
fi

d="$(new_stubs "$WINDOWS_UNAME")"
run_script "$d" -- --arch x64 --out-dir "$d/out"
expect_args "passes the workspace version to iscc" iscc "/DAppVersion=$WORKSPACE_VERSION "
rm -rf "$d"

# The shared target dir the build compiles into; the script must point iscc at its release output.
# Resolved from $ROOT, where run_script runs the script, since CI's `CARGO_TARGET_DIR: target` is
# relative and iscc needs it absolute.
TARGET_DIR="$(cd "$ROOT" && scripts/build-lock.sh --print-target-dir)"
case "$TARGET_DIR" in
/*) ;;
*) TARGET_DIR="$ROOT/$TARGET_DIR" ;;
esac

for pair in x64:x86_64-pc-windows-msvc arm64:aarch64-pc-windows-msvc; do
  arch="${pair%%:*}" triple="${pair#*:}"
  d="$(new_stubs "$WINDOWS_UNAME")"
  run_script "$d" -- --arch "$arch" --out-dir "$d/out"
  expect_args "passes /DArch=$arch to iscc" iscc "/DArch=$arch "
  expect_args "points iscc at the $triple release binaries" iscc "/DBinDir=$TARGET_DIR/$triple/release "
  rm -rf "$d"
done

# CI sets `CARGO_TARGET_DIR: target`. iscc resolves a relative path against the .iss's own directory,
# so the script must hand it the directory cargo actually built into, resolved from where it ran.
d="$(new_stubs "$WINDOWS_UNAME")"
run_script "$d" CARGO_TARGET_DIR=target -- --arch arm64 --out-dir "$d/out"
expect_args "resolves a relative CARGO_TARGET_DIR before handing it to iscc" iscc \
  "/DBinDir=$ROOT/target/aarch64-pc-windows-msvc/release "
rm -rf "$d"

# CI passes `--out-dir dist` and then smokes `dist/*-setup.exe` from the same directory, so a relative
# --out-dir is resolved the same way.
d="$(new_stubs "$WINDOWS_UNAME")"
out_rel="$(realpath --relative-to="$ROOT" "$d")/out"
run_script "$d" -- --arch x64 --out-dir "$out_rel"
expect_args "resolves a relative --out-dir before handing it to iscc" iscc "/O$ROOT/$out_rel "
rm -rf "$d"

# Exactly the app and the daemon, never the showcase binary built from the same crate (FR-002).
for pair in x64:x86_64-pc-windows-msvc arm64:aarch64-pc-windows-msvc; do
  arch="${pair%%:*}" triple="${pair#*:}"
  d="$(new_stubs "$WINDOWS_UNAME")"
  run_script "$d" -- --arch "$arch" --out-dir "$d/out"
  expect_args "builds the app and the daemon for $triple" cargo \
    "build --release --locked -p micold-client --bin micold-ai-ide -p micold-daemon --target $triple"
  rm -rf "$d"
done

# --- the compiler ------------------------------------------------------------------------------

# Where a user gets Inno Setup; the script names it rather than installing anything (FR-016).
INNO_SETUP_DOWNLOAD=https://jrsoftware.org/isdl.php

d="$(new_stubs "$WINDOWS_UNAME")"
rm "$d/bin/iscc"
run_script "$d" ISCC= -- --arch x64 --out-dir "$d/out"
expect_refusal "names the Inno Setup download when no iscc is found" "$INNO_SETUP_DOWNLOAD"
if [ -s "$d/cargo.args" ]; then
  fail "looks for iscc before building" "cargo ran: $(cat "$d/cargo.args")"
fi
rm -rf "$d"

# -----------------------------------------------------------------------------------------------

printf '\n%d case(s), %d failure(s)\n' "$cases" "$failures"
[ "$failures" -eq 0 ]
