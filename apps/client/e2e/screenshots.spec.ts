// Screenshots for the visual pass (#1735, FR-021, SC-003, T057): the home route at 1280, 800 and
// 390 px in light and dark, in Chromium, to e2e/screenshots-out/ (gitignored;
// the committed e2e/screenshots/ are the recorded set of the visual pass). Also asserts the Overview's column
// count: one column below the sidebar under 600 px, the sidebar beside it from 600 px.
import { expect, test } from "@playwright/test";
import { signIn } from "./helpers";

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 800, 390]) {
    test(`home at ${width} px, ${scheme}`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      await page.setViewportSize({ width, height: 900 });
      await signIn(page);
      const columns = await page.getByTestId("overview-card").evaluateAll((cards) => new Set(cards.map((c) => Math.round(c.getBoundingClientRect().left))).size);
      const sidebar = (await page.getByTestId("sidebar-pane").boundingBox())!;
      const overview = (await page.getByTestId("overview-pane").boundingBox())!;
      if (width < 600) {
        expect(columns).toBe(1);
        expect(overview.y).toBeGreaterThanOrEqual(sidebar.y + sidebar.height - 1);
      } else {
        expect(overview.x).toBeGreaterThanOrEqual(sidebar.x + sidebar.width - 1);
      }
      await page.screenshot({ path: `e2e/screenshots-out/home-${width}-${scheme}.png` });
      if (width < 600) {
        // The Overview sits under the sidebar in the one scroll view: show where it starts.
        await page.getByTestId("overview-pane").evaluate((el) => el.scrollIntoView());
        await page.screenshot({ path: `e2e/screenshots-out/home-${width}-${scheme}-overview.png` });
      }
    });
  }
}
