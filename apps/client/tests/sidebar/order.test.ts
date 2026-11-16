// The client's snapshot store keeps the orchestrator's order (FR-011, SC-001, US2 scenarios 2, 5).
import { daemonId, type SessionState } from "@micold/protocol";
import { createMockApi } from "@micold/protocol/mock";
import { createSnapshotStore, isStale, orderIndex, sidebarTree } from "../../src/features/sidebar/store";

const STATES: SessionState[] = ["working", "waiting", "idle", "error", "stopped"];
const flush = () => new Promise((r) => setTimeout(r, 0));

async function started() {
  const mock = createMockApi();
  let clock = 1000;
  const store = createSnapshotStore(mock.api, () => clock);
  const stop = store.getState().start();
  await flush();
  return { mock, store, stop, tick: (ms: number) => (clock += ms) };
}

describe("snapshot store", () => {
  it("loads the fixture and keeps key and index of every row across 100 scripted changes", async () => {
    const { mock, store } = await started();
    const snap = store.getState().snapshot!;
    const before = [...orderIndex(snap)];
    expect(before).toHaveLength(90);
    for (let i = 0; i < 100; i += 1) {
      mock.control.pushChange({ kind: "session-state", sessionId: snap.sessions[(i * 7) % 60]!.id, state: STATES[i % 5]! });
      expect([...orderIndex(store.getState().snapshot!)]).toEqual(before);
    }
    expect(store.getState().snapshot!.seq).toBe(100);
  });

  it("applies a batch: the final state of each row shows and order is unchanged", async () => {
    const { mock, store } = await started();
    const snap = store.getState().snapshot!;
    const before = [...orderIndex(snap)];
    const [a, b] = snap.sessions;
    mock.control.pushBatch([
      { kind: "session-state", sessionId: a!.id, state: "error" },
      { kind: "session-state", sessionId: b!.id, state: "idle" },
      { kind: "session-state", sessionId: a!.id, state: "waiting" },
    ]);
    const after = store.getState().snapshot!;
    expect([after.sessions[0]!.state, after.sessions[1]!.state]).toEqual(["waiting", "idle"]);
    expect([...orderIndex(after)]).toEqual(before);
  });

  it("groups rows by daemon and project in snapshot order", async () => {
    const { store } = await started();
    const tree = sidebarTree(store.getState().snapshot!);
    expect(tree.map((d) => d.daemon.name)).toEqual(["workstation", "build-box"]);
    expect(tree[0]!.projects.map((p) => p.key)).toEqual(["d1/d1-p1", "d1/d1-p2", "d1/d1-p3"]);
    expect(tree[0]!.projects[0]!.locations[0]!.sessions.map((s) => s.key)).toEqual([
      "d1/d1-p1/d1-p1-l1/d1-p1-l1-s1",
      "d1/d1-p1/d1-p1-l1/d1-p1-l1-s2",
    ]);
    expect(tree.flatMap((d) => d.projects.flatMap((p) => p.locations))).toHaveLength(30);
  });

  it("marks a daemon's rows stale when it goes offline or the connection is not connected, and keeps them", async () => {
    const { mock, store, tick } = await started();
    expect(isStale("online", store.getState().connection)).toBe(false);
    tick(5000);
    mock.control.pushChange({ kind: "daemon-status", daemonId: daemonId("d2"), status: "offline" });
    const tree = sidebarTree(store.getState().snapshot!);
    expect(tree[1]!.daemon.status).toBe("offline");
    expect(tree[1]!.projects).toHaveLength(3);
    expect(store.getState().updatedAt).toBe(6000);
    mock.control.dropConnection();
    expect(isStale("online", store.getState().connection)).toBe(true);
    mock.control.restoreConnection();
    expect(isStale("online", store.getState().connection)).toBe(false);
  });

  it("records a failed load and stops listening when stopped", async () => {
    const mock = createMockApi();
    mock.api.orchestrator.getSnapshot = () => Promise.reject(new Error("down"));
    const store = createSnapshotStore(mock.api);
    const stop = store.getState().start();
    await flush();
    expect(store.getState().failed).toBe(true);
    stop();
    mock.control.goOffline();
    expect(store.getState().connection.kind).toBe("connected");
  });

  it("ignores a snapshot that arrives after stop", async () => {
    const mock = createMockApi();
    const store = createSnapshotStore(mock.api);
    store.getState().start()();
    await flush();
    expect(store.getState().snapshot).toBeNull();
  });
});
