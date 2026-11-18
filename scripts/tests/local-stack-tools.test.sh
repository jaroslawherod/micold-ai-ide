#!/usr/bin/env bash
# The local-stack task's mise tool pins must equal scripts/install-kube-tools.sh's (CI's installer).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
line="$(awk '/^\[tasks\.local-stack\]/{f=1;next} /^\[/{f=0} f && /^tools *=/' "$ROOT/mise.toml")"
[ -n "$line" ] || { echo "FAIL: [tasks.local-stack] has no tools line"; exit 1; }
fail=0
for t in kind kubectl helm; do
  want="$(grep -E "^${t^^}_VERSION=" "$ROOT/scripts/install-kube-tools.sh" | cut -d= -f2 | tr -d v)"
  got="$(sed -nE "s/.*[{ ,]${t} *= *\"([^\"]+)\".*/\1/p" <<<"$line")"
  if [ "$got" = "$want" ]; then echo "ok: $t $got"; else echo "FAIL: $t mise=$got install-kube-tools=$want"; fail=1; fi
done
exit $fail
