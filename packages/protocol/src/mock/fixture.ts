// The mock's fixture (SC-001 data): 2 daemons, 6 projects, 30 worktrees, 60 sessions.
import type { ContentEntry } from "../generated/client-api";
import { daemonId, locationId, projectId, sessionId, type Snapshot, type SessionState } from "../types";

const STATES: SessionState[] = ["working", "waiting", "idle", "error", "stopped"];

export function buildFixture(): Snapshot {
  const snap: Snapshot = { seq: 0, daemons: [], projects: [], locations: [], sessions: [] };
  for (let d = 1; d <= 2; d += 1) {
    const did = daemonId(`d${d}`);
    snap.daemons.push({ id: did, name: d === 1 ? "workstation" : "build-box", status: "online" });
    for (let p = 1; p <= 3; p += 1) {
      const pid = projectId(`d${d}-p${p}`);
      snap.projects.push({ id: pid, daemonId: did, name: `project-${d}${p}` });
      for (let l = 1; l <= 5; l += 1) {
        const lid = locationId(`${pid}-l${l}`);
        snap.locations.push({ id: lid, projectId: pid, name: l === 1 ? "main" : `feature-${l}`, branch: l === 1 ? "main" : `feature/${l}`, isDefault: l === 1 });
        for (let s = 1; s <= 2; s += 1) {
          const n = snap.sessions.length;
          snap.sessions.push({
            id: sessionId(`${lid}-s${s}`),
            locationId: lid,
            name: `session ${n + 1}`,
            kind: s === 1 ? "structured" : "terminal",
            state: STATES[n % STATES.length]!,
          });
        }
      }
    }
  }
  return snap;
}

/** A short canned conversation for any session of the mock (FR-014b). */
export function mockContent(id: string): ContentEntry[] {
  return [
    { seq: 1, role: "user", kind: "text", text: `Hello from ${id}` },
    { seq: 2, role: "assistant", kind: "text", text: "Hello. What shall we do?" },
  ];
}
