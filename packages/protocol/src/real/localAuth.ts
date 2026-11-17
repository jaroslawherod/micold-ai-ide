// The orchestrator's client API has no sign-in yet: its listener is loopback only and gated by the
// origin allow-list (FR-004); real sign-in arrives with TLS (#1733). Until then the real build
// offers one local provider, so the existing sign-in gate and flow stay in place, and the session
// it makes carries no secret.
import { createAuthSession, type AuthService, type AuthSession, type Unsubscribe } from "../api";

export const LOCAL_PROVIDER_ID = "local";

export function createLocalAuth(): AuthService {
  let session: AuthSession | null = null;
  const lost = new Set<() => void>();
  return {
    listProviders: async () => [{ id: LOCAL_PROVIDER_ID, name: "Local orchestrator", configured: true }],
    async signIn(providerId) {
      if (providerId !== LOCAL_PROVIDER_ID) return { failure: "refused" };
      session = createAuthSession("", { name: "Local" });
      return session;
    },
    currentSession: () => session,
    signOut: async () => {
      session = null;
    },
    onSessionLost: (cb): Unsubscribe => {
      lost.add(cb);
      return () => lost.delete(cb);
    },
  };
}
