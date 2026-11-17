import { ApiError } from "../../src/api";
import { RealOrchestratorApi } from "../../src/real/RealOrchestratorApi";
import { daemonId, projectId, sessionId, type Change, type ConnectionState } from "../../src/types";
import { FakeClock, FakeTransport } from "../fakes";

const wireSession = (id: string, extra: Record<string, unknown> = {}) => ({
  id,
  kind: "structured",
  location_id: "l1",
  name: id,
  reachable: true,
  state: "idle",
  ...extra,
});
const wireSnapshot = (seq: number, epoch = 7) => ({
  epoch,
  seq,
  daemons: [{ id: "d1", name: "local", status: "online" }],
  projects: [{ id: "p1", daemon_id: "d1", name: "proj" }],
  locations: [{ id: "l1", project_id: "p1", name: "main", branch: "main", is_default: true }],
  sessions: [wireSession("s1"), wireSession("s2")],
});

function setup(random = () => 1) {
  const transport = new FakeTransport();
  const clock = new FakeClock();
  const api = new RealOrchestratorApi(transport, clock, { random });
  const states: ConnectionState[] = [];
  api.connection.subscribe((s) => states.push(s));
  const connect = async (seq = 10) => {
    const snapshot = api.getSnapshot();
    transport.accept();
    transport.deliver({ id: 1, type: "subscribed", epoch: 7, seq, snapshot: wireSnapshot(seq) });
    return snapshot;
  };
  return { transport, clock, api, states, connect };
}

