#!/usr/bin/env bash
# scripts/bare-cargo-hook.sh blocks a bare cargo build and lets locked or unrelated commands through.
set -uo pipefail
hook="$(git rev-parse --show-toplevel)/scripts/bare-cargo-hook.sh"
fail=0
check() { # expected-exit command
  printf '{"tool_input":{"command":%s}}' "$(jq -Rn --arg c "$2" '$c')" | "$hook" >/dev/null 2>&1
  rc=$?
  [ "$rc" -eq "$1" ] || { echo "FAIL: want $1 got $rc for: $2"; fail=1; }
}
check 2 'cargo test -p micold-client'
check 2 'cd x && cargo build --release'
check 2 'cargo +nightly check'
check 2 'echo hi; cargo clippy'
check 2 $'cd x\ncargo test'
check 2 'cargo --locked test'
check 0 'scripts/build-lock.sh cargo test'
check 0 'mise run gate'
check 0 'cargo metadata --format-version 1'
check 0 'cargo fmt --check'
check 0 'CARGO_TARGET_DIR=/x cargo build'
check 0 'git log --oneline'
[ "$fail" -eq 0 ] && echo "ok: bare-cargo hook"
exit "$fail"
