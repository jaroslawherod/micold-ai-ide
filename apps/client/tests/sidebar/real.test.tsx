// The sidebar over a feed that can lose and regain its connection (US1 scenarios 3-4, US2, FR-014a).
import React from "react";
import { AccessibilityInfo } from "react-native";
import { act, fireEvent, screen } from "@testing-library/react-native";
import { createMockApi } from "@micold/protocol/mock";
import { createSnapshotStore } from "../../src/features/sidebar/store";
import { Sidebar } from "../../src/features/sidebar/Sidebar";
import { SnapshotProvider } from "../../src/features/sidebar/SnapshotProvider";
import { localeStore } from "../../src/i18n/localeStore";
import { renderWithApi } from "../support/renderWithApi";

describe("snapshot store over a real-style feed", () => {
  it("loads again when the connection comes up after a failed first load", async () => {
    const { api } = createMockApi();
    let up = false;
    const listeners = new Set<(s: { kind: "connected" } | { kind: "offline" }) => void>();
    const real = api.orchestrator;
    const first = real.getSnapshot;
    api.orchestrator = {
      ...real,
      getSnapshot: () => (up ? first() : Promise.reject(new Error("down"))),
      connection: { get: () => (up ? { kind: "connected" } : { kind: "offline" }), subscribe: (cb) => (listeners.add(cb), () => listeners.delete(cb)) },
    };
    const store = createSnapshotStore(api);
    const stop = store.getState().start();
    await act(async () => {});
    expect(store.getState().failed).toBe(true);
    up = true;
    await act(async () => listeners.forEach((cb) => cb({ kind: "connected" })));
    expect(store.getState().failed).toBe(false);
    expect(store.getState().snapshot?.sessions.length).toBeGreaterThan(0);
    stop();
  });

  it("replaces its snapshot when the orchestrator sends a new one after a resync", async () => {
    const { api } = createMockApi();
    let push: (s: Awaited<ReturnType<typeof api.orchestrator.getSnapshot>>) => void = () => undefined;
    api.orchestrator.onSnapshot = (cb) => ((push = cb), () => undefined);
    const store = createSnapshotStore(api);
    const stop = store.getState().start();
    await act(async () => {});
    const current = store.getState().snapshot!;
    await act(async () => push({ ...current, seq: 99, sessions: [] }));
    expect(store.getState().snapshot?.seq).toBe(99);
    expect(store.getState().snapshot?.sessions).toEqual([]);
    stop();
  });
});

describe("unreachable sessions", () => {
  beforeEach(() => {
    localeStore.getState().setLocale("en");
    jest.spyOn(AccessibilityInfo, "isReduceMotionEnabled").mockResolvedValue(true);
  });
  afterEach(() => jest.restoreAllMocks());

  it("stay listed with a translated label, and open when pressed", async () => {
    const mock = createMockApi();
    const snap = await mock.api.orchestrator.getSnapshot();
    const s = snap.sessions[0]!;
    mock.control.pushChange({ kind: "upsert", entity: { type: "session", value: { ...s, reachable: false } } });
    const opened: string[] = [];
    const { view } = renderWithApi(
      <SnapshotProvider>
        <Sidebar onOpenSession={(id) => opened.push(id)} />
      </SnapshotProvider>,
      mock,
    );
    await view;
    await act(async () => {});
    expect(screen.getAllByTestId("session-unreachable")).toHaveLength(1);
    expect(screen.getAllByTestId("session-unreachable")[0]).toHaveTextContent("Unreachable");
    expect(screen.getAllByTestId("session-row")).toHaveLength(60);
    await act(async () => fireEvent.press(screen.getAllByTestId("session-row")[0]!));
    expect(opened).toEqual([s.id]);
    mock.control.pushChange({ kind: "upsert", entity: { type: "session", value: { ...s, reachable: true } } });
    await act(async () => {});
    expect(screen.queryByTestId("session-unreachable")).toBeNull();
  });
});
