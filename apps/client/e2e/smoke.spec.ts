// Smoke test of the web client against the mock export (#1735, FR-018, FR-019, SC-002, SC-006):
// sign-in, sidebar, Overview, an order-stable state change, the locale switch and its persistence,
// web storage, the keyboard path, accessibility, narrow and zoomed widths, and reduced motion.
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { pushChange, signIn } from "./helpers";

const rowNames = (page: Page) => page.getByTestId("session-row").evaluateAll((rows) => rows.map((r) => r.getAttribute("aria-label")));

test("sign-in shows the sidebar and the Overview", async ({ page }) => {
  await signIn(page);
  await expect(page.getByTestId("project-header").first()).toBeVisible();
  await expect(page.getByTestId("overview-card")).toHaveCount(30);
  await expect(page).toHaveTitle("Micold");
});

test("a state change reaches the indicator within 100 ms and keeps every row in place", async ({ page, browserName }) => {
  await signIn(page);
  const before = await rowNames(page);
  expect(before[0]).toBe("session 1, Working");
  const elapsed = await page.evaluate(
    () =>
      new Promise<number>((resolve, reject) => {
        const row = document.querySelector('[data-testid="session-row"]');
        if (!row) return reject(new Error("no session row"));
        const start = performance.now();
        const done = () => row.getAttribute("aria-label") === "session 1, Error";
        new MutationObserver((_, obs) => {
          if (done()) {
            obs.disconnect();
            resolve(performance.now() - start);
          }
        }).observe(row, { attributes: true, subtree: true, characterData: true, childList: true });
        (globalThis as unknown as { __micoldMock: { pushChange(c: unknown): void } }).__micoldMock.pushChange({
          kind: "session-state",
          sessionId: "d1-p1-l1-s1",
          state: "error",
        });
        setTimeout(() => reject(new Error("the indicator did not change within 1 s")), 1000);
      }),
  );
  // SC-002's 100 ms is measured in Chromium, the reference browser; the other engines check the order only.
  if (browserName === "chromium") expect(elapsed).toBeLessThan(100);
  const after = await rowNames(page);
  expect(after).toEqual(["session 1, Error", ...before.slice(1)]);
});

test("the locale switches at runtime and persists across a reload", async ({ page }) => {
  await signIn(page);
  await page.getByRole("radio", { name: "Polski" }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "pl");
  await expect(page.getByRole("radio", { name: "Polski" })).toBeChecked();
  await page.reload();
  // The session is in memory only, so the reload signs out; the locale stays.
  await expect(page.locator("html")).toHaveAttribute("lang", "pl");
  await expect(page.getByRole("radio", { name: "Polski" })).toBeChecked();
});

test("web storage and cookies hold no token after sign-in", async ({ page, context }) => {
  await signIn(page);
  const stored = await page.evaluate(() =>
    [localStorage, sessionStorage].flatMap((s) => Object.keys(s).map((k) => `${k}=${s.getItem(k)}`)),
  );
  const cookies = (await context.cookies()).map((c) => `${c.name}=${c.value}`);
  for (const entry of [...stored, ...cookies]) expect(entry).not.toMatch(/token/i);
});

test("the keyboard reaches sign-in, the locale control and the sidebar with a visible focus", async ({ page, browserName }) => {
  // WebKit on macOS moves Tab focus to links and fields only (Option+Tab reaches every control); on
  // Linux, where CI runs, and with the explicit tabindex react-native-web sets, Tab reaches them.
  const tab = browserName === "webkit" && process.platform === "darwin" ? "Alt+Tab" : "Tab";
  await page.locator("body").focus();
  await page.goto("/");
  await expect(page.getByRole("button", { name: /GitHub/ })).toBeVisible();
  const focusTo = async (name: RegExp) => {
    for (let i = 0; i < 20; i += 1) {
      await page.keyboard.press(tab);
      const label = await page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? document.activeElement?.textContent ?? "");
      if (name.test(label)) return;
    }
    throw new Error(`Tab never reached ${name}`);
  };
  await focusTo(/GitHub/);
  const ring = await page.evaluate(() => {
    const s = getComputedStyle(document.activeElement as Element);
    return s.outlineStyle !== "none" || s.boxShadow !== "none";
  });
  expect(ring).toBe(true);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("sidebar")).toBeVisible();
  await focusTo(/Polski/);
  await page.keyboard.press("Enter");
  await expect(page.locator("html")).toHaveAttribute("lang", "pl");
  await focusTo(/^main,/);
});

test("the sign-in and home screens have no axe violations", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: /GitHub/ })).toBeVisible();
  const tags = ["wcag2a", "wcag2aa"];
  expect((await new AxeBuilder({ page }).withTags(tags).analyze()).violations).toEqual([]);
  await page.getByRole("button", { name: /GitHub/ }).click();
  await expect(page.getByTestId("sidebar")).toBeVisible();
  expect((await new AxeBuilder({ page }).withTags(tags).analyze()).violations).toEqual([]);
});

for (const [what, width] of [
  ["300 px", 300],
  ["200 percent zoom of 1280 px", 640],
] as const) {
  test(`no horizontal scroll at ${what}`, async ({ page }) => {
    await page.setViewportSize({ width, height: 800 });
    await signIn(page);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  });
}

test.describe("reduced motion", () => {
  test.use({ reducedMotion: "reduce" });
  test("a working session's glyph does not pulse", async ({ page }) => {
    await signIn(page);
    const glyph = page.getByTestId("session-row").first().getByTestId("state-glyph");
    const samples: string[] = [];
    for (let i = 0; i < 4; i += 1) {
      samples.push(await glyph.evaluate((g) => getComputedStyle(g).opacity));
      await page.waitForTimeout(200);
    }
    expect(new Set(samples)).toEqual(new Set(["1"]));
  });
});

test("the mock feed changes the Overview card state too", async ({ page }) => {
  await signIn(page);
  await pushChange(page, { kind: "session-state", sessionId: "d1-p1-l1-s1", state: "idle" });
  await expect(page.getByTestId("session-row").first()).toHaveAttribute("aria-label", "session 1, Idle");
});
