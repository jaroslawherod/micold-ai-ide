#!/usr/bin/env bash
# scripts/local-stack-probe.sh (#1732 M5, SC-008, US5.1) against stub servers: a tiny WebSocket
# server answering list_sessions, and a static web root. The probe passes when the seeded session
# is listed and the web root answers 200, fails naming what is missing, and gives up at its timeout.

set -uo pipefail
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
PROBE="$ROOT/scripts/local-stack-probe.sh"
command -v node >/dev/null || { echo "skip  node is not installed"; exit 0; }
failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }
tmp="$(mktemp -d)"
pids=()
trap 'kill "${pids[@]}" 2>/dev/null; rm -rf "$tmp"' EXIT

# stub <port> <rows json> <web status>: one HTTP server, /api/ws upgrades, / answers the status.
cat >"$tmp/stub.mjs" <<'JS'
import { createServer } from "node:http";
import { createHash } from "node:crypto";
const [port, rows, status] = process.argv.slice(2);
const frame = (s) => { const b = Buffer.from(s); const h = b.length < 126 ? Buffer.from([0x81, b.length]) : Buffer.from([0x81, 126, b.length >> 8, b.length & 255]); return Buffer.concat([h, b]); };
const srv = createServer((req, res) => { res.writeHead(Number(status)); res.end("<html>web</html>"); });
srv.on("upgrade", (req, sock) => {
  const key = createHash("sha1").update(req.headers["sec-websocket-key"] + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").digest("base64");
  sock.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${key}\r\n\r\n`);
  sock.on("data", (d) => {
    if ((d[0] & 0x0f) !== 1) return;
    const len = d[1] & 0x7f, mask = d.subarray(2, 6), body = Buffer.from(d.subarray(6, 6 + len).map((x, i) => x ^ mask[i % 4])).toString();
    const req = JSON.parse(body);
    // Like the orchestrator: a request without a numeric id is refused, not answered.
    if (typeof req.id !== "number") sock.write(frame(JSON.stringify({ type: "error", code: "bad_request", message: "missing field `id`" })));
    else if (req.type === "list_sessions") sock.write(frame(JSON.stringify({ id: req.id, type: "sessions", rows: JSON.parse(rows) })));
  });
  sock.on("error", () => {});
});
srv.listen(Number(port), "127.0.0.1");
JS
port() { node -e 'const s=require("net").createServer().listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})'; }
# start <var> <rows> <status>: sets <var> to the stub's port.
start() { local p; p="$(port)"; node "$tmp/stub.mjs" "$p" "$2" "$3" >/dev/null 2>&1 & pids+=($!); sleep 0.5; printf -v "$1" %s "$p"; }
seeded='[{"id":"6f1c2a54-0000-4000-8000-000000000001","title":"Fix the login bug"}]'

start good "$seeded" 200
if MICOLD_LOCAL_API_PORT="$good" MICOLD_LOCAL_WEB_PORT="$good" MICOLD_PROBE_TIMEOUT=10 "$PROBE" >"$tmp/o" 2>&1; then pass "passes when both answer"; else fail "good: $(cat "$tmp/o")"; fi

start empty '[]' 200
if MICOLD_LOCAL_API_PORT="$empty" MICOLD_LOCAL_WEB_PORT="$empty" MICOLD_PROBE_TIMEOUT=3 "$PROBE" >"$tmp/o" 2>&1; then fail "passed with no session"
elif grep -q "seeded session 6f1c2a54-0000-4000-8000-000000000001 not listed" "$tmp/o"; then pass "names the missing session"; else fail "empty: $(cat "$tmp/o")"; fi

start bad "$seeded" 503
if MICOLD_LOCAL_API_PORT="$bad" MICOLD_LOCAL_WEB_PORT="$bad" MICOLD_PROBE_TIMEOUT=3 "$PROBE" >"$tmp/o" 2>&1; then fail "passed with a 503 web root"
elif grep -q "web root http://127.0.0.1:$bad/ did not answer 200" "$tmp/o"; then pass "names a failing web root"; else fail "503: $(cat "$tmp/o")"; fi

dead="$(port)"
start_s=$SECONDS
MICOLD_LOCAL_API_PORT="$dead" MICOLD_LOCAL_WEB_PORT="$dead" MICOLD_PROBE_TIMEOUT=3 "$PROBE" >"$tmp/o" 2>&1
rc=$?
[ "$rc" -ne 0 ] && [ $((SECONDS - start_s)) -le 8 ] && pass "gives up at its timeout" || fail "timeout: rc $rc after $((SECONDS - start_s)) s"
exit "$failures"
