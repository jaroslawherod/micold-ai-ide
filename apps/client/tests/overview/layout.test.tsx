// The Overview grid and the stacking breakpoint (FR-012, SC-003, US3 scenarios 1-3; Edge Cases, Viewport).
import React from "react";
import { AccessibilityInfo, StyleSheet } from "react-native";
import { act, fireEvent, screen } from "@testing-library/react-native";
import Index from "../../app/(app)/index";
import { localeStore } from "../../src/i18n/localeStore";
import { CARD_GAP, CARD_MIN_WIDTH, gridColumns } from "../../src/theme/layout";
import { renderWithApi } from "../support/renderWithApi";

jest.mock("react-native/Libraries/Utilities/useWindowDimensions", () => ({ __esModule: true, default: jest.fn() }));
const dims = require("react-native/Libraries/Utilities/useWindowDimensions").default as jest.Mock;

async function at(width: number) {
  dims.mockReturnValue({ width, height: 900, scale: 1, fontScale: 1 });
  const { mock, view, rerender } = renderWithApi(<Index />);
  await view;
  await act(async () => {});
  return { mock, rerender };
}
const flat = (id: string) => StyleSheet.flatten(screen.getByTestId(id).props.style);
const cardWidths = () => screen.getAllByTestId("overview-card").map((c) => StyleSheet.flatten(c.props.style).width as number);

describe("gridColumns", () => {
  it("follows the iced rule: floor((w + gap) / (min + gap)), 1 to 5, one column when narrow", () => {
    expect([CARD_MIN_WIDTH, CARD_GAP]).toEqual([280, 16]);
    expect(gridColumns(279, true)).toBe(1);
    expect(gridColumns(576, true)).toBe(2);
    expect(gridColumns(575, true)).toBe(1);
    expect(gridColumns(5000, true)).toBe(5);
    expect(gridColumns(5000, false)).toBe(1);
  });
});

describe("home layout", () => {
  beforeEach(() => {
    localeStore.getState().setLocale("en");
    jest.spyOn(AccessibilityInfo, "isReduceMotionEnabled").mockResolvedValue(true);
  });
  afterEach(() => jest.restoreAllMocks());

  it("puts the sidebar beside a multi-column grid at 1280 px", async () => {
    await at(1280);
    expect(flat("home-body").flexDirection).toBe("row");
    expect(screen.getAllByTestId("overview-card")).toHaveLength(30);
    const widths = cardWidths();
    expect(new Set(widths).size).toBe(1);
    expect(widths[0]).toBeGreaterThanOrEqual(CARD_MIN_WIDTH);
    expect(widths[0]! * 3).toBeLessThan(1280 - 300);
  });

  it("uses the wide arrangement at exactly 600 px and stacks at 599 px, sidebar above the grid", async () => {
    await at(600);
    expect(flat("home-body").flexDirection).toBe("row");
    await screen.unmount();
    await at(599);
    expect(flat("home-body").flexDirection).toBe("column");
    const children = screen.getByTestId("home-body").children as { props: { testID?: string } }[];
    expect(children.map((c) => c.props.testID)).toEqual(["sidebar-pane", "overview-pane"]);
  });

  it.each([599, 390, 319, 160])("keeps every card inside a %i px window with no horizontal scroll", async (width) => {
    await at(width);
    for (const w of cardWidths()) expect(w).toBeLessThanOrEqual(width);
    expect(screen.queryByTestId("home-scroll")?.props.horizontal).toBeFalsy();
  });

  it("re-flows on resize and on the grid's measured width", async () => {
    const { rerender } = await at(1280);
    await fireEvent(screen.getByTestId("overview-grid"), "layout", { nativeEvent: { layout: { width: 592, height: 10, x: 0, y: 0 } } });
    expect(cardWidths()[0]).toBe((592 - CARD_GAP) / 2);
    dims.mockReturnValue({ width: 500, height: 900, scale: 1, fontScale: 1 });
    await rerender(<Index />);
    expect(flat("home-body").flexDirection).toBe("column");
  });

  it("floors a fractional card width so the last card of a row never wraps", async () => {
    await at(1280);
    await fireEvent(screen.getByTestId("overview-grid"), "layout", { nativeEvent: { layout: { width: 1000, height: 10, x: 0, y: 0 } } });
    const [w] = cardWidths();
    expect(Number.isInteger(w)).toBe(true);
    expect(w! * 3 + CARD_GAP * 2).toBeLessThanOrEqual(1000);
    expect(w).toBe(Math.floor((1000 - CARD_GAP * 2) / 3));
  });

  it("keeps a card in place when its state changes", async () => {
    const { mock } = await at(1280);
    const order = screen.getAllByTestId("overview-card").map((c) => c.props.accessibilityLabel);
    const snap = await mock.api.orchestrator.getSnapshot();
    await act(async () => mock.control.pushChange({ kind: "session-state", sessionId: snap.sessions[2]!.id, state: "error" }));
    expect(screen.getAllByTestId("overview-card").map((c) => c.props.accessibilityLabel)).toEqual(order);
  });

  it("shows an empty Overview with a message", async () => {
    dims.mockReturnValue({ width: 1280, height: 900, scale: 1, fontScale: 1 });
    const { createMockApi } = require("@micold/protocol/mock");
    const mock = createMockApi();
    mock.api.orchestrator.getSnapshot = async () => ({ seq: 0, daemons: [], projects: [], locations: [], sessions: [] });
    const { view } = renderWithApi(<Index />, mock);
    await view;
    await act(async () => {});
    expect(screen.getByText("No worktrees to show.")).toBeTruthy();
  });
});
