// micold-ai-ide's activity reporter for Pi (feature 029, FR-012a/FR-012b).
//
// The session service loads this file into a Pi session it starts, with `pi -e <this file>`, and
// points `MICOLD_PI_ACTIVITY_LOG` at a file of its own. Nothing installs it into Pi's extension
// directories, so a `pi` run outside the application never loads it.
//
// This is all of it. It does exactly one thing: for each event below, it appends one line to that
// log:
//
//     {"type":"<event name>","at":"<RFC 3339 time>"}
//
// It never reads, alters or sends conversation content, since no event payload is written. It
// writes to no destination but that one file. It registers no tool, command, shortcut or flag.
// The session service maps the lines to the session's activity badge
// (`crates/micold-daemon/src/activity.rs`, `pi_event`).
//
// To decline it, turn off "Show activity for Pi sessions" in Settings → Environment. Pi then
// starts without this file and the badge reads unknown.

import { appendFileSync } from "node:fs";

// Pi's own event names, as the contract maps them (specs/029-pi-cli-provider/contracts/pi-cli.md).
const EVENTS = [
  "turn_start",
  "agent_start",
  "tool_execution_start",
  "tool_execution_end",
  "agent_settled",
  "turn_end",
] as const;

// Pi treats any error thrown while an extension loads as fatal and exits, so the session would
// never start. Everything below is therefore guarded: if this Pi's extension API is not the one
// this file was written against, the badge reads unknown and the session runs (FR-012d).
export default function (pi: { on: (event: string, handler: (event: any) => void) => void }) {
  try {
    subscribe(pi);
  } catch {
    // Declined by circumstance rather than by the user; the session still starts.
  }
}

function subscribe(pi: { on: (event: string, handler: (event: any) => void) => void }) {
  const log = process.env.MICOLD_PI_ACTIVITY_LOG;
  if (!log) {
    return;
  }

  const report = (type: string) => {
    try {
      appendFileSync(log, JSON.stringify({ type, at: new Date().toISOString() }) + "\n");
    } catch {
      // Best effort. A failed write costs the badge, never the session (FR-012d).
    }
  };

  for (const type of EVENTS) {
    pi.on(type, () => report(type));
  }

  // Pi also emits `session_shutdown` when `/new`, `/resume`, `/fork` or `/reload` replaces the
  // session inside a process that keeps running. Only quitting ends it.
  pi.on("session_shutdown", (event) => {
    if (event?.reason === "quit") {
      report("session_shutdown");
    }
  });
}
