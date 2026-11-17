// STAND-IN: hand-written entity types for the web client (#1735, data-model.md). #1732's generated
// types replace this file (constitution V); keep every import of them going through this package.

declare const brand: unique symbol;
/** An opaque id that cannot be passed where another kind of id is expected. */
export type Brand<B extends string> = string & { readonly [brand]: B };

export type DaemonId = Brand<"DaemonId">;
export type ProjectId = Brand<"ProjectId">;
export type LocationId = Brand<"LocationId">;
export type SessionId = Brand<"SessionId">;

export const daemonId = (raw: string): DaemonId => raw as DaemonId;
export const projectId = (raw: string): ProjectId => raw as ProjectId;
export const locationId = (raw: string): LocationId => raw as LocationId;
export const sessionId = (raw: string): SessionId => raw as SessionId;

export type DaemonStatus = "online" | "offline" | "needs-update";
export type SessionKind = "structured" | "terminal";
export type SessionState = "working" | "waiting" | "idle" | "error" | "stopped";

export interface Daemon {
  id: DaemonId;
  name: string;
  status: DaemonStatus;
}
export interface Project {
  id: ProjectId;
  daemonId: DaemonId;
  name: string;
}
/** A worktree. */
export interface Location {
  id: LocationId;
  projectId: ProjectId;
  name: string;
  branch: string;
  isDefault: boolean;
}
export interface Session {
  id: SessionId;
  locationId: LocationId;
  name: string;
  kind: SessionKind;
  state: SessionState;
  /** False while its daemon is offline (FR-014a); absent means reachable. */
  reachable?: boolean;
}
export interface AuthProvider {
  id: string;
  name: string;
  configured: boolean;
}

export type ConnectionState =
  | { kind: "connected" }
  | { kind: "reconnecting"; attempt: number }
  | { kind: "offline" };

/** Arrays are in the orchestrator's order; the client never sorts them. */
export interface Snapshot {
  seq: number;
  daemons: Daemon[];
  projects: Project[];
  locations: Location[];
  sessions: Session[];
}

export type Entity =
  | { type: "daemon"; value: Daemon }
  | { type: "project"; value: Project }
  | { type: "location"; value: Location }
  | { type: "session"; value: Session };

export type EntityRef =
  | { type: "daemon"; id: DaemonId }
  | { type: "project"; id: ProjectId }
  | { type: "location"; id: LocationId }
  | { type: "session"; id: SessionId };

export type Change =
  | { seq: number; kind: "session-state"; sessionId: SessionId; state: SessionState }
  | { seq: number; kind: "daemon-status"; daemonId: DaemonId; status: DaemonStatus }
  | { seq: number; kind: "upsert"; entity: Entity; afterKey?: string }
  | { seq: number; kind: "remove"; entity: EntityRef };

/** Exhaustiveness guard for a `switch` over a literal union. */
export function assertNever(value: never): never {
  throw new Error(`unexpected value: ${JSON.stringify(value)}`);
}
