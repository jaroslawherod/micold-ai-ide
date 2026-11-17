#!/usr/bin/env node
// Static server for the mock web export (#1735, T055): serves `dist-mock/`, resolving a route
// `/x` to `x.html` (expo-router's static output) and an unknown one to `index.html`, so a reload
// on any route works as it would behind the real host. Usage: node e2e/serve.mjs <dir> <port>.
import { createServer } from "node:http";
import { statSync } from "node:fs";
import { readFile, stat } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const [dir = "dist-mock", port = "8081"] = process.argv.slice(2);
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".ttf": "font/ttf", ".png": "image/png", ".ico": "image/x-icon" };

if (!statSync(join(dir, "index.html"), { throwIfNoEntry: false })?.isFile()) {
  console.error(`serve.mjs: no ${join(dir, "index.html")}; run \`pnpm web:export:mock\` first`);
  process.exit(2);
}

async function file(p) {
  return (await stat(p).catch(() => null))?.isFile() ? p : null;
}

createServer(async (req, res) => {
  try {
    await respond(req, res);
  } catch (err) {
    res.writeHead(400, { "content-type": "text/plain" });
    res.end(`bad request: ${err instanceof Error ? err.message : String(err)}`);
  }
}).listen(Number(port), "127.0.0.1");

async function respond(req, res) {
  const path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^(\.\.[/\\])+/, "");
  const base = join(dir, path);
  const hit = (await file(base)) ?? (await file(`${base}.html`)) ?? (await file(join(base, "index.html"))) ?? join(dir, "index.html");
  const body = await readFile(hit);
  res.writeHead(200, { "content-type": TYPES[extname(hit)] ?? "application/octet-stream" });
  res.end(body);
}
