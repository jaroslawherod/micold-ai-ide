// Holds this tab's session store, and clears it when the auth service reports the session lost
// (expiry mid-use); the guard then sends the person to sign-in with the open route as `returnTo`.
import React, { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { useStore } from "zustand";
import { useApi } from "../api/ApiProvider";
import { createSessionStore, type SessionState, type SessionStore } from "./sessionStore";

const SessionContext = createContext<SessionStore | undefined>(undefined);

export function SessionProvider({ children }: { children: ReactNode }) {
  const { auth } = useApi();
  const [store] = useState(createSessionStore);
  useEffect(() => auth.onSessionLost(() => store.getState().clear()), [auth, store]);
  return <SessionContext.Provider value={store}>{children}</SessionContext.Provider>;
}

export function useSession<T>(select: (s: SessionState) => T): T {
  const store = useContext(SessionContext);
  if (!store) throw new Error("useSession needs a SessionProvider above it");
  return useStore(store, select);
}
