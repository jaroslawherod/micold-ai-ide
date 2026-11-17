import type { Api, AuthService, OrchestratorApi } from "../src/api";
import { ApiError, createAuthSession, isAuthFailure } from "../src/api";
import { assertNever, type ConnectionState, type Snapshot } from "../src/types";

const empty: Snapshot = { seq: 0, daemons: [], projects: [], locations: [], sessions: [] };

function stubApi(): Api {
  const state: ConnectionState = { kind: "connected" };
  const orchestrator: OrchestratorApi = {
    getSnapshot: async () => empty,
    subscribe: () => () => undefined,
    connection: { get: () => state, subscribe: () => () => undefined },
  };
  const auth: AuthService = {
    listProviders: async () => [],
    signIn: async () => ({ failure: "cancelled" }),
    currentSession: () => null,
    signOut: async () => undefined,
    onSessionLost: () => () => undefined,
  };
  return { orchestrator, auth };
}

describe("protocol interfaces", () => {
  it("are implementable by a stub", async () => {
    const api = stubApi();
    await expect(api.orchestrator.getSnapshot()).resolves.toEqual(empty);
    expect(isAuthFailure(await api.auth.signIn("github"))).toBe(true);
  });

  it("keeps the access token out of any serialised form", () => {
    const session = createAuthSession("tok-secret", { name: "Ada" });
    expect(session.accessToken).toBe("tok-secret");
    expect("toJSON" in session).toBe(false);
    expect(JSON.stringify(session)).not.toContain("tok-secret");
    expect(Object.keys(session)).not.toContain("accessToken");
    expect(isAuthFailure(session)).toBe(false);
  });

  it("carries an error code", () => {
    const err = new ApiError("timeout");
    expect(err).toBeInstanceOf(Error);
    expect(err.code).toBe("timeout");
  });

  it("assertNever throws on a value the union does not allow", () => {
    expect(() => assertNever({ kind: "bogus" } as never)).toThrow(/unexpected/);
  });
});
