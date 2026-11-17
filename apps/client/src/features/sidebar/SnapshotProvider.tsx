// One snapshot store per `Api`, started while mounted, shared by the sidebar and the Overview.
import React, { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { useStore, type StoreApi } from "zustand";
import { useApi } from "../../api/ApiProvider";
import { createSnapshotStore, type SnapshotState } from "./store";

const SnapshotContext = createContext<StoreApi<SnapshotState> | undefined>(undefined);

export function SnapshotProvider({ children }: { children: ReactNode }) {
  const api = useApi();
  const [store] = useState(() => createSnapshotStore(api));
  useEffect(() => store.getState().start(), [store]);
  return <SnapshotContext.Provider value={store}>{children}</SnapshotContext.Provider>;
}

export function useSnapshot<T>(select: (s: SnapshotState) => T): T {
  const store = useContext(SnapshotContext);
  if (!store) throw new Error("useSnapshot needs a SnapshotProvider above it");
  return useStore(store, select);
}
