// A state change from the feed reaches the rendered indicator within 100 ms (SC-002), and no row moves.
import React from "react";
import { AccessibilityInfo } from "react-native";
import { act, screen } from "@testing-library/react-native";
import { Sidebar } from "../../src/features/sidebar/Sidebar";
import { SnapshotProvider } from "../../src/features/sidebar/SnapshotProvider";
import { localeStore } from "../../src/i18n/localeStore";
import { renderWithApi } from "../support/renderWithApi";

describe("feed to indicator latency", () => {
  beforeEach(() => {
    localeStore.getState().setLocale("en");
    jest.spyOn(AccessibilityInfo, "isReduceMotionEnabled").mockResolvedValue(true);
  });
  afterEach(() => jest.restoreAllMocks());

  it("shows each change within 100 ms with every row index unchanged", async () => {
    const { mock, view } = renderWithApi(
      <SnapshotProvider>
        <Sidebar />
      </SnapshotProvider>,
    );
    await view;
    await act(async () => {});
    const snap = await mock.api.orchestrator.getSnapshot();
    const rows = () => screen.getAllByText(/^session \d+$/).map((t) => t.props.children as string);
    const before = rows();
    const target = 7;
    for (const state of ["working", "waiting", "idle"] as const) {
      const start = performance.now();
      await act(async () => mock.control.pushChange({ kind: "session-state", sessionId: snap.sessions[target]!.id, state }));
      const text = screen.getAllByTestId("state-text")[target]!;
      const elapsed = performance.now() - start;
      expect(text).toHaveTextContent(state === "waiting" ? "Waiting for you" : state[0]!.toUpperCase() + state.slice(1));
      expect(elapsed).toBeLessThan(100);
      expect(rows()).toEqual(before);
    }
  });
});
