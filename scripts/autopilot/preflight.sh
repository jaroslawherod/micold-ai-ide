#!/usr/bin/env bash
# Preflight for speckit-autopilot: can this environment complete the flow? Run it before any fetch
# or claim, so a missing piece leaves no label, comment or claim on the issue.
#
# Usage: preflight.sh        (PREFLIGHT_NEED_LSP=1 also requires rust-analyzer)
#
# Checks everything and names every problem at once. Prints PREFLIGHT-OK (exit 0), or
# PREFLIGHT-FAILED "this environment cannot run the flow, because:" and one line per problem
# (exit 1). The merge step uses `gh pr merge`, so `gh` is the merge route.
set -uo pipefail

problems=()
add() { problems+=("$1"); }

tools=(git gh mise cargo python3)
[ "${PREFLIGHT_NEED_LSP:-}" = 1 ] && tools+=(rust-analyzer)
missing=()
for t in "${tools[@]}"; do command -v "$t" >/dev/null 2>&1 || missing+=("$t"); done
[ ${#missing[@]} -eq 0 ] || add "missing tools: ${missing[*]}"

have() { command -v "$1" >/dev/null 2>&1; }

if have mise; then
  # `mise trust --show` also lists ancestor configs: judge only this repo's own line.
  root="$(git rev-parse --show-toplevel 2>/dev/null)"; [ -n "$root" ] || root="$PWD"
  shown="${root/#$HOME/\~}"
  while IFS= read -r line; do
    case "$line" in
      "$root: "* | "$shown: "*)
        case "$line" in *untrusted*) add "mise config is untrusted: run 'mise trust'" ;; esac ;;
    esac
  done < <(mise trust --show 2>&1)
fi

if have git; then
  if out="$(git ls-remote origin HEAD 2>&1 >/dev/null)"; then :; else
    msg="git ls-remote origin HEAD failed: $out"
    url="$(git remote get-url origin 2>/dev/null || true)"
    case "$out" in
      *"Host key verification failed"*)
        case "$url" in
          git@* | ssh://*) msg="$msg (remote is $url: the HTTPS rewrite or credential helper is missing, not SSH)" ;;
        esac ;;
    esac
    add "$msg"
  fi
  [ -n "$(git config user.email 2>/dev/null)" ] || add "git config user.email is not set"
fi

if have gh; then
  gh auth status >/dev/null 2>&1 || add "gh is not authenticated (gh auth status fails): no merge route"
fi

if [ ${#problems[@]} -eq 0 ]; then
  echo "PREFLIGHT-OK"
  exit 0
fi
echo "PREFLIGHT-FAILED this environment cannot run the flow, because:"
for p in "${problems[@]}"; do echo "  - $p"; done
exit 1
