import { isAuthFailure } from "../src/api";
import { createMockApi, MOCK_SENTINEL } from "../src/mock";
import type { Change, ConnectionState } from "../src/types";
import { sessionId } from "../src/types";

describe("mock orchestrator", () => {
  it("carries the bundle-check sentinel", () => {
    expect(MOCK_SENTINEL).toBe("__MICOLD_MOCK__");
  });

  it("serves the fixture: 2 daemons, 6 projects, 30 worktrees, 60 sessions, linked", async () => {
    const { api } = createMockApi();
    const snap = await api.orchestrator.getSnapshot();
    expect([snap.daemons.length, snap.projects.length, snap.locations.length, snap.sessions.length]).toEqual([2, 6, 30, 60]);
    const daemons = new Set(snap.daemons.map((d) => d.id));
    const projects = new Set(snap.projects.map((p) => p.id));
    const locations = new Set(snap.locations.map((l) => l.id));
    expect(snap.projects.every((p) => daemons.has(p.daemonId))).toBe(true);
    expect(snap.locations.every((l) => projects.has(l.projectId))).toBe(true);
    expect(snap.sessions.every((s) => locations.has(s.locationId))).toBe(true);
    expect(snap.locations.filter((l) => l.isDefault)).toHaveLength(6);
    expect(new Set(snap.sessions.map((s) => s.state)).size).toBe(5);
  });

  it("returns a copy, so a caller cannot change the mock's state", async () => {
    const { api } = createMockApi();
    const a = await api.orchestrator.getSnapshot();
    a.sessions.length = 0;
    expect((await api.orchestrator.getSnapshot()).sessions).toHaveLength(60);
  });

  it("delivers pushed changes in batches, applies them and replays after sinceSeq", async () => {
    const { api, control } = createMockApi();
    const base = (await api.orchestrator.getSnapshot()).seq;
    const batches: Change[][] = [];
    const off = api.orchestrator.subscribe(base, (b) => batches.push(b));
    const first = (await api.orchestrator.getSnapshot()).sessions[0]!;
    control.pushChange({ kind: "session-state", sessionId: first.id, state: "error" });
    control.pushBatch([
      { kind: "daemon-status", daemonId: first.id as never, status: "offline" },
      { kind: "remove", entity: { type: "session", id: sessionId("nope") } },
    ]);
    expect(batches.map((b) => b.map((c) => c.seq))).toEqual([[base + 1], [base + 2, base + 3]]);
    const snap = await api.orchestrator.getSnapshot();
    expect(snap.seq).toBe(base + 3);
    expect(snap.sessions[0]!.state).toBe("error");
    off();
    control.pushChange({ kind: "session-state", sessionId: first.id, state: "idle" });
    expect(batches).toHaveLength(2);
    const replay: Change[][] = [];
    api.orchestrator.subscribe(base + 2, (b) => replay.push(b));
    expect(replay.flat().map((c) => c.seq)).toEqual([base + 3, base + 4]);
  });

  it("applies daemon status, upsert after a key and remove", async () => {
    const { api, control } = createMockApi();
    const snap = await api.orchestrator.getSnapshot();
    const d = snap.daemons[1]!;
    const loc = snap.locations[0]!;
    control.pushChange({ kind: "daemon-status", daemonId: d.id, status: "needs-update" });
    const added = { ...snap.sessions[0]!, id: sessionId("new"), name: "new" };
    control.pushChange({ kind: "upsert", entity: { type: "session", value: added }, afterKey: snap.sessions[0]!.id });
    control.pushChange({ kind: "upsert", entity: { type: "location", value: { ...loc, name: "renamed" } } });
    control.pushChange({ kind: "upsert", entity: { type: "project", value: { ...snap.projects[0]!, id: "p-new" as never } } });
    control.pushChange({ kind: "remove", entity: { type: "daemon", id: snap.daemons[0]!.id } });
    const next = await api.orchestrator.getSnapshot();
    expect(next.daemons.map((x) => x.status)).toEqual(["needs-update"]);
    expect(next.sessions[1]!.id).toBe("new");
    expect(next.locations[0]!.name).toBe("renamed");
    expect(next.projects.at(-1)!.id).toBe("p-new");
  });

  it("drops and restores the connection, notifying subscribers", () => {
    const { api, control } = createMockApi();
    const seen: ConnectionState[] = [];
    api.orchestrator.connection.subscribe((s) => seen.push(s));
    expect(api.orchestrator.connection.get()).toEqual({ kind: "connected" });
    control.dropConnection();
    control.dropConnection();
    control.goOffline();
    control.restoreConnection();
    expect(seen).toEqual([
      { kind: "reconnecting", attempt: 1 },
      { kind: "reconnecting", attempt: 2 },
      { kind: "offline" },
      { kind: "connected" },
    ]);
  });
});

describe("mock auth", () => {
  it("reports two configured providers and one that is not", async () => {
    const { api } = createMockApi();
    const providers = await api.auth.listProviders();
    expect(providers.filter((p) => p.configured).map((p) => p.name)).toEqual(["GitHub", "Google"]);
    expect(providers.filter((p) => !p.configured)).toHaveLength(1);
  });

  it("can be configured with other providers", async () => {
    const { api, control } = createMockApi();
    control.setProviders([]);
    expect(await api.auth.listProviders()).toEqual([]);
  });

  it("signs in to a memory session and signs out", async () => {
    const { api } = createMockApi();
    const result = await api.auth.signIn("github");
    expect(isAuthFailure(result)).toBe(false);
    expect(api.auth.currentSession()).toBe(result);
    await api.auth.signOut();
    expect(api.auth.currentSession()).toBeNull();
  });

  it("refuses, cancels and refuses an unconfigured provider, with no session", async () => {
    const { api, control } = createMockApi();
    control.setSignInOutcome("refused");
    expect(await api.auth.signIn("github")).toEqual({ failure: "refused" });
    control.setSignInOutcome("cancelled");
    expect(await api.auth.signIn("github")).toEqual({ failure: "cancelled" });
    control.setSignInOutcome("ok");
    expect(await api.auth.signIn("gitlab")).toEqual({ failure: "refused" });
    expect(api.auth.currentSession()).toBeNull();
  });

  it("expires a session mid-use and tells the listeners", async () => {
    const { api, control } = createMockApi();
    await api.auth.signIn("google");
    const lost = jest.fn();
    const off = api.auth.onSessionLost(lost);
    control.expireSession();
    expect(lost).toHaveBeenCalledTimes(1);
    expect(api.auth.currentSession()).toBeNull();
    off();
    control.expireSession();
    expect(lost).toHaveBeenCalledTimes(1);
  });

  it("gives two mock instances separate sessions and distinct tokens", async () => {
    const a = createMockApi().api;
    const b = createMockApi().api;
    const sa = await a.auth.signIn("github");
    expect(b.auth.currentSession()).toBeNull();
    const sb = await b.auth.signIn("github");
    expect((sa as { accessToken: string }).accessToken).not.toBe((sb as { accessToken: string }).accessToken);
  });
});
