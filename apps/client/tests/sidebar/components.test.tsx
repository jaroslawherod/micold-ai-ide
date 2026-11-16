// StateIndicator, LocationRow and Sidebar (US2 scenarios 1, 4, 6; FR-010, FR-011, FR-019; Edge Cases).
import React from "react";
import { Animated, AccessibilityInfo } from "react-native";
import { act, fireEvent, screen } from "@testing-library/react-native";
import { buildFixture, createMockApi } from "@micold/protocol/mock";
import { StateIndicator } from "../../src/components/StateIndicator";
import { LocationRow } from "../../src/components/LocationRow";
import { Sidebar } from "../../src/features/sidebar/Sidebar";
import { SnapshotProvider } from "../../src/features/sidebar/SnapshotProvider";
import { localeStore } from "../../src/i18n/localeStore";
import { sidebarTree } from "../../src/features/sidebar/store";
import { renderWithApi } from "../support/renderWithApi";

const flush = () => act(async () => {});

function reduceMotion(on: boolean) {
  jest.spyOn(AccessibilityInfo, "isReduceMotionEnabled").mockResolvedValue(on);
}

describe("StateIndicator", () => {
  beforeEach(() => localeStore.getState().setLocale("en"));
  afterEach(() => jest.restoreAllMocks());

  it("differs by glyph and text for every state, not by colour alone", async () => {
    reduceMotion(true);
    const seen = new Set<string>();
    for (const state of ["working", "waiting", "idle", "error", "stopped"] as const) {
      const { view } = renderWithApi(<StateIndicator state={state} />);
      await view;
      await flush();
      const glyph = screen.getByTestId("state-glyph").props.children as string;
      const text = screen.getByTestId("state-text").props.children as string;
      seen.add(`${glyph}|${text}`);
      expect(new Set([...seen].map((s) => s.split("|")[0])).size).toBe(seen.size);
      await screen.unmount();
    }
    expect(seen.size).toBe(5);
  });

  it("pulses while working, and changes without animation under reduced motion", async () => {
    const loop = jest.spyOn(Animated, "loop");
    reduceMotion(false);
    const { view } = renderWithApi(<StateIndicator state="working" />);
    await view;
    await flush();
    expect(loop).toHaveBeenCalledTimes(1);
    await screen.unmount();
    loop.mockClear();
    reduceMotion(true);
    const second = renderWithApi(<StateIndicator state="working" />);
    await second.view;
    await flush();
    expect(loop).not.toHaveBeenCalled();
    expect(screen.getByTestId("state-text")).toHaveTextContent("Working");
  });

  it("follows a reduced-motion change while shown", async () => {
    reduceMotion(false);
    let listener: (on: boolean) => void = () => {};
    jest.spyOn(AccessibilityInfo, "addEventListener").mockImplementation((_e, cb) => {
      listener = cb as (on: boolean) => void;
      return { remove: jest.fn() } as never;
    });
    const loop = jest.spyOn(Animated, "loop");
    const { view } = renderWithApi(<StateIndicator state="working" />);
    await view;
    await flush();
    loop.mockClear();
    await act(async () => listener(true));
    await act(async () => listener(false));
    expect(loop).toHaveBeenCalledTimes(1);
  });
});

