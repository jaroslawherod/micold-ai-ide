#!/usr/bin/env bash
# scripts/local-stack.sh in process mode (#1732, FR-017, research R12) with a stub daemon and a stub web export:
# it seeds a project with two sessions, starts the orchestrator with the API address and both
# allowed origins, a daemon that dials it, and the web server; it refuses a second copy; Ctrl+C
# stops everything. The real daemon and export are exercised by the quickstart's visual pass.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
command -v node >/dev/null || { echo "skip  node is not installed"; exit 0; }

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

tmp="$(mktemp -d)"
trap 'kill "${stack:-0}" 2>/dev/null || true; rm -rf "$tmp"' EXIT

# A daemon that records its arguments and sleeps.
cat >"$tmp/daemon" <<'STUB'
#!/usr/bin/env bash
echo "$*" >>"$STUB_LOG"
echo "$$" >>"$STUB_LOG.pids"
exec sleep 300
STUB
chmod +x "$tmp/daemon"
mkdir "$tmp/web"
echo "<html>stack</html>" >"$tmp/web/index.html"

port() { node -e 'const s=require("net").createServer().listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})'; }
api="$(port)"
web="$(port)"
export MICOLD_LOCAL_MODE=process STUB_LOG="$tmp/log" MICOLD_LOCAL_API_PORT="$api" MICOLD_LOCAL_WEB_PORT="$web" \
  MICOLD_LOCAL_DAEMON_BIN="$tmp/daemon" MICOLD_LOCAL_WEB_DIR="$tmp/web" MICOLD_LOCAL_DIR="$tmp/data"

setsid scripts/local-stack.sh >"$tmp/out" 2>&1 &
stack=$!
for _ in $(seq 1 100); do
  curl -fs "http://127.0.0.1:$web/" >/dev/null 2>&1 && break
  sleep 0.1
done

if curl -fs "http://127.0.0.1:$web/" | grep -q stack; then pass "the web export is served on the web port"; else fail "nothing served on $web"; fi
if grep -q -- "orchestrator --data-dir $tmp/data/orchestrator --api-addr 127.0.0.1:$api --allow-origin http://127.0.0.1:$web --allow-origin http://localhost:$web" "$tmp/log"; then
  pass "the orchestrator gets the API address and both origins"
else fail "orchestrator arguments: $(cat "$tmp/log")"; fi
if grep -q -- "--config $tmp/data/daemon.toml" "$tmp/log" && grep -q "enabled = true" "$tmp/data/daemon.toml"; then
  pass "the daemon is configured to dial the orchestrator"
else fail "daemon not configured to dial"; fi
if python3 - "$tmp/data/daemon/projects.json" <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))["projects"][0]
assert len(p["sessions"]) == 2 and p["sessions"][1]["worktree_dir"] == "feature"
PY
then pass "the seed has a project, a worktree and two sessions"; else fail "seed projects.json"; fi
[ -d "$tmp/data/seed/demo/.claude/worktrees/feature" ] && pass "the worktree exists" || fail "no worktree"

if MICOLD_LOCAL_DIR="$tmp/second" scripts/local-stack.sh >"$tmp/second.out" 2>&1; then
  fail "a second copy started"
elif grep -q "port $web is already in use" "$tmp/second.out"; then pass "a second copy reports the port conflict and exits"; else fail "second copy: $(cat "$tmp/second.out")"; fi

printf '#!/usr/bin/env bash\nexit 3\n' >"$tmp/dies"
chmod +x "$tmp/dies"
if MICOLD_LOCAL_DIR="$tmp/third" MICOLD_LOCAL_API_PORT="$(port)" MICOLD_LOCAL_WEB_PORT="$(port)" MICOLD_LOCAL_DAEMON_BIN="$tmp/dies" \
  timeout 20 scripts/local-stack.sh >"$tmp/third.out" 2>&1 && grep -Eq "the (orchestrator|daemon) \(pid [0-9]+\) exited" "$tmp/third.out"; then
  pass "an exiting process is named and ends the stack"
else fail "exit not named: $(cat "$tmp/third.out")"; fi

kill -INT "-$stack" 2>/dev/null || kill -INT "$stack"
for _ in $(seq 1 50); do kill -0 "$stack" 2>/dev/null || break; sleep 0.1; done
if kill -0 "$stack" 2>/dev/null; then fail "the stack did not stop on interrupt"; else pass "the stack stops on interrupt"; fi
if curl -fs "http://127.0.0.1:$web/" >/dev/null 2>&1; then fail "the web server outlived the stack"; else pass "the web server stopped"; fi
alive=0
while read -r pid; do if kill -0 "$pid" 2>/dev/null; then alive=1; fi; done <"$tmp/log.pids"
if [ "$alive" = 1 ]; then fail "a daemon outlived the stack"; else pass "the orchestrator and the daemon stopped"; fi

exit "$failures"
