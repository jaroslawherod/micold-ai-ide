#!/usr/bin/env bash
#
# Cargo runner for test binaries when the gate runs as root, as it does in cloud containers.
#
# Several tests check that an unreadable file or an undeletable directory is refused. Root reads and
# removes through any permission bits, so as root those tests fail on a correct tree. This runner
# executes each test binary as a normal user instead, so the gate means the same thing as root as it
# does on a developer machine. scripts/gate.sh sets it as cargo's target runner for its root_only
# tests when uid is 0; the rest of the suite needs the root-owned build folder and stays as root.
#
# The user is $MICOLD_TEST_USER, else the first of `claude` and `nobody` that exists. It gets a fresh
# HOME and TMPDIR it owns, and a git identity there, because the tests create git repositories.
# GIT_* variables are dropped: an inherited GIT_CONFIG_* or GIT_SSL_CAINFO can name files under
# /root, which the user cannot read, and git then fails every command.

set -euo pipefail

user=${MICOLD_TEST_USER:-}
if [ -z "$user" ]; then
	for u in claude nobody; do
		if id -u "$u" >/dev/null 2>&1; then
			user=$u
			break
		fi
	done
fi
[ -n "$user" ] || {
	echo "run-unprivileged: no non-root user to run tests as; set MICOLD_TEST_USER" >&2
	exit 1
}

# Under /tmp, not $TMPDIR: a session TMPDIR can sit below a 0700 directory the user cannot enter.
home=$(mktemp -d /tmp/micold-test-home.XXXXXX)
trap 'rm -rf -- "$home"' EXIT
mkdir "$home/tmp"
printf '[user]\n\tname = Gate\n\temail = gate@localhost\n' >"$home/.gitconfig"
chown -R "$user" "$home"
chmod 755 "$home"

keep=()
while IFS= read -r -d '' kv; do
	case $kv in
	GIT_* | HOME=* | TMPDIR=* | USER=* | LOGNAME=*) ;;
	*) keep+=("$kv") ;;
	esac
done < <(env -0)

runuser -u "$user" -- env -i "${keep[@]}" HOME="$home" TMPDIR="$home/tmp" USER="$user" LOGNAME="$user" "$@"