describe("LocationRow", () => {
  beforeEach(() => {
    localeStore.getState().setLocale("en");
    reduceMotion(true);
  });
  afterEach(() => jest.restoreAllMocks());
  const row = sidebarTree(buildFixture())[0]!.projects[0]!.locations[0]!;

  it("truncates a long name to one line, keeps the full text in its label, and keeps the indicator first", async () => {
    const long = { ...row, location: { ...row.location, name: "a-very-long-worktree-name-".repeat(6) } };
    const { view } = renderWithApi(<LocationRow row={long} stale={false} selected={false} onSelect={() => {}} />);
    await view;
    const name = screen.getByTestId("location-name");
    expect(name.props.numberOfLines).toBe(1);
    expect(screen.getByRole("button").props.accessibilityLabel).toContain(long.location.name);
    const sessions = screen.getAllByTestId("session-row");
    expect(sessions[0]!.children[0]).toBe(screen.getAllByTestId("state-indicator")[0]);
  });

  it("lists terminal sessions with their state like structured ones, the kind as text", async () => {
    const { view } = renderWithApi(<LocationRow row={row} stale={false} selected={false} onSelect={() => {}} />);
    await view;
    expect(screen.getAllByTestId("session-row")).toHaveLength(2);
    expect(screen.getByText("Terminal")).toBeTruthy();
    expect(screen.getByText("Structured")).toBeTruthy();
    expect(screen.getAllByTestId("state-text")).toHaveLength(2);
  });

  it("is keyboard reachable, shows a focus ring and reports press and selection", async () => {
    const onSelect = jest.fn();
    const { view } = renderWithApi(<LocationRow row={row} stale selected onSelect={onSelect} />);
    await view;
    const button = screen.getByRole("button");
    expect(button.props.focusable).toBe(true);
    expect(button.props.accessibilityState).toMatchObject({ selected: true });
    expect(button.props.accessibilityLabel).toContain("last known");
    await fireEvent(button, "focus");
    expect(JSON.stringify(screen.getByRole("button").props.style)).toContain("outlineWidth");
    await fireEvent(button, "blur");
    expect(JSON.stringify(screen.getByRole("button").props.style)).not.toContain("outlineWidth");
    await fireEvent.press(button);
    expect(onSelect).toHaveBeenCalledWith(row.key);
  });
});

describe("Sidebar", () => {
  beforeEach(() => {
    localeStore.getState().setLocale("en");
    reduceMotion(true);
  });
  afterEach(() => jest.restoreAllMocks());

  it("lists every daemon, project, worktree and session of the fixture under its parents", async () => {
    const { view } = renderWithApi(
      <SnapshotProvider>
        <Sidebar />
      </SnapshotProvider>,
    );
    await view;
    await flush();
    expect(screen.getAllByTestId("daemon-header")).toHaveLength(2);
    expect(screen.getAllByTestId("project-header")).toHaveLength(6);
    expect(screen.getAllByRole("button")).toHaveLength(30);
    expect(screen.getAllByTestId("session-row")).toHaveLength(60);
    expect(screen.getByText("workstation")).toBeTruthy();
  });

  it("selects a worktree when pressed", async () => {
    const { view } = renderWithApi(
      <SnapshotProvider>
        <Sidebar />
      </SnapshotProvider>,
    );
    await view;
    await flush();
    const second = screen.getAllByRole("button")[1]!;
    await fireEvent.press(second);
    expect(screen.getAllByRole("button")[1]!.props.accessibilityState).toMatchObject({ selected: true });
  });

  it("shows a loading message, then a message for an empty list", async () => {
    const mock = createMockApi();
    let resolve: (s: ReturnType<typeof buildFixture>) => void = () => {};
    mock.api.orchestrator.getSnapshot = () => new Promise((r) => (resolve = r));
    const { view } = renderWithApi(
      <SnapshotProvider>
        <Sidebar />
      </SnapshotProvider>,
      mock,
    );
    await view;
    expect(screen.getByText("Loading projects…")).toBeTruthy();
    await act(async () => resolve({ seq: 0, daemons: [], projects: [], locations: [], sessions: [] }));
    expect(screen.getByText("No daemons are connected yet.")).toBeTruthy();
  });

  it("says so for a daemon with no projects, and when the list cannot be loaded", async () => {
    const mock = createMockApi();
    const snap = buildFixture();
    mock.api.orchestrator.getSnapshot = async () => ({ ...snap, daemons: [snap.daemons[0]!], projects: [] });
    const { view } = renderWithApi(
      <SnapshotProvider>
        <Sidebar />
      </SnapshotProvider>,
      mock,
    );
    await view;
    await flush();
    expect(screen.getByText("No projects on this daemon.")).toBeTruthy();
    await screen.unmount();
    const failing = createMockApi();
    failing.api.orchestrator.getSnapshot = () => Promise.reject(new Error("down"));
    const second = renderWithApi(
      <SnapshotProvider>
        <Sidebar />
      </SnapshotProvider>,
      failing,
    );
    await second.view;
    await flush();
    expect(screen.getByRole("alert")).toHaveTextContent("The projects could not be loaded.");
  });
});
