export { BrowserTransport, POLICY_CLOSE, type BrowserEnv, type SocketLike } from "./BrowserTransport";
export { REAL_DEFAULTS, RealOrchestratorApi, type RealOptions } from "./RealOrchestratorApi";

/** The orchestrator address when `EXPO_PUBLIC_ORCHESTRATOR_URL` is unset: the local stack's (FR-016). */
export const DEFAULT_ORCHESTRATOR_URL = "ws://127.0.0.1:8730/api/ws";
export { createLocalAuth, LOCAL_PROVIDER_ID } from "./localAuth";

import type { Api } from "../api";
import { BrowserTransport } from "./BrowserTransport";
import { createLocalAuth } from "./localAuth";
import { RealOrchestratorApi } from "./RealOrchestratorApi";

/** The real `Api` for the orchestrator at `url`; it connects at once so the banner tells the truth. */
export function createRealApi(url: string = DEFAULT_ORCHESTRATOR_URL): Api {
  const orchestrator = new RealOrchestratorApi(new BrowserTransport(url), {
    now: () => Date.now(),
    setTimeout: (fn, ms) => setTimeout(fn, ms),
    clear: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
  });
  orchestrator.start();
  return { orchestrator, auth: createLocalAuth() };
}
