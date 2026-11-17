// The orchestrator API boundary (#1735, contracts/orchestrator-api.md and transport.md). The app gets
// an `Api` only from its `ApiProvider`; the mock and the real client (#1732, #1733) both implement it.
import type { ContentEntry, ErrorCode } from "./generated/client-api";
import type { AuthProvider, Change, ConnectionState, DaemonId, ProjectId, SessionId, Snapshot } from "./types";

export type Unsubscribe = () => void;

export interface OrchestratorApi {
  /** Current snapshot, in the orchestrator's stable order. */
  getSnapshot(): Promise<Snapshot>;
  /** Changes after `sinceSeq`; the callback receives batches in sequence order. */
  subscribe(sinceSeq: number, onBatch: (changes: Change[]) => void): Unsubscribe;
  /** A full snapshot that replaces the store's copy: after a resume the orchestrator could not serve (FR-014). */
  onSnapshot(cb: (snapshot: Snapshot) => void): Unsubscribe;
  /** A session's structured content as the daemon holds it now: a snapshot, not a stream (FR-014b). */
  readSession(ref: SessionRef): Promise<ContentEntry[]>;
  /** Connection state for the banner. */
  connection: { get(): ConnectionState; subscribe(cb: (s: ConnectionState) => void): Unsubscribe };
  /** Leave `offline` and connect again now. */
  retry(): void;
}

export type { ContentEntry };

/** Where a session lives: what `read_session` needs. */
export interface SessionRef {
  daemon: DaemonId;
  project: ProjectId;
  session: SessionId;
}

/** Held in memory only (FR-006). The token is not enumerable and there is no serialiser. */
export interface AuthSession {
  readonly accessToken: string;
  readonly identity: { readonly name: string };
}
export interface AuthFailure {
  failure: "refused" | "cancelled";
}

export interface AuthService {
  /** Every provider the orchestrator knows; the sign-in page shows only the configured ones. */
  listProviders(): Promise<AuthProvider[]>;
  signIn(providerId: string): Promise<AuthSession | AuthFailure>;
  currentSession(): AuthSession | null;
  signOut(): Promise<void>;
  /** The session expired or was revoked mid-use. */
  onSessionLost(cb: () => void): Unsubscribe;
}

export interface Api {
  orchestrator: OrchestratorApi;
  auth: AuthService;
}

export function isAuthFailure(result: AuthSession | AuthFailure): result is AuthFailure {
  return "failure" in result;
}

/** A session whose token no `JSON.stringify`, spread or `Object.keys` reveals. */
export function createAuthSession(accessToken: string, identity: { name: string }): AuthSession {
  const session = { identity: Object.freeze({ ...identity }) };
  Object.defineProperty(session, "accessToken", { value: accessToken, enumerable: false });
  return Object.freeze(session) as AuthSession;
}

/** The client's own failures plus every code the orchestrator can answer with (FR-015: each maps to a translated line). */
export type ApiErrorCode = "unauthenticated" | "unavailable" | "timeout" | ErrorCode;

export class ApiError extends Error {
  constructor(readonly code: ApiErrorCode) {
    super(`orchestrator API: ${code}`);
    this.name = "ApiError";
  }
}

// Transport seam (contracts/transport.md). The wire frame format belongs to #1732.
export interface Transport {
  open(): void;
  close(): void;
  send(frame: string): void;
  onOpen(cb: () => void): void;
  /** `code` is the WebSocket close code; `fatal` is a policy close (1008): do not retry. */
  onClose(cb: (info: { clean: boolean; code?: number; fatal?: boolean }) => void): void;
  /** The page became visible again (browsers throttle timers while hidden). */
  onVisible?(cb: () => void): void;
  onMessage(cb: (frame: string) => void): void;
}
export type Handle = unknown;
export interface Clock {
  now(): number;
  setTimeout(fn: () => void, ms: number): Handle;
  clear(h: Handle): void;
}
export interface ConnectionOptions {
  baseMs: number;
  maxMs: number;
  random(): number;
  requestTimeoutMs: number;
  /** Attempts before the state turns `offline`; default 8. */
  maxAttempts?: number;
}
