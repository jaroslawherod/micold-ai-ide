// Stable order (FR-011, data-model.md "Row identity and order"): a change mutates a row's value and
// never its position. The arrays stay in the orchestrator's order; nothing here sorts.
import type { Change, Snapshot } from "./types";

/** A row's identity: `daemon/project/location` for a location, `.../session` for a session. */
export type RowKey = string;

type Lists = Pick<Snapshot, "daemons" | "projects" | "locations" | "sessions">;
type ListName = keyof Lists;

const LIST: Record<"daemon" | "project" | "location" | "session", ListName> = {
  daemon: "daemons",
  project: "projects",
  location: "locations",
  session: "sessions",
};

function patch<T extends { id: string }>(list: T[], id: string, edit: (old: T) => T): T[] {
  const at = list.findIndex((x) => x.id === id);
  if (at < 0) return list;
  const next = [...list];
  next[at] = edit(list[at]!);
  return next;
}

function applyOne(snap: Snapshot, change: Change): Snapshot {
  const next: Snapshot = { ...snap, seq: change.seq };
  switch (change.kind) {
    case "session-state":
      next.sessions = patch(snap.sessions, change.sessionId, (s) => ({ ...s, state: change.state }));
      return next;
    case "daemon-status":
      next.daemons = patch(snap.daemons, change.daemonId, (d) => ({ ...d, status: change.status }));
      return next;
    case "upsert": {
      const name = LIST[change.entity.type];
      const list = snap[name] as { id: string }[];
      const value = change.entity.value;
      const at = list.findIndex((x) => x.id === value.id);
      const out = [...list];
      if (at >= 0) out[at] = value;
      else {
        const after = list.findIndex((x) => x.id === change.afterKey);
        out.splice(after >= 0 ? after + 1 : list.length, 0, value);
      }
      (next as unknown as Record<ListName, unknown>)[name] = out;
      return next;
    }
    case "remove": {
      const name = LIST[change.entity.type];
      const id = change.entity.id;
      (next as unknown as Record<ListName, unknown>)[name] = (snap[name] as { id: string }[]).filter((x) => x.id !== id);
      return next;
    }
  }
}

/** The snapshot after `changes`, applied in sequence order; the input is not touched. */
export function applyChanges(snap: Snapshot, changes: readonly Change[]): Snapshot {
  return [...changes].sort((a, b) => a.seq - b.seq).reduce(applyOne, snap);
}

/** Every location and session row key, in display order (a location, then its sessions). */
export function rowKeys(snap: Snapshot): RowKey[] {
  const daemonOf = new Map(snap.projects.map((p) => [p.id as string, p.daemonId as string]));
  const sessionsOf = new Map<string, string[]>();
  for (const s of snap.sessions) sessionsOf.set(s.locationId, [...(sessionsOf.get(s.locationId) ?? []), s.id]);
  const keys: RowKey[] = [];
  for (const l of snap.locations) {
    const base = `${daemonOf.get(l.projectId)}/${l.projectId}/${l.id}`;
    keys.push(base, ...(sessionsOf.get(l.id) ?? []).map((s) => `${base}/${s}`));
  }
  return keys;
}
