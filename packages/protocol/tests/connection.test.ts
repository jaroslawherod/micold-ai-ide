import { ApiError } from "../src/api";
import { ConnectionManager } from "../src/connection";
import type { ConnectionState } from "../src/types";
import { FakeClock, FakeTransport } from "./fakes";

function setup(random = 0.5, maxAttempts?: number) {
  const transport = new FakeTransport();
  const clock = new FakeClock();
  const states: ConnectionState[] = [];
  const manager = new ConnectionManager(transport, clock, {
    baseMs: 100,
    maxMs: 1000,
    random: () => random,
    requestTimeoutMs: 500,
    maxAttempts,
  });
  manager.subscribe((s) => states.push(s));
  return { transport, clock, manager, states };
}

describe("ConnectionManager", () => {
  it("is offline until the transport opens, then connected", () => {
    const { transport, manager } = setup();
    expect(manager.get()).toEqual({ kind: "offline" });
    manager.start();
    expect(transport.opens).toBe(1);
    transport.accept();
    expect(manager.get()).toEqual({ kind: "connected" });
  });

  it("backs off with full jitter, capped at maxMs, and counts attempts", () => {
    const { transport, clock, manager, states } = setup(0.5);
    manager.start();
    transport.accept();
    const delays: number[] = [];
    for (let n = 1; n <= 6; n += 1) {
      transport.drop();
      expect(manager.get()).toEqual({ kind: "reconnecting", attempt: n });
      delays.push(...clock.pending());
      clock.advance(10_000);
    }
    // 0.5 * min(1000, 100 * 2^(n-1))
    expect(delays).toEqual([50, 100, 200, 400, 500, 500]);
    expect(transport.opens).toBe(7);
    expect(states.at(-1)).toEqual({ kind: "reconnecting", attempt: 6 });
  });

  it("a failed open also counts as an attempt; attempts reset on open", () => {
    const { transport, clock, manager } = setup(0);
    manager.start();
    transport.drop();
    expect(manager.get()).toEqual({ kind: "reconnecting", attempt: 1 });
    clock.advance(0);
    transport.drop();
    expect(manager.get()).toEqual({ kind: "reconnecting", attempt: 2 });
    clock.advance(0);
    transport.accept();
    transport.drop();
    expect(manager.get()).toEqual({ kind: "reconnecting", attempt: 1 });
  });

  it("goes offline after maxAttempts and retryNow restarts", () => {
    const { transport, clock, manager } = setup(0, 2);
    manager.start();
    transport.drop();
    clock.advance(0);
    transport.drop();
    clock.advance(0);
    transport.drop();
    expect(manager.get()).toEqual({ kind: "offline" });
    expect(clock.pending()).toEqual([]);
    const opens = transport.opens;
    manager.retryNow();
    expect(transport.opens).toBe(opens + 1);
    transport.accept();
    expect(manager.get()).toEqual({ kind: "connected" });
  });

  it("resumes by the last applied seq and drops frames it already has", () => {
    const { transport, clock, manager } = setup(0);
    const seen: number[] = [];
    manager.onEvent((frame) => seen.push(frame.seq));
    manager.start();
    transport.accept();
    expect(transport.sent).toEqual([]);
    transport.deliver({ seq: 1 });
    transport.deliver({ seq: 2 });
    transport.deliver({ seq: 2 });
    transport.deliver({ seq: 1 });
    expect(seen).toEqual([1, 2]);
    transport.drop();
    clock.advance(0);
    transport.accept();
    expect(transport.sent).toEqual([{ method: "resume", params: { sinceSeq: 2 } }]);
    expect(manager.lastSeq).toBe(2);
  });

  it("asks for a fresh snapshot on a gap and takes its seq", async () => {
    const { transport, manager } = setup();
    const snapshots: unknown[] = [];
    manager.onSnapshot((s) => snapshots.push(s));
    manager.start();
    transport.accept();
    transport.deliver({ seq: 1 });
    transport.deliver({ seq: 5 });
    const req = transport.sent.at(-1)!;
    expect(req.method).toBe("snapshot");
    transport.deliver({ id: req.id, result: { seq: 7 } });
    await Promise.resolve();
    expect(snapshots).toEqual([{ seq: 7 }]);
    expect(manager.lastSeq).toBe(7);
  });

  it("correlates responses by id", async () => {
    const { transport, manager } = setup();
    manager.start();
    transport.accept();
    const a = manager.request("a", { x: 1 });
    const b = manager.request("b");
    const [fa, fb] = transport.sent;
    expect(fa).toMatchObject({ method: "a", params: { x: 1 } });
    expect(fa!.id).not.toBe(fb!.id);
    transport.deliver({ id: fb!.id, result: "B" });
    transport.deliver({ id: fa!.id, result: "A" });
    transport.deliver({ id: 999, result: "stray" });
    await expect(a).resolves.toBe("A");
    await expect(b).resolves.toBe("B");
  });

  it("rejects an error response with its code", async () => {
    const { transport, manager } = setup();
    manager.start();
    transport.accept();
    const p = manager.request("x");
    transport.deliver({ id: transport.sent[0]!.id, error: "unauthenticated" });
    await expect(p).rejects.toEqual(new ApiError("unauthenticated"));
  });

  it("rejects a request with timeout when no answer comes", async () => {
    const { transport, clock, manager } = setup();
    manager.start();
    transport.accept();
    const p = manager.request("slow");
    clock.advance(500);
    await expect(p).rejects.toMatchObject({ code: "timeout" });
  });

  it("rejects pending requests with unavailable when the connection drops, and refuses while down", async () => {
    const { transport, manager } = setup();
    manager.start();
    transport.accept();
    const p = manager.request("x");
    transport.drop();
    await expect(p).rejects.toMatchObject({ code: "unavailable" });
    await expect(manager.request("y")).rejects.toMatchObject({ code: "unavailable" });
  });

  it("ignores frames that are not JSON objects", () => {
    const { transport, manager } = setup();
    const seen: unknown[] = [];
    manager.onEvent((f) => seen.push(f));
    manager.start();
    transport.accept();
    transport.deliverRaw("not json");
    transport.deliverRaw("42");
    expect(seen).toEqual([]);
  });

  it("stop closes the transport, cancels the retry and stays offline", () => {
    const { transport, clock, manager } = setup();
    manager.start();
    transport.drop();
    manager.stop();
    expect(transport.closes).toBe(1);
    expect(clock.pending()).toEqual([]);
    transport.drop(true);
    expect(manager.get()).toEqual({ kind: "offline" });
  });

  it("unsubscribe stops state callbacks", () => {
    const { transport, manager } = setup();
    const seen: ConnectionState[] = [];
    const off = manager.subscribe((s) => seen.push(s));
    off();
    manager.start();
    transport.accept();
    expect(seen).toEqual([]);
  });

  it("keeps one snapshot request in flight across two gaps and never moves seq back", async () => {
    const { transport, manager } = setup();
    manager.start();
    transport.accept();
    transport.deliver({ seq: 1 });
    transport.deliver({ seq: 5 });
    transport.deliver({ seq: 9 });
    const snaps = transport.sent.filter((f) => f.method === "snapshot");
    expect(snaps).toHaveLength(1);
    transport.deliver({ seq: 2 });
    transport.deliver({ id: snaps[0]!.id, result: { seq: 1 } });
    await Promise.resolve();
    expect(manager.lastSeq).toBe(2);
  });

  it("asks again for a snapshot that failed, on the next open", async () => {
    const { transport, clock, manager } = setup(0);
    manager.start();
    transport.accept();
    transport.deliver({ seq: 3 });
    transport.drop();
    await Promise.resolve();
    clock.advance(0);
    transport.sent = [];
    transport.accept();
    expect(transport.sent.map((f) => f.method)).toEqual(["resume", "snapshot"]);
  });

  it("maps an unknown error code to unavailable", async () => {
    const { transport, manager } = setup();
    manager.start();
    transport.accept();
    const p = manager.request("x");
    transport.deliver({ id: transport.sent[0]!.id, error: "boom" });
    await expect(p).rejects.toMatchObject({ code: "unavailable" });
  });

  it("reports an unauthenticated answer so the auth service can drop the session", async () => {
    const { transport, manager } = setup();
    const lost = jest.fn();
    manager.onUnauthenticated(lost);
    manager.start();
    transport.accept();
    const p = manager.request("x");
    transport.deliver({ id: transport.sent[0]!.id, error: "unauthenticated" });
    await expect(p).rejects.toMatchObject({ code: "unauthenticated" });
    expect(lost).toHaveBeenCalledTimes(1);
  });

  it("retryNow does nothing while connected or while an open is in flight", () => {
    const { transport, manager } = setup();
    manager.start();
    manager.retryNow();
    expect(transport.opens).toBe(1);
    transport.accept();
    manager.retryNow();
    expect(transport.opens).toBe(1);
  });
});
