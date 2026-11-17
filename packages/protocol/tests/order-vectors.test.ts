// FR-012 parity: the orchestrator's computed order (crates/micold-core/tests/client_api_vectors.rs)
// and this package's reducer agree on every shared vector.
import { applyChanges, changeFromWire, rowKeys, snapshotFromWire } from "../src";
import vectors from "./vectors/sidebar-order.json";
import type { Change as WireChange, SessionState, Snapshot as WireSnapshot } from "../src/generated/client-api";

interface Vector {
  name: string;
  snapshot: WireSnapshot;
  changes: WireChange[];
  expected_keys: string[];
  expected_sessions: Record<string, { state: SessionState; reachable: boolean }>;
}

const { cases } = vectors as unknown as { cases: Vector[] };

describe("sidebar-order vectors", () => {
  it("has the grouping, offline and remove-then-upsert cases", () => {
    expect(cases.map((c) => c.name)).toEqual(
      expect.arrayContaining(["grouping and nesting with new rows", "a daemon goes offline", "remove then upsert one key"]),
    );
  });

  it.each(cases.map((c) => [c.name, c] as const))("%s: same order and states as the orchestrator", (_name, c) => {
    const after = applyChanges(snapshotFromWire(c.snapshot), c.changes.map(changeFromWire));
    expect(rowKeys(after)).toEqual(c.expected_keys);
    const sessions = Object.fromEntries(after.sessions.map((s) => [s.id, { state: s.state, reachable: s.reachable }]));
    expect(sessions).toEqual(c.expected_sessions);
  });

  it("applying the same changes one at a time gives the same order", () => {
    for (const c of cases) {
      const after = c.changes
        .map(changeFromWire)
        .reduce((snap, change) => applyChanges(snap, [change]), snapshotFromWire(c.snapshot));
      expect(rowKeys(after)).toEqual(c.expected_keys);
    }
  });
});
