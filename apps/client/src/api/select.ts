// The one place the mock and the socket are named (FR-004, FR-005, FR-013). Expo inlines
// `EXPO_PUBLIC_API_MODE` at build time: only `mock` selects the in-memory mock, which a production
// export (`real`, the default) folds away so `@micold/protocol/mock` never reaches the bundle (the
// bundle check proves it). The orchestrator address is `EXPO_PUBLIC_ORCHESTRATOR_URL` (FR-016).
import type { Api } from "@micold/protocol";
import { createRealApi } from "@micold/protocol/real";

export function selectApi(): Api {
  if (process.env.EXPO_PUBLIC_API_MODE === "mock") {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const mock = require("@micold/protocol/mock") as typeof import("@micold/protocol/mock");
    const { api, control } = mock.createMockApi();
    // The Playwright harness drives the mock through this hook; it exists only in a mock build.
    (globalThis as { __micoldMock?: unknown }).__micoldMock = control;
    return api;
  }
  return createRealApi(process.env.EXPO_PUBLIC_ORCHESTRATOR_URL || undefined);
}
