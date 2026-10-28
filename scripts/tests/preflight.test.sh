#!/usr/bin/env bash
# Drives scripts/autopilot/preflight.sh (#1441) with a PATH of stand-in tools, so what is checked is
# the decision: every missing piece is named at once, and a healthy environment passes.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
SCRIPT="$PWD/scripts/autopilot/preflight.sh"
BASH_BIN="$(command -v bash)"
failures=0
pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Stand-in tool: prints nothing, exits 0.
ok_tool() { printf '#!/bin/sh\nexit 0\n' >"$work/bin/$1"; chmod +x "$work/bin/$1"; }
# Stand-in git: ls-remote prints $LS_ERR and fails when set; config user.email prints $EMAIL.
stand_git() {
  cat >"$work/bin/git" <<'STANDIN'
#!/bin/sh
case "$1" in
  ls-remote) [ -z "${LS_ERR:-}" ] || { echo "$LS_ERR" >&2; exit 128; } ;;
  config) printf '%s' "${EMAIL:-}" ;;
  remote) echo "${REMOTE:-https://example.com/o/r.git}" ;;
esac
exit 0
STANDIN
  chmod +x "$work/bin/git"
}
healthy() { # rebuild the bin dir with every tool present
  rm -rf "$work/bin"; mkdir -p "$work/bin"
  for t in gh mise cargo python3; do ok_tool "$t"; done
  stand_git
}
# Use a PATH with only the stand-ins and the few system tools the script needs (grep).
mkdir -p "$work/sys"
ln -s "$(command -v grep)" "$work/sys/grep"
run() { env -i PATH="$work/bin:$work/sys" HOME="$work" EMAIL="${EMAIL-me@x.y}" LS_ERR="${LS_ERR:-}" \
  REMOTE="${REMOTE:-}" PREFLIGHT_NEED_LSP="${NEED_LSP:-}" "$BASH_BIN" "$SCRIPT" 2>&1 || true; }
has() { printf '%s\n' "$out" | grep -qF -- "$1"; }

healthy
out="$(run)"
if has PREFLIGHT-OK; then pass "healthy environment passes"; else fail "healthy environment passes" "$out"; fi

rm "$work/bin/gh" "$work/bin/cargo"
out="$(run)"
if has PREFLIGHT-FAILED && has "missing tools: gh cargo"; then pass "names every missing tool at once"; else fail "names every missing tool at once" "$out"; fi

healthy; out="$(NEED_LSP=1 run)"
if has "rust-analyzer"; then pass "rust-analyzer required when LSP is"; else fail "rust-analyzer required when LSP is" "$out"; fi

healthy; out="$(EMAIL= run)"
if has "user.email is not set"; then pass "missing git email named"; else fail "missing git email named" "$out"; fi

healthy
out="$(LS_ERR='Host key verification failed.' REMOTE='git@github.com:o/r.git' run)"
if has "Host key verification failed" && has "HTTPS rewrite or credential helper"; then
  pass "host key failure on git@ remote blames the missing rewrite, not SSH"
else fail "host key failure on git@ remote blames the missing rewrite, not SSH" "$out"; fi

healthy; printf '#!/bin/sh\nexit 1\n' >"$work/bin/gh"
out="$(run)"
if has "gh is not authenticated"; then pass "unauthenticated gh named"; else fail "unauthenticated gh named" "$out"; fi

[ "$failures" -eq 0 ]
