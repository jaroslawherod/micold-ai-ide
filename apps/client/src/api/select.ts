// The one place the mock is named (FR-004, FR-005). Expo inlines `EXPO_PUBLIC_API_MODE` at build
// time, so in a production export (`real`) the condition is a constant, Metro folds the branch away
// and `@micold/protocol/mock` never reaches the bundle (the bundle check proves it).
import type { Api } from "@micold/protocol";

export const REAL_CLIENT_MISSING = "real client not built (#1732)";

export function selectApi(): Api {
  if (process.env.EXPO_PUBLIC_API_MODE === "mock") {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const mock = require("@micold/protocol/mock") as typeof import("@micold/protocol/mock");
    const { api, control } = mock.createMockApi();
    // The Playwright harness drives the mock through this hook; it exists only in a mock build.
    (globalThis as { __micoldMock?: unknown }).__micoldMock = control;
    return api;
  }
  throw new Error(REAL_CLIENT_MISSING);
}
