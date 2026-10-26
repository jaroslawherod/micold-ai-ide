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
  python3 -I - "$1" <<'PY'
import sys, tomllib

with open(sys.argv[1], "rb") as f:
    tools = tomllib.load(f).get("tools", {})

rust = tools.get("rust")
if rust is None:
    print("[tools] declares no rust")
else:
    comps = rust.get("components", "") if isinstance(rust, dict) else ""
    if isinstance(comps, list):
        comps = ",".join(comps)
    have = {c.strip() for c in comps.split(",") if c.strip()}
    for want in ("rustfmt", "clippy", "rust-analyzer"):
        if want not in have:
            print(f"[tools] rust does not install the component {want}")
if "uv" not in tools:
    print("[tools] declares no uv")
PY
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
