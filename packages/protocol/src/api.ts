// The orchestrator API boundary (#1735, contracts/orchestrator-api.md and transport.md). The app gets
// an `Api` only from its `ApiProvider`; the mock and the real client (#1732, #1733) both implement it.
import type { AuthProvider, Change, ConnectionState, Snapshot } from "./types";

export type Unsubscribe = () => void;

export interface OrchestratorApi {
  /** Current snapshot, in the orchestrator's stable order. */
  getSnapshot(): Promise<Snapshot>;
  /** Changes after `sinceSeq`; the callback receives batches in sequence order. */
  subscribe(sinceSeq: number, onBatch: (changes: Change[]) => void): Unsubscribe;
  /** Connection state for the banner. */
  connection: { get(): ConnectionState; subscribe(cb: (s: ConnectionState) => void): Unsubscribe };
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

export type ApiErrorCode = "unauthenticated" | "unavailable" | "timeout";

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
  onClose(cb: (info: { clean: boolean }) => void): void;
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
