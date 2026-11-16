// Stable order (FR-011, SC-001): changes mutate row values only; keys and indices never move.
import { applyChanges, rowKeys, sessionId, daemonId, locationId, projectId, type Change, type SessionState } from "../src";
import { buildFixture } from "../src/mock/fixture";

const STATES: SessionState[] = ["working", "waiting", "idle", "error", "stopped"];

/** A small deterministic generator, so the property test is reproducible. */
function rng(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 2 ** 32;
  };
}

function stateChanges(n: number, random: () => number, startSeq = 1): Change[] {
  const ids = buildFixture().sessions.map((s) => s.id);
  return Array.from({ length: n }, (_, i) => ({
    seq: startSeq + i,
    kind: "session-state" as const,
    sessionId: ids[Math.floor(random() * ids.length)]!,
    state: STATES[Math.floor(random() * STATES.length)]!,
  }));
}

describe("rowKeys", () => {
  it("keys locations daemon/project/location and sessions one level deeper", () => {
    const keys = rowKeys(buildFixture());
    expect(keys[0]).toBe("d1/d1-p1/d1-p1-l1");
    expect(keys[1]).toBe("d1/d1-p1/d1-p1-l1/d1-p1-l1-s1");
    expect(keys).toHaveLength(30 + 60);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe("applyChanges", () => {
  it("keeps every key at its index across 100 scripted changes, one at a time", () => {
    let snap = buildFixture();
    const before = rowKeys(snap);
    for (const change of stateChanges(100, rng(7))) {
      snap = applyChanges(snap, [change]);
      expect(rowKeys(snap)).toEqual(before);
    }
    expect(snap.seq).toBe(100);
  });

  it("keeps order for one batch of 100 changes and shows the last state of each row", () => {
    const changes = stateChanges(100, rng(11));
    const snap = applyChanges(buildFixture(), changes);
    expect(rowKeys(snap)).toEqual(rowKeys(buildFixture()));
    const last = new Map(changes.map((c) => [c.kind === "session-state" ? c.sessionId : "", c]));
    for (const [id, c] of last) {
      if (c.kind === "session-state") expect(snap.sessions.find((s) => s.id === id)?.state).toBe(c.state);
    }
  });

  it("gives the same result for distinct keys whatever their order within a batch", () => {
    const random = rng(3);
    const base = buildFixture();
    for (let run = 0; run < 50; run += 1) {
      const ids = [...base.sessions.map((s) => s.id)].sort(() => random() - 0.5).slice(0, 8);
      const batch: Change[] = ids.map((id, i) => ({ seq: i + 1, kind: "session-state", sessionId: id, state: STATES[i % 5]! }));
      const shuffled = [...batch].sort(() => random() - 0.5);
      expect(applyChanges(base, shuffled)).toEqual(applyChanges(base, batch));
    }
  });

  it("leaves the input untouched", () => {
    const base = buildFixture();
    const copy = JSON.parse(JSON.stringify(base));
    applyChanges(base, [{ seq: 1, kind: "session-state", sessionId: base.sessions[0]!.id, state: "error" }]);
    expect(base).toEqual(copy);
  });

  it("changes a daemon's status without moving its rows", () => {
    const base = buildFixture();
    const snap = applyChanges(base, [{ seq: 1, kind: "daemon-status", daemonId: daemonId("d2"), status: "offline" }]);
    expect(snap.daemons[1]!.status).toBe("offline");
    expect(rowKeys(snap)).toEqual(rowKeys(base));
  });

  it("puts an upsert after its afterKey, replaces in place, and removes without renumbering others", () => {
    const base = buildFixture();
    const first = base.sessions[0]!;
    const added = { ...first, id: sessionId("new"), name: "new" };
    let snap = applyChanges(base, [{ seq: 1, kind: "upsert", entity: { type: "session", value: added }, afterKey: first.id }]);
    expect(snap.sessions[1]!.id).toBe("new");
    snap = applyChanges(snap, [{ seq: 2, kind: "upsert", entity: { type: "session", value: { ...added, state: "error" } } }]);
    expect(snap.sessions[1]!.state).toBe("error");
    snap = applyChanges(snap, [{ seq: 3, kind: "upsert", entity: { type: "session", value: { ...added, id: sessionId("tail") } } }]);
    expect(snap.sessions.at(-1)!.id).toBe("tail");
    snap = applyChanges(snap, [
      { seq: 4, kind: "remove", entity: { type: "session", id: sessionId("new") } },
      { seq: 5, kind: "remove", entity: { type: "session", id: sessionId("absent") } },
    ]);
    expect(rowKeys(snap).filter((k) => !k.endsWith("/tail"))).toEqual(rowKeys(base));
  });

  it("applies changes of every entity type and ignores unknown session ids", () => {
    const base = buildFixture();
    const d = { ...base.daemons[0]!, name: "renamed" };
    const p = { ...base.projects[0]!, name: "renamed" };
    const l = { ...base.locations[0]!, name: "renamed" };
    const snap = applyChanges(base, [
      { seq: 1, kind: "upsert", entity: { type: "daemon", value: d } },
      { seq: 2, kind: "upsert", entity: { type: "project", value: p } },
      { seq: 3, kind: "upsert", entity: { type: "location", value: l } },
      { seq: 4, kind: "session-state", sessionId: sessionId("absent"), state: "idle" },
      { seq: 5, kind: "daemon-status", daemonId: daemonId("absent"), status: "online" },
    ]);
    expect([snap.daemons[0]!.name, snap.projects[0]!.name, snap.locations[0]!.name]).toEqual(["renamed", "renamed", "renamed"]);
    expect(snap.seq).toBe(5);
  });

  it("drops a replayed or old change at or below the snapshot's seq", () => {
    const base = applyChanges(buildFixture(), [{ seq: 5, kind: "session-state", sessionId: buildFixture().sessions[0]!.id, state: "error" }]);
    const snap = applyChanges(base, [
      { seq: 3, kind: "session-state", sessionId: base.sessions[0]!.id, state: "idle" },
      { seq: 5, kind: "session-state", sessionId: base.sessions[0]!.id, state: "working" },
    ]);
    expect(snap.sessions[0]!.state).toBe("error");
    expect(snap.seq).toBe(5);
    expect(snap).toBe(base);
  });
});

describe("rowKeys grouping", () => {
  it("groups by daemon then project, whatever the flat order, and drops orphans", () => {
    const base = buildFixture();
    const lastD2 = base.locations.at(-1)!;
    const moved = { ...base.locations[0]!, id: locationId("d1-p1-extra"), name: "extra" };
    const orphan = { ...base.locations[0]!, id: locationId("orphan"), projectId: projectId("absent") };
    const snap = applyChanges(base, [
      { seq: 1, kind: "upsert", entity: { type: "location", value: moved }, afterKey: lastD2.id },
      { seq: 2, kind: "upsert", entity: { type: "location", value: orphan } },
    ]);
    const keys = rowKeys(snap);
    expect(keys.some((k) => k.includes("orphan") || k.startsWith("undefined"))).toBe(false);
    const lastOfP1 = keys.lastIndexOf("d1/d1-p1/d1-p1-l5/d1-p1-l5-s2");
    expect(keys[lastOfP1 + 1]).toBe("d1/d1-p1/d1-p1-extra");
    expect(keys[lastOfP1 + 2]).toBe("d1/d1-p2/d1-p2-l1");
  });
});

