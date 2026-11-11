#!/usr/bin/env bash
# Proves the release rule for the retired `MICOLD_*` start-up variables end to end (#1537, FR-012,
# SC-001): a `micold-daemon` built WITHOUT the `test-env` feature ignores `MICOLD_LISTEN_ADDR`,
# `MICOLD_HOME` and `MICOLD_DATA_DIR`, takes the configuration file's values, and logs one warning
# per variable naming the key that replaces it. Every unit test builds with `test-env` on, so this
# is the only place the release behaviour is seen. `TEST_ENV = true` in retired_env.rs fails it.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() { printf 'FAIL  %s\n      %s\n' "$1" "$2"; failures=$((failures + 1)); }

# `cargo build -p` (no dev-dependencies) leaves `test-env` off: the daemon's self dev-dependency
# that turns it on applies to its tests only.
(cd "$ROOT" && scripts/build-lock.sh cargo build -q -p micold-daemon) \
  || { echo "FAIL  build micold-daemon"; exit 1; }
target="$(cd "$ROOT" && scripts/build-lock.sh --print-target-dir)"
bin="$target/debug/micold-daemon"
[ -x "$bin" ] || { echo "FAIL  $bin is not built"; exit 1; }

work="$(mktemp -d)"
pid=""
trap '[ -n "$pid" ] && kill "$pid" 2>/dev/null; rm -rf "$work"' EXIT

free_port() {
  python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}
file_port="$(free_port)"
env_port="$(free_port)"
while [ "$env_port" = "$file_port" ]; do env_port="$(free_port)"; done

mkdir -p "$work/file-data" "$work/file-home" "$work/env-data" "$work/env-home"
printf 'token\n' >"$work/token"
chmod 0600 "$work/token"
cat >"$work/daemon.toml" <<TOML
schema = 1
[daemon]
name = "release-rule"
kind = "container"
data_dir = "$work/file-data"
home = "$work/file-home"
[listen]
tcp = "127.0.0.1:$file_port"
[auth]
client_token_file = "$work/token"
TOML

MICOLD_LISTEN_ADDR="127.0.0.1:$env_port" MICOLD_HOME="$work/env-home" \
  MICOLD_DATA_DIR="$work/env-data" "$bin" --config "$work/daemon.toml" \
  >"$work/stdout" 2>&1 </dev/null &
pid=$!

# With a data directory set, the daemon logs to `<data_dir>/micold-daemon.log`.
log="$work/file-data/micold-daemon.log"
for _ in $(seq 1 100); do
  grep -qs 'listening (sandboxed)' "$log" && break
  kill -0 "$pid" 2>/dev/null || break
  sleep 0.1
done

if grep -qs "listening (sandboxed) addr=127.0.0.1:$file_port" "$log"; then
  pass "listen.tcp from the file wins over MICOLD_LISTEN_ADDR"
else
  fail "listen.tcp from the file wins over MICOLD_LISTEN_ADDR" \
    "no 'listening' line for port $file_port; $(head -c 400 "$work/stdout" "$log" 2>/dev/null)"
fi
if [ -s "$log" ] && [ -z "$(ls -A "$work/env-data")" ]; then
  pass "daemon.data_dir from the file wins over MICOLD_DATA_DIR"
else
  fail "daemon.data_dir from the file wins over MICOLD_DATA_DIR" \
    "log missing under the file's data dir, or the variable's dir was written: $(ls -A "$work/env-data")"
fi
if [ -z "$(ls -A "$work/env-home")" ]; then
  pass "daemon.home from the file wins over MICOLD_HOME"
else
  fail "daemon.home from the file wins over MICOLD_HOME" "the variable's dir was written"
fi

for pair in MICOLD_LISTEN_ADDR:listen.tcp MICOLD_HOME:daemon.home MICOLD_DATA_DIR:daemon.data_dir; do
  var="${pair%%:*}"; key="${pair#*:}"
  n="$(grep -c "WARN .*$var is no longer read.*\`$key\`" "$log" 2>/dev/null)"
  if [ "$n" = 1 ]; then pass "one warning for $var naming $key"; else fail "one warning for $var naming $key" "found $n"; fi
done

[ "$failures" -eq 0 ]
