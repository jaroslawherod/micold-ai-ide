#!/usr/bin/env bash
# Feature 1439 (FR-008, quickstart A3): this repository declares through `mise.toml` every tool its
# gate and its instructions need, so `mise install` alone provides them in a container session:
# `rust` with the components `rustfmt`, `clippy` (the gate) and `rust-analyzer` (the `LSP` tool),
# and `uv`.
#
# Checks the repository's `mise.toml`, then that a copy without `rust-analyzer` is refused, so the
# check cannot pass vacuously.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

# Prints one line per problem with the `[tools]` table of the mise.toml given; nothing when it
# declares rust with the three components, and uv.
problems() {
  # The `[tools]` table's lines, up to the next table header (no TOML parser: the gate runs on hosts
  # whose python3 predates tomllib).
  local tools rust comps want
  tools="$(awk '/^\[/ { in_tools = ($0 == "[tools]"); next } in_tools' "$1")"
  rust="$(printf '%s\n' "$tools" | grep -E '^[[:space:]]*rust[[:space:]]*=' || true)"
  if [ -z "$rust" ]; then
    echo "[tools] declares no rust"
  else
    comps="$(printf '%s\n' "$rust" | sed -n 's/.*components[[:space:]]*=[[:space:]]*\(.*\)/\1/p' | tr -d '"[]}' | tr ',' ' ')"
    for want in rustfmt clippy rust-analyzer; do
      case " $comps " in
        *" $want "*) ;;
        *) echo "[tools] rust does not install the component $want" ;;
      esac
    done
  fi
  if ! printf '%s\n' "$tools" | grep -qE '^[[:space:]]*uv[[:space:]]*='; then
    echo "[tools] declares no uv"
  fi
}

out="$(problems mise.toml)"
if [ -z "$out" ]; then
  pass "mise.toml declares rust with rustfmt, clippy and rust-analyzer, and uv"
else
  fail "mise.toml: $out"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
sed 's/,rust-analyzer//; s/rust-analyzer,//' mise.toml >"$tmp/mise.toml"
if problems "$tmp/mise.toml" | grep -q 'rust-analyzer'; then
  pass "a mise.toml whose rust lacks rust-analyzer is refused"
else
  fail "a mise.toml whose rust lacks rust-analyzer is not refused"
fi

[ "$failures" -eq 0 ]