describe("RealOrchestratorApi", () => {
  it("is connecting on the first open, then connected with a snapshot and one subscribe", async () => {
    const { api, transport, connect } = setup();
    const snapshot = api.getSnapshot();
    expect(api.connection.get()).toEqual({ kind: "connecting" });
    expect(transport.opens).toBe(1);
    transport.accept();
    expect(api.connection.get()).toEqual({ kind: "connected" });
    expect(transport.sent).toEqual([{ id: 1, type: "subscribe" }]);
    transport.deliver({ id: 1, type: "subscribed", epoch: 7, seq: 10, snapshot: wireSnapshot(10) });
    const s = await snapshot;
    expect(s.seq).toBe(10);
    expect(s.sessions.map((x) => x.id)).toEqual(["s1", "s2"]);
  });

  it("applies a changes message to the open store without a reload (US1.3)", async () => {
    const { api, transport, connect } = setup();
    await connect();
    const batches: Change[][] = [];
    api.subscribe(10, (b) => batches.push(b));
    transport.deliver({ type: "changes", epoch: 7, changes: [{ seq: 11, kind: "session_state", session_id: "s1", state: "working" }] });
    expect(batches).toEqual([[{ seq: 11, kind: "session-state", sessionId: sessionId("s1"), state: "working" }]]);
    expect((await api.getSnapshot()).sessions[0]!.state).toBe("working");
  });

  it("keeps an unreachable session listed, marked unreachable (FR-014a)", async () => {
    const { api, transport, connect } = setup();
    await connect();
    transport.deliver({ type: "changes", epoch: 7, changes: [{ seq: 11, kind: "upsert", entity: { type: "session", value: wireSession("s1", { reachable: false }) } }] });
    const sessions = (await api.getSnapshot()).sessions;
    expect(sessions).toHaveLength(2);
    expect(sessions[0]).toMatchObject({ id: "s1", reachable: false });
  });

  it("pings every 20 s and is dead at 40 s of silence", async () => {
    const { api, transport, clock, connect } = setup();
    await connect();
    clock.advance(20_000);
    expect(transport.sent.at(-1)).toMatchObject({ type: "ping" });
    transport.deliver({ id: 9, type: "pong" });
    clock.advance(20_000);
    expect(api.connection.get()).toEqual({ kind: "connected" });
    clock.advance(20_000); // 40 s since the pong
    expect(api.connection.get()).toEqual({ kind: "reconnecting", attempt: 1 });
  });

  it("backs off with full jitter, 500 ms base, 15 s ceiling, and goes offline after 8 attempts", async () => {
    const { api, transport, clock, connect } = setup(() => 1);
    await connect();
    transport.drop();
    const delays: number[] = [];
    for (let attempt = 1; attempt <= 8; attempt++) {
      expect(api.connection.get()).toEqual({ kind: "reconnecting", attempt });
      delays.push(clock.pending()[0]!);
      clock.advance(clock.pending()[0]!);
      transport.drop();
    }
    expect(delays).toEqual([500, 1000, 2000, 4000, 8000, 15000, 15000, 15000]);
    expect(api.connection.get()).toEqual({ kind: "offline" });
    expect(clock.pending()).toEqual([]);
  });

  it("jitters: a random of 0.5 halves the wait", async () => {
    const { clock, transport, connect } = setup(() => 0.5);
    await connect();
    transport.drop();
    expect(clock.pending()).toEqual([250]);
  });

  it("goes offline on a policy close with no retry, and Retry reconnects (US1.4)", async () => {
    const { api, transport, clock, connect } = setup();
    await connect();
    transport.drop(true, { code: 1008, fatal: true });
    expect(api.connection.get()).toEqual({ kind: "offline" });
    expect(clock.pending()).toEqual([]);
    expect(transport.opens).toBe(1);
    api.retry();
    expect(api.connection.get()).toEqual({ kind: "connecting" });
    expect(transport.opens).toBe(2);
  });

  it("rejects the first snapshot when the orchestrator never answers and the state is offline", async () => {
    const { api, transport } = setup();
    const snapshot = api.getSnapshot();
    transport.drop(); // attempt 1..
    for (let i = 0; i < 8; i++) {
      const [d] = (api as unknown as { clock: FakeClock }).clock.pending();
      (api as unknown as { clock: FakeClock }).clock.advance(d!);
      transport.drop();
    }
    await expect(snapshot).rejects.toEqual(new ApiError("unavailable"));
  });

  it("resumes with since {epoch, seq} and delivers each change exactly once across reconnects", async () => {
    const { api, transport, clock, connect } = setup();
    await connect();
    const seen: number[] = [];
    api.subscribe(10, (b) => seen.push(...b.map((c) => c.seq)));
    transport.deliver({ type: "changes", epoch: 7, changes: [{ seq: 11, kind: "session_state", session_id: "s1", state: "idle" }] });
    transport.drop();
    clock.advance(500);
    transport.accept();
    expect(transport.sent.at(-1)).toEqual({ id: 2, type: "subscribe", since: { epoch: 7, seq: 11 } });
    transport.deliver({ id: 2, type: "subscribed", epoch: 7, seq: 11 });
    // Replays of 11 are dropped; 12 and 13 arrive once.
    transport.deliver({ type: "changes", epoch: 7, changes: [11, 12, 13].map((seq) => ({ seq, kind: "session_state", session_id: "s1", state: "working" })) });
    expect(seen).toEqual([11, 12, 13]);
    expect(api.connection.get()).toEqual({ kind: "connected" });
  });

  it("on resync_needed subscribes with no since and the store replaces its snapshot", async () => {
    const { api, transport, clock, connect } = setup();
    await connect();
    const replaced: number[] = [];
    api.onSnapshot((s) => replaced.push(s.seq));
    transport.drop();
    clock.advance(500);
    transport.accept();
    transport.deliver({ id: 2, type: "resync_needed", epoch: 91, seq: 4, reason: "epoch_changed" });
    expect(transport.sent.at(-1)).toEqual({ id: 3, type: "subscribe" });
    transport.deliver({ id: 3, type: "subscribed", epoch: 91, seq: 4, snapshot: wireSnapshot(4, 91) });
    expect(replaced).toEqual([4]);
    expect((await api.getSnapshot()).seq).toBe(4);
  });

  it("reconnects at once when the page is visible again and the last word is stale (R6)", async () => {
    const { api, transport, clock, connect } = setup();
    await connect();
    clock.advance(10_000); // a recent word: nothing to do
    transport.becomeVisible();
    expect(transport.opens).toBe(1);
    clock.advance(10_000); // the ping went out, no pong: a whole interval of silence
    transport.becomeVisible();
    expect(api.connection.get()).toEqual({ kind: "reconnecting", attempt: 1 });
    expect(transport.opens).toBe(2);
  });

  it("reads a session's content and maps wire error codes", async () => {
    const { api, transport, connect } = setup();
    await connect();
    const ref = { daemon: daemonId("d1"), project: projectId("p1"), session: sessionId("s1") };
    const content = api.readSession(ref);
    expect(transport.sent.at(-1)).toMatchObject({ type: "read_session", daemon: "d1", project: "p1", session: "s1" });
    const id = transport.sent.at(-1)!.id;
    transport.deliver({ id, type: "session_content", session: "s1", entries: [{ seq: 1, role: "user", kind: "text", text: "hi" }] });
    await expect(content).resolves.toEqual([{ seq: 1, role: "user", kind: "text", text: "hi" }]);
    const failing = api.readSession(ref);
    transport.deliver({ id: transport.sent.at(-1)!.id, type: "error", code: "daemon_unreachable", message: "x" });
    await expect(failing).rejects.toEqual(new ApiError("daemon_unreachable"));
  });

  it("fails a read at once when not connected, and times out a silent one", async () => {
    const { api, transport, clock, connect } = setup();
    const ref = { daemon: daemonId("d1"), project: projectId("p1"), session: sessionId("s1") };
    await expect(api.readSession(ref)).rejects.toEqual(new ApiError("unavailable"));
    await connect();
    const slow = api.readSession(ref);
    clock.advance(10_000);
    await expect(slow).rejects.toEqual(new ApiError("timeout"));
    expect(transport.sent.some((f) => f.type === "read_session")).toBe(true);
  });
});
