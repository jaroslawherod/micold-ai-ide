// The signed-in session, in memory only (FR-006): a zustand store with no persistence, so a reload
// signs the person out and nothing token-shaped reaches web storage or a cookie. Each tab is its own
// JS realm and so its own store; `createSessionStore` makes a fresh one per app (and per test).
import { createStore, type StoreApi } from "zustand/vanilla";
import type { AuthSession } from "@micold/protocol";

export interface SessionState {
  session: AuthSession | null;
  setSession(session: AuthSession): void;
  clear(): void;
}

export type SessionStore = StoreApi<SessionState>;

export function createSessionStore(): SessionStore {
  return createStore<SessionState>()((set) => ({
    session: null,
    setSession: (session) => set({ session }),
    clear: () => set({ session: null }),
  }));
}
