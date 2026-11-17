// Every user-visible string on sign-in, the sidebar and the Overview comes from the catalog (SC-004):
// under the pseudo-locale no text is left untransformed, accessibility labels included, and Polish
// plurals render for counts (US5 scenario 4).
import React from "react";
import { AccessibilityInfo } from "react-native";
import { act, screen } from "@testing-library/react-native";
import { buildFixture } from "@micold/protocol/mock";
import Index from "../../app/(app)/index";
import { localeStore } from "../../src/i18n/localeStore";
import { createTranslator } from "../../src/i18n/translator";
import { renderWithApi } from "../support/renderWithApi";

jest.mock("expo-router", () => ({ useRouter: () => ({ replace: jest.fn() }), useLocalSearchParams: () => ({}) }));
import SignIn from "../../app/sign-in";

const fixture = buildFixture();
/** Data from the orchestrator, shown as is, and the state glyphs. */
const DATA = new Set<string>([
  ...fixture.daemons.map((d) => d.name),
  ...fixture.projects.map((p) => p.name),
  ...fixture.locations.map((l) => l.name),
  ...fixture.sessions.map((s) => s.name),
  ..."◐◆○✕■●▲",
  "GitHub",
  "Google",
  "Polski",
  "English",
]);

function visibleStrings(): string[] {
  const out: string[] = [];
  const walk = (node: unknown) => {
    if (typeof node === "string") out.push(node);
    else if (Array.isArray(node)) node.forEach(walk);
    else if (node && typeof node === "object") {
      const n = node as { props?: Record<string, unknown>; children?: unknown };
      if (typeof n.props?.accessibilityLabel === "string") out.push(n.props.accessibilityLabel);
      walk(n.children);
    }
  };
  walk(screen.toJSON());
  return out.map((s) => s.trim()).filter((s) => s.length > 0);
}

const untransformed = () => visibleStrings().filter((s) => !DATA.has(s) && !/\[.*\]/s.test(s));

describe("screens under the pseudo-locale", () => {
  beforeEach(() => jest.spyOn(AccessibilityInfo, "isReduceMotionEnabled").mockResolvedValue(true));
  afterEach(() => {
    jest.restoreAllMocks();
    localeStore.getState().setLocale("en");
  });

  it("leaves no string of the sidebar and the Overview untransformed", async () => {
    localeStore.setState({ locale: "pseudo" });
    await renderWithApi(<Index />).view;
    await act(async () => {});
    expect(visibleStrings().length).toBeGreaterThan(100);
    expect(untransformed()).toEqual([]);
  });

  it("leaves no string of the sign-in page untransformed", async () => {
    localeStore.setState({ locale: "pseudo" });
    await renderWithApi(<SignIn />).view;
    await act(async () => {});
    expect(untransformed()).toEqual([]);
  });

  it("renders Polish plurals for counts, on the cards and in the catalog", async () => {
    const t = createTranslator("pl").t;
    const card = (n: number) => t("overview.card", { project: "p", name: "n", sessions: n });
    expect([card(1), card(2), card(5), card(22)]).toEqual(["p: n, 1 sesja", "p: n, 2 sesje", "p: n, 5 sesji", "p: n, 22 sesje"]);
    localeStore.getState().setLocale("pl");
    await renderWithApi(<Index />).view;
    await act(async () => {});
    expect(screen.getAllByTestId("overview-card")[0]!.props.accessibilityLabel).toBe("project-11: main, 2 sesje");
  });
});
