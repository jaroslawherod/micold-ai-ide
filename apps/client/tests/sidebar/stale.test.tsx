// Offline and stale daemons stay listed with their last known rows (FR-013, US2 scenario 3).
import React from "react";
import { AccessibilityInfo } from "react-native";
import { act, screen } from "@testing-library/react-native";
import { daemonId } from "@micold/protocol";
import { Sidebar } from "../../src/features/sidebar/Sidebar";
import { SnapshotProvider } from "../../src/features/sidebar/SnapshotProvider";
import { localeStore } from "../../src/i18n/localeStore";
import { renderWithApi } from "../support/renderWithApi";

async function shown() {
  const { mock, view } = renderWithApi(
    <SnapshotProvider>
      <Sidebar />
    </SnapshotProvider>,
  );
  await view;
  await act(async () => {});
  return mock;
}
const labels = () => screen.getAllByRole("button").map((b) => b.props.accessibilityLabel as string);

describe("stale marking", () => {
  beforeEach(() => {
    localeStore.getState().setLocale("en");
    jest.spyOn(AccessibilityInfo, "isReduceMotionEnabled").mockResolvedValue(true);
  });
  afterEach(() => jest.restoreAllMocks());

  it("marks an offline daemon's rows stale, keeps them listed, and moves no other row", async () => {
    const mock = await shown();
    const before = labels();
    expect(screen.queryByTestId("stale-note")).toBeNull();
    await act(async () => mock.control.pushChange({ kind: "daemon-status", daemonId: daemonId("d2"), status: "offline" }));
    const after = labels();
    expect(after).toHaveLength(30);
    expect(after.slice(0, 15)).toEqual(before.slice(0, 15));
    expect(after.slice(15).every((l) => l.endsWith("last known state"))).toBe(true);
    expect(screen.getAllByTestId("stale-note")).toHaveLength(1);
    expect(screen.getByTestId("stale-note")).toHaveTextContent(/^Last known data, updated /);
    expect(screen.getByText(/Offline/)).toBeTruthy();
  });

  it("marks every row stale while the connection is not connected, with last known state", async () => {
    const mock = await shown();
    const firstSession = screen.getAllByTestId("state-text")[0]!.props.children;
    await act(async () => mock.control.dropConnection());
    expect(labels().every((l) => l.endsWith("last known state"))).toBe(true);
    expect(screen.getAllByTestId("stale-note")).toHaveLength(2);
    expect(screen.getAllByTestId("state-text")[0]!.props.children).toBe(firstSession);
    await act(async () => mock.control.restoreConnection());
    expect(screen.queryByTestId("stale-note")).toBeNull();
  });

  it("names a daemon that needs an update, and says it in Polish", async () => {
    localeStore.getState().setLocale("pl");
    const mock = await shown();
    await act(async () => mock.control.pushChange({ kind: "daemon-status", daemonId: daemonId("d1"), status: "needs-update" }));
    expect(screen.getByText(/Wymaga aktualizacji/)).toBeTruthy();
    await act(async () => mock.control.goOffline());
    expect(screen.getAllByTestId("stale-note")[0]).toHaveTextContent(/^Ostatnie znane dane, zaktualizowane /);
    expect(labels()[0]).toMatch(/ostatni znany stan$/);
  });
});
