// The orchestrator's wire types (generated from the Rust types into ./generated/client-api.ts, #1732)
// mapped onto this package's client types: snake_case fields and kinds become the camelCase the
// client uses. Pure renames; no row is reordered, added or dropped (FR-012).
import type * as Wire from "./generated/client-api";
import {
  daemonId,
  locationId,
  projectId,
  sessionId,
  type Change,
  type Daemon,
  type DaemonStatus,
  type Entity,
  type EntityRef,
  type Location,
  type Project,
  type Session,
  type Snapshot,
} from "./types";

const status = (s: Wire.DaemonStatus): DaemonStatus => (s === "needs_update" ? "needs-update" : s);

const daemon = (d: Wire.Daemon): Daemon => ({ id: daemonId(d.id), name: d.name, status: status(d.status) });
const project = (p: Wire.Project): Project => ({ id: projectId(p.id), daemonId: daemonId(p.daemon_id), name: p.name });
const location = (l: Wire.Location): Location => ({
  id: locationId(l.id),
  projectId: projectId(l.project_id),
  name: l.name,
  branch: l.branch,
  isDefault: l.is_default,
});
const session = (s: Wire.Session): Session => ({
  id: sessionId(s.id),
  locationId: locationId(s.location_id),
  name: s.name,
  kind: s.kind,
  state: s.state,
  reachable: s.reachable,
});

function entity(e: Wire.Entity): Entity {
  switch (e.type) {
    case "daemon":
      return { type: "daemon", value: daemon(e.value) };
    case "project":
      return { type: "project", value: project(e.value) };
    case "location":
      return { type: "location", value: location(e.value) };
    case "session":
      return { type: "session", value: session(e.value) };
  }
}

function entityRef(e: Wire.EntityRef): EntityRef {
  switch (e.type) {
    case "daemon":
      return { type: "daemon", id: daemonId(e.id) };
    case "project":
      return { type: "project", id: projectId(e.id) };
    case "location":
      return { type: "location", id: locationId(e.id) };
    case "session":
      return { type: "session", id: sessionId(e.id) };
  }
}

/** A wire snapshot as the client's snapshot, rows in the same order. */
export function snapshotFromWire(s: Wire.Snapshot): Snapshot {
  return {
    seq: s.seq,
    daemons: s.daemons.map(daemon),
    projects: s.projects.map(project),
    locations: s.locations.map(location),
    sessions: s.sessions.map(session),
  };
}

/** A wire change as the client's change. */
export function changeFromWire(c: Wire.Change): Change {
  switch (c.kind) {
    case "session_state":
      return { seq: c.seq, kind: "session-state", sessionId: sessionId(c.session_id), state: c.state };
    case "daemon_status":
      return { seq: c.seq, kind: "daemon-status", daemonId: daemonId(c.daemon_id), status: status(c.status) };
    case "upsert":
      return c.after_key == null
        ? { seq: c.seq, kind: "upsert", entity: entity(c.entity) }
        : { seq: c.seq, kind: "upsert", entity: entity(c.entity), afterKey: c.after_key };
    case "remove":
      return { seq: c.seq, kind: "remove", entity: entityRef(c.entity) };
  }
}
