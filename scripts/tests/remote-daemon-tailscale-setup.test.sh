#!/usr/bin/env bash
# scripts/remote-daemon-tailscale-setup.sh (issue 808): argument checks and the --dry-run plan.
# Nothing here installs, joins a tailnet or needs root.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/remote-daemon-tailscale-setup.sh"
failures=0

ok() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n      %s\n' "$1" "$2"
  failures=$((failures + 1))
}
expect_has() {
  if grep -qF -- "$3" <<<"$2"; then ok "$1"; else fail "$1" "want \`$3\` in: ${2:0:200}"; fi
}
expect_exit() { # <name> <want> <got>
  if [ "$2" -eq "$3" ]; then ok "$1"; else fail "$1" "exit $3, want $2"; fi
}

plan="$("$SCRIPT" --authkey-file /tmp/k --hostname vps1 --tag tag:micold --lock-down --dry-run 2>&1)"
expect_exit "--dry-run succeeds without root" 0 $?
expect_has "plan joins with Tailscale SSH" "$plan" "tailscale up --ssh --authkey=file:/tmp/k --hostname=vps1"
expect_has "plan advertises the tag" "$plan" "--advertise-tags=tag:micold"
expect_has "plan sources the keys from .bashrc" "$plan" "/home/micold/.bashrc"
expect_has "plan writes the key template" "$plan" "/home/micold/.micold/api-keys.env (mode 600)"
expect_has "plan locks the public ports" "$plan" "allow in on tailscale0"

plan="$("$SCRIPT" --authkey-file /tmp/k --dry-run 2>&1)"
if grep -qF ufw <<<"$plan"; then fail "no ufw step without --lock-down" "$plan"; else ok "no ufw step without --lock-down"; fi

"$SCRIPT" --dry-run >/dev/null 2>&1
expect_exit "the auth key file is required" 2 $?
"$SCRIPT" --authkey-file /tmp/k --user 'bad user' --dry-run >/dev/null 2>&1
expect_exit "a bad --user is refused" 2 $?
"$SCRIPT" --authkey-file /tmp/k --hostname '-x' --dry-run >/dev/null 2>&1
expect_exit "a bad --hostname is refused" 2 $?
"$SCRIPT" --authkey-file /tmp/k --tag micold --dry-run >/dev/null 2>&1
expect_exit "a bad --tag is refused" 2 $?
"$SCRIPT" --authkey-file /tmp/k --bogus >/dev/null 2>&1
expect_exit "an unknown flag is refused" 2 $?

if grep -q -- '--authkey [^=]' "$SCRIPT"; then fail "the key never goes on the command line" "found --authkey <value>"; else ok "the key never goes on the command line"; fi

if [ "$failures" -ne 0 ]; then
  echo "remote-daemon-tailscale-setup: $failures failure(s)"
  exit 1
fi
echo "remote-daemon-tailscale-setup: all assertions passed"
