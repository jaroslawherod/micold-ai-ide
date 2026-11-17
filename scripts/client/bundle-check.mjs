#!/usr/bin/env node
// Bundle check for the web client's production export (#1735, FR-017, FR-018, SC-006, SC-007):
// fails when any file of the export contains the mock sentinel `__MICOLD_MOCK__` (only the mock
// modules carry it), or a terminal emulator (xterm, @xterm/*, node-pty, hterm, term.js) is in the
// export or in the dependency list (`pnpm-lock.yaml`).
// Usage: node scripts/client/bundle-check.mjs <export-dir> [--lock <pnpm-lock.yaml>]; the lockfile
// defaults to ./pnpm-lock.yaml. Exit 0 clean, 1 findings, 2 misuse.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const SENTINEL = "__MICOLD_MOCK__";
const TERMINAL = ["xterm", "node-pty", "hterm", "term.js"];
// A script, stylesheet or page of the export naming a terminal emulator (its CSS classes, its
// package path), as a whole word: `xterm-helper`, `@xterm/xterm`, `node-pty`, not `myxtermish`.
const SCANNED = /\.(?:js|css|html)$/;
const IN_EXPORT = /(?<![\w$])(?:xterm|node-pty|hterm|term\.js)(?![\w$])/;
// A lockfile package key: `  xterm@5.3.0:`, `  '@xterm/xterm@5.5.0':`, `  /node-pty@1.0.0:`.
const IN_LOCK = /^\s+'?\/?((?:@xterm\/[^@\s']+)|xterm(?:-[a-z0-9-]+)?|node-pty|hterm(?:-[a-z0-9-]+)?|term\.js)@/gm;

function files(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = join(dir, e.name);
    return e.isDirectory() ? files(p) : [p];
  });
}

function usage(msg) {
  console.error(`usage: bundle-check.mjs <export-dir> [--lock <pnpm-lock.yaml>] (${msg})`);
  process.exit(2);
}

const [dir, flag, lockArg] = process.argv.slice(2);
if (!dir || !statSync(dir, { throwIfNoEntry: false })?.isDirectory()) usage("an existing directory");
if (flag !== undefined && (flag !== "--lock" || !lockArg)) usage("--lock takes a path");
const lock = lockArg ?? "pnpm-lock.yaml";
if (!statSync(lock, { throwIfNoEntry: false })?.isFile()) usage(`no lockfile at ${lock}`);

const all = files(dir);
const findings = [];
for (const f of all) {
  const text = readFileSync(f, "latin1");
  if (text.includes(SENTINEL)) findings.push(`the mock sentinel ${SENTINEL} is in ${relative(dir, f)}`);
  if (SCANNED.test(f) && IN_EXPORT.test(text)) findings.push(`a terminal emulator is in ${relative(dir, f)}`);
}
for (const m of readFileSync(lock, "utf8").matchAll(IN_LOCK)) findings.push(`${lock} lists the terminal emulator ${m[1]}`);
if (findings.length) {
  console.error("bundle-check failed:");
  for (const f of findings) console.error(`  ${f}`);
  process.exit(1);
}
console.log(`bundle-check: ${all.length} files, no mock sentinel, no terminal emulator (${TERMINAL.join(", ")}) in the export or ${lock}`);
