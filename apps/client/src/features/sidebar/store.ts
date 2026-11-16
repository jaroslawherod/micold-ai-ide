// The client's copy of the orchestrator snapshot (FR-010, FR-011, FR-013; data-model.md "Row
// identity and order"). Changes from the feed update row values in place; nothing here sorts, so a
// row keeps its index for as long as it exists.
import { createStore, type StoreApi } from "zustand/vanilla";
import {
  applyChanges,
  rowKeys,
  type Api,
  type ConnectionState,
  type Daemon,
  type DaemonStatus,
  type Location,
  type Project,
  type RowKey,
  type Session,
  type Snapshot,
} from "@micold/protocol";

export interface SnapshotState {
  snapshot: Snapshot | null;
  connection: ConnectionState;
  /** The first snapshot could not be loaded. */
  failed: boolean;
  /** Client time of the last snapshot or change applied: the age of the "last known" data. */
  updatedAt: number | null;
  /** Load the snapshot and follow the feed; returns the stop function. */
  start(): () => void;
}

export function createSnapshotStore(api: Api, now: () => number = Date.now): StoreApi<SnapshotState> {
  const { orchestrator } = api;
  return createStore<SnapshotState>()((set, get) => ({
    snapshot: null,
    connection: orchestrator.connection.get(),
    failed: false,
    updatedAt: null,
    start() {
      let stopped = false;
      let unsubscribeFeed = () => {};
      const unsubscribeConnection = orchestrator.connection.subscribe((connection) => set({ connection }));
      orchestrator.getSnapshot().then(
        (first) => {
          if (stopped) return;
          set({ snapshot: first, failed: false, updatedAt: now() });
          unsubscribeFeed = orchestrator.subscribe(first.seq, (batch) => {
            const current = get().snapshot;
            if (current) set({ snapshot: applyChanges(current, batch), updatedAt: now() });
          });
        },
        () => {
          if (!stopped) set({ failed: true });
        },
      );
      return () => {
        stopped = true;
        unsubscribeFeed();
        unsubscribeConnection();
      };
    },
  }));
}

/** Row key to index, in display order (location, then its sessions). */
export function orderIndex(snapshot: Snapshot): Map<RowKey, number> {
  return new Map(rowKeys(snapshot).map((key, i) => [key, i]));
}

/** A daemon's rows are stale while it is offline or the orchestrator connection is not up (FR-013). */
export function isStale(status: DaemonStatus, connection: ConnectionState): boolean {
  return status === "offline" || connection.kind !== "connected";
}

export interface SessionRowModel {
  key: RowKey;
  session: Session;
}
export interface LocationRowModel {
  key: RowKey;
  location: Location;
  project: Project;
  sessions: SessionRowModel[];
}
export interface ProjectGroup {
  key: RowKey;
  project: Project;
  locations: LocationRowModel[];
}
export interface DaemonGroup {
  key: RowKey;
  daemon: Daemon;
  projects: ProjectGroup[];
}

const trees = new WeakMap<Snapshot, DaemonGroup[]>();

/** Daemons, their projects, worktrees and sessions, each in snapshot order (memoised per snapshot). */
export function sidebarTree(snapshot: Snapshot): DaemonGroup[] {
  const cached = trees.get(snapshot);
  if (cached) return cached;
  const daemons = new Map<string, DaemonGroup>();
  for (const daemon of snapshot.daemons) daemons.set(daemon.id, { key: daemon.id, daemon, projects: [] });
  const projects = new Map<string, ProjectGroup>();
  for (const project of snapshot.projects) {
    const parent = daemons.get(project.daemonId);
    if (!parent) continue;
    const group: ProjectGroup = { key: `${parent.key}/${project.id}`, project, locations: [] };
    parent.projects.push(group);
    projects.set(project.id, group);
  }
  const locations = new Map<string, LocationRowModel>();
  for (const location of snapshot.locations) {
    const parent = projects.get(location.projectId);
    if (!parent) continue;
    const row: LocationRowModel = { key: `${parent.key}/${location.id}`, location, project: parent.project, sessions: [] };
    parent.locations.push(row);
    locations.set(location.id, row);
  }
  for (const session of snapshot.sessions) {
    const parent = locations.get(session.locationId);
    if (parent) parent.sessions.push({ key: `${parent.key}/${session.id}`, session });
  }
  const tree = [...daemons.values()];
  trees.set(snapshot, tree);
  return tree;
}
