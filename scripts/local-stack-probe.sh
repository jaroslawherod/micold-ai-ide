#!/usr/bin/env bash
# Probe a running local stack (#1732 M5, SC-008, US5.1): the seeded session
# 6f1c2a54-0000-4000-8000-000000000001 is listed over ws://127.0.0.1:<api>/api/ws, and the web root
# http://127.0.0.1:<web>/ answers 200. Retries until MICOLD_PROBE_TIMEOUT seconds (120) pass, then
# fails naming what is missing. CI's local-stack-kind job runs it; so can you, beside a stack.
#
#   MICOLD_LOCAL_API_PORT (8730)  MICOLD_LOCAL_WEB_PORT (8731)  MICOLD_PROBE_TIMEOUT (120)

set -euo pipefail
exec node - "${MICOLD_LOCAL_API_PORT:-8730}" "${MICOLD_LOCAL_WEB_PORT:-8731}" "${MICOLD_PROBE_TIMEOUT:-120}" <<'JS'
const [api, web, timeout] = process.argv.slice(2);
const SEEDED = "6f1c2a54-0000-4000-8000-000000000001";
const deadline = Date.now() + Number(timeout) * 1000;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function listed() {
  return new Promise((resolve) => {
    let ws;
    const done = (v) => { try { ws?.close(); } catch {} resolve(v); };
    const timer = setTimeout(() => done("no answer to list_sessions"), 5000);
    try { ws = new WebSocket(`ws://127.0.0.1:${api}/api/ws`); } catch (e) { clearTimeout(timer); return resolve(String(e)); }
    ws.onopen = () => ws.send(JSON.stringify({ type: "list_sessions" }));
    ws.onerror = () => { clearTimeout(timer); done(`cannot connect to ws://127.0.0.1:${api}/api/ws`); };
    ws.onmessage = (m) => {
      const msg = JSON.parse(m.data);
      if (msg.type !== "sessions") return;
      clearTimeout(timer);
      done(msg.rows.some((r) => r.id === SEEDED) ? true : `seeded session ${SEEDED} not listed`);
    };
  });
}

async function webOk() {
  const url = `http://127.0.0.1:${web}/`;
  try { const r = await fetch(url); return r.status === 200 ? true : `web root ${url} did not answer 200 (${r.status})`; }
  catch { return `web root ${url} did not answer 200 (no connection)`; }
}

let last = "";
while (Date.now() < deadline) {
  const a = await listed();
  const w = await webOk();
  if (a === true && w === true) { console.log(`local-stack-probe: ok: ${SEEDED} listed, web root 200`); process.exit(0); }
  last = [a, w].filter((x) => x !== true).join("; ");
  await sleep(1000);
}
console.error(`local-stack-probe: ${last}`);
process.exit(1);
JS
