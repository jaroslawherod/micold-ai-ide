// The mock orchestrator and auth service (FR-005), for dev, tests and the Playwright harness only.
// The production export must never contain this module: the bundle check (scripts/client/
// bundle-check.mjs) fails on the sentinel below.
import { createAuthSession, type Api, type AuthSession, type Unsubscribe } from "../api";
import type { AuthProvider, Change, ConnectionState, Snapshot } from "../types";
import { applyChanges } from "../order";
import { buildFixture } from "./fixture";

export const MOCK_SENTINEL = "__MICOLD_MOCK__";

type ChangeInput = Change extends infer C ? (C extends Change ? Omit<C, "seq"> : never) : never;

export interface MockControl {
  pushChange(change: ChangeInput): void;
  pushBatch(changes: ChangeInput[]): void;
  setProviders(providers: AuthProvider[]): void;
  setSignInOutcome(outcome: "ok" | "refused" | "cancelled"): void;
  expireSession(): void;
  dropConnection(): void;
  goOffline(): void;
  restoreConnection(): void;
}

const DEFAULT_PROVIDERS: AuthProvider[] = [
  { id: "github", name: "GitHub", configured: true },
  { id: "google", name: "Google", configured: true },
  { id: "gitlab", name: "GitLab", configured: false },
];

let tokenCounter = 0;

export function createMockApi(): { api: Api; control: MockControl } {
  // Used at runtime, so no minifier can drop the string the bundle check looks for.
  (globalThis as Record<string, unknown>)[MOCK_SENTINEL] = true;
  let snap = buildFixture();
  const log: Change[] = [];
  const feeds = new Set<(b: Change[]) => void>();
  const connListeners = new Set<(s: ConnectionState) => void>();
  const lostListeners = new Set<() => void>();
  let connection: ConnectionState = { kind: "connected" };
  let providers = DEFAULT_PROVIDERS.map((p) => ({ ...p }));
  let outcome: "ok" | "refused" | "cancelled" = "ok";
  let session: AuthSession | null = null;

  const setConnection = (s: ConnectionState) => {
    connection = s;
    for (const cb of connListeners) cb(s);
  };
  const listen = <T>(set: Set<T>, cb: T): Unsubscribe => {
    set.add(cb);
    return () => set.delete(cb);
  };
  const pushBatch = (inputs: ChangeInput[]) => {
    const batch = inputs.map((c, i) => ({ ...c, seq: snap.seq + 1 + i }) as Change);
    snap = applyChanges(snap, batch);
    log.push(...batch);
    for (const cb of feeds) cb(batch);
  };

  const api: Api = {
    orchestrator: {
      getSnapshot: async () => JSON.parse(JSON.stringify(snap)) as Snapshot,
      subscribe(sinceSeq, onBatch) {
        const missed = log.filter((c) => c.seq > sinceSeq);
        if (missed.length) onBatch(missed);
        return listen(feeds, onBatch);
      },
      connection: { get: () => connection, subscribe: (cb) => listen(connListeners, cb) },
    },
    auth: {
      listProviders: async () => providers.map((p) => ({ ...p })),
      async signIn(providerId) {
        const provider = providers.find((p) => p.id === providerId && p.configured);
        if (!provider || outcome === "refused") return { failure: "refused" };
        if (outcome === "cancelled") return { failure: "cancelled" };
        tokenCounter += 1;
        session = createAuthSession(`mock-token-${tokenCounter}-${Math.random().toString(36).slice(2)}`, { name: "Mock User" });
        return session;
      },
      currentSession: () => session,
      signOut: async () => {
        session = null;
      },
      onSessionLost: (cb) => listen(lostListeners, cb),
    },
  };

  const control: MockControl = {
    pushChange: (c) => pushBatch([c]),
    pushBatch,
    setProviders: (next) => {
      providers = next.map((p) => ({ ...p }));
    },
    setSignInOutcome: (next) => {
      outcome = next;
    },
    expireSession: () => {
      session = null;
      for (const cb of lostListeners) cb();
    },
    dropConnection: () => setConnection({ kind: "reconnecting", attempt: connection.kind === "reconnecting" ? connection.attempt + 1 : 1 }),
    goOffline: () => setConnection({ kind: "offline" }),
    restoreConnection: () => setConnection({ kind: "connected" }),
  };
  return { api, control };
}
export { buildFixture } from "./fixture";
