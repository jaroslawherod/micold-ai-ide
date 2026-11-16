#!/usr/bin/env bash
# The web client's tool pins have one source of truth, mise.toml (#1735, research R7); the `client`
# job of ci.yml installs through setup-node and corepack instead of mise, so this suite fails when
# its pins drift from mise.toml: node (major), and pnpm (major, against `packageManager`).
# A self-test then proves a drifted copy is refused, so the check cannot pass vacuously.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

# problems <mise.toml> <ci.yml> <package.json>: one line per drift, nothing when the pins agree.
problems() {
  local mise_node mise_pnpm ci_node pm_pnpm
  mise_node="$(sed -n 's/^node[[:space:]]*=[[:space:]]*"\([0-9]*\).*/\1/p' "$1" | head -1)"
  mise_pnpm="$(sed -n 's/^pnpm[[:space:]]*=[[:space:]]*"\([0-9]*\).*/\1/p' "$1" | head -1)"
  ci_node="$(awk '/^  client:/ { in_job = 1; next } in_job && /^  [a-z-]+:/ { in_job = 0 } in_job' "$2" \
    | sed -n 's/.*node-version:[[:space:]]*"\{0,1\}\([0-9]*\).*/\1/p' | head -1)"
  pm_pnpm="$(sed -n 's/.*"packageManager":[[:space:]]*"pnpm@\([0-9]*\)\..*/\1/p' "$3" | head -1)"
  [ -n "$mise_node" ] || echo "mise.toml pins no node"
  [ -n "$mise_pnpm" ] || echo "mise.toml pins no pnpm"
  [ -n "$ci_node" ] || echo "the client job of ci.yml sets no node-version"
  [ -n "$pm_pnpm" ] || echo "package.json has no packageManager pnpm@<version>"
  [ "$mise_node" = "$ci_node" ] || echo "node: mise.toml says '$mise_node', the client job says '$ci_node'"
  [ "$mise_pnpm" = "$pm_pnpm" ] || echo "pnpm: mise.toml says '$mise_pnpm', package.json says '$pm_pnpm'"
}

found="$(problems mise.toml .github/workflows/ci.yml package.json)"
if [ -n "$found" ]; then fail "pins drifted:"; printf '%s\n' "$found" | sed 's/^/        /'; else pass "node and pnpm pins agree"; fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
sed 's/^node = "24"/node = "22"/' mise.toml > "$tmp/mise.toml"
if [ -n "$(problems "$tmp/mise.toml" .github/workflows/ci.yml package.json)" ]; then pass "a drifted node pin is refused"; else fail "a drifted node pin passed"; fi
sed 's/pnpm@10/pnpm@9/' package.json > "$tmp/package.json"
if [ -n "$(problems mise.toml .github/workflows/ci.yml "$tmp/package.json")" ]; then pass "a drifted pnpm pin is refused"; else fail "a drifted pnpm pin passed"; fi

[ "$failures" -eq 0 ] || exit 1
echo "client-pins: all cases passed"
