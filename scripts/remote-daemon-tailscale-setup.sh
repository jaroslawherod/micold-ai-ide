#!/usr/bin/env bash
# Prepare a fresh Linux VPS to host a Micold daemon that is reachable only over a Tailscale tailnet
# (issue 808). Run it as root on the VPS.
#
#   scripts/remote-daemon-tailscale-setup.sh --authkey-file FILE [--hostname NAME] [--user NAME]
#                                            [--lock-down] [--dry-run]
#
# What it does, in order:
#   1. creates the unprivileged account the daemon will run as (default `micold`);
#   2. installs Tailscale (the official installer) and joins the tailnet with Tailscale SSH on, so
#      the client's own `ssh` reaches the host over the encrypted 100.x.x.x mesh;
#   3. writes `~USER/.micold/api-keys.env` (mode 600) as a template for the AI CLIs' API keys;
#   4. with --lock-down and `ufw` present, denies all incoming traffic except on `tailscale0`, so no
#      public port stays open.
#
# The daemon itself is NOT installed here and listens on no network port: the Micold app copies its
# own daemon over the SSH session (docs/user-guide/remote-daemons.md). This script only provides the
# host and the private network path. The auth key is read from a file, never from the command line,
# so it does not show up in `ps` or shell history. --dry-run prints the plan and changes nothing.
# Driven by scripts/tests/remote-daemon-tailscale-setup.test.sh.

set -euo pipefail

usage() {
	echo "usage: ${0##*/} --authkey-file FILE [--hostname NAME] [--user NAME] [--lock-down] [--dry-run]" >&2
	exit 2
}

authkey_file=""
hostname_arg=""
account="micold"
lock_down=0
dry_run=0
while [ "$#" -gt 0 ]; do
	case "$1" in
	--authkey-file)
		[ "$#" -ge 2 ] || usage
		authkey_file="$2"
		shift 2
		;;
	--hostname)
		[ "$#" -ge 2 ] || usage
		hostname_arg="$2"
		shift 2
		;;
	--user)
		[ "$#" -ge 2 ] || usage
		account="$2"
		shift 2
		;;
	--lock-down)
		lock_down=1
		shift
		;;
	--dry-run)
		dry_run=1
		shift
		;;
	*) usage ;;
	esac
done

[ -n "$authkey_file" ] || usage
[[ "$account" =~ ^[a-z_][a-z0-9_-]*$ ]] || {
	echo "invalid --user: $account" >&2
	exit 2
}
if [ -n "$hostname_arg" ] && ! [[ "$hostname_arg" =~ ^[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?$ ]]; then
	echo "invalid --hostname: $hostname_arg" >&2
	exit 2
fi

if [ "$dry_run" -eq 1 ]; then
	echo "would create the account $account (if missing)"
	echo "would install Tailscale and run: tailscale up --ssh --authkey=file:$authkey_file${hostname_arg:+ --hostname=$hostname_arg}"
	echo "would write /home/$account/.micold/api-keys.env (mode 600) if missing"
	if [ "$lock_down" -eq 1 ]; then
		echo "would run ufw: deny incoming, allow in on tailscale0, enable"
	fi
	exit 0
fi

[ "$(id -u)" -eq 0 ] || {
	echo "run as root" >&2
	exit 1
}
[ -r "$authkey_file" ] || {
	echo "cannot read --authkey-file $authkey_file" >&2
	exit 1
}

id "$account" >/dev/null 2>&1 || useradd --create-home --shell /bin/bash "$account"
home="$(getent passwd "$account" | cut -d: -f6)"

command -v tailscale >/dev/null 2>&1 || curl -fsSL https://tailscale.com/install.sh | sh

up=(tailscale up --ssh "--authkey=file:$authkey_file")
[ -z "$hostname_arg" ] || up+=("--hostname=$hostname_arg")
"${up[@]}"

install -d -m 700 -o "$account" -g "$account" "$home/.micold"
keys="$home/.micold/api-keys.env"
if [ ! -e "$keys" ]; then
	install -m 600 -o "$account" -g "$account" /dev/null "$keys"
	cat >"$keys" <<'KEYS'
# Sourced before every session when Settings > environment-include points at this file.
# Uncomment and fill in the keys your AI CLIs need.
# export ANTHROPIC_API_KEY=
# export OPENAI_API_KEY=
KEYS
fi

if [ "$lock_down" -eq 1 ]; then
	command -v ufw >/dev/null 2>&1 || {
		echo "--lock-down needs ufw; install it or close the public ports in your provider's firewall" >&2
		exit 1
	}
	ufw default deny incoming
	ufw default allow outgoing
	ufw allow in on tailscale0
	ufw --force enable
fi

echo "Done. Reach this host as ${account}@$(tailscale status --self --json 2>/dev/null | sed -n 's/.*"DNSName": *"\([^".]*\).*/\1/p' | head -n 1 || true) over the tailnet."
echo "Next: edit $keys, then add the host in Micold (Settings > Daemons > Remote over SSH)."
