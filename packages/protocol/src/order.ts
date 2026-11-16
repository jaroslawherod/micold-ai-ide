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

/**
 * The snapshot after `changes`, applied in sequence order; the input is not touched. A change at or
 * below the snapshot's `seq` (a replay or an old one) is dropped, so state and `seq` never go back.
 */
export function applyChanges(snap: Snapshot, changes: readonly Change[]): Snapshot {
  return changes
    .filter((c) => c.seq > snap.seq)
    .sort((a, b) => a.seq - b.seq)
    .reduce((acc, c) => (c.seq > acc.seq ? applyOne(acc, c) : acc), snap);
}

function groupBy<T>(items: readonly T[], key: (item: T) => string): Map<string, T[]> {
  const out = new Map<string, T[]>();
  for (const item of items) out.set(key(item), [...(out.get(key(item)) ?? []), item]);
  return out;
}

/**
 * Every location and session row key, in display order: daemons, then each daemon's projects, then
 * each project's locations and their sessions, each level in snapshot order. Rows whose parent is
 * absent are not displayed and get no key. The client's sidebar tree follows the same order.
 */
export function rowKeys(snap: Snapshot): RowKey[] {
  const projectsOf = groupBy(snap.projects, (p) => p.daemonId);
  const locationsOf = groupBy(snap.locations, (l) => l.projectId);
  const sessionsOf = groupBy(snap.sessions, (s) => s.locationId);
  const keys: RowKey[] = [];
  for (const d of snap.daemons)
    for (const p of projectsOf.get(d.id) ?? [])
      for (const l of locationsOf.get(p.id) ?? []) {
        const base = `${d.id}/${p.id}/${l.id}`;
        keys.push(base, ...(sessionsOf.get(l.id) ?? []).map((s) => `${base}/${s.id}`));
      }
  return keys;
}
