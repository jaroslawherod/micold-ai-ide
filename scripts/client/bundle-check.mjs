#!/usr/bin/env node
// Bundle check for the web client's production export (#1735, FR-018, SC-006): fails when any file
// of the export contains the mock sentinel `__MICOLD_MOCK__`, which only the mock modules
// (`@micold/protocol/mock`, the spike) carry. M4 (T061) adds the terminal-emulator check.
// Usage: node scripts/client/bundle-check.mjs <export-dir>. Exit 0 clean, 1 findings, 2 misuse.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const SENTINEL = "__MICOLD_MOCK__";

function files(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = join(dir, e.name);
    return e.isDirectory() ? files(p) : [p];
  });
}

const dir = process.argv[2];
if (!dir || !statSync(dir, { throwIfNoEntry: false })?.isDirectory()) {
  console.error("usage: bundle-check.mjs <export-dir> (an existing directory)");
  process.exit(2);
}
const all = files(dir);
const hits = all.filter((f) => readFileSync(f).includes(SENTINEL));
if (hits.length) {
  console.error(`bundle-check: the mock sentinel ${SENTINEL} is in the export:`);
  for (const f of hits) console.error(`  ${relative(dir, f)}`);
  process.exit(1);
}
console.log(`bundle-check: ${all.length} files, no mock sentinel`);
