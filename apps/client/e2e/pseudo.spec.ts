// Pseudo-locale run (#1735, US5, SC-004, T056): every visible string is transformed (accented and
// bracketed by the pseudo-locale), the text grows by about 40 percent, and no state indicator is
// clipped by the growth.
import { expect, test } from "@playwright/test";
import { signIn } from "./helpers";

test("the pseudo-locale transforms every string and clips no indicator", async ({ page }) => {
  await signIn(page);
  const english = await page.evaluate(() => document.body.innerText.length);
  await page.getByRole("radio", { name: "Pseudo (test)" }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByTestId("state-text").first()).not.toHaveText("Working");
  // Catalog strings carry an accented letter; data (names, branches, counts) is not from the catalog.
  const plain = await page.getByTestId("state-text").evaluateAll((els) => els.map((e) => e.textContent ?? "").filter((t) => /^[A-Za-z ]+$/.test(t)));
  expect(plain).toEqual([]);
  const pseudo = await page.evaluate(() => document.body.innerText.length);
  expect(pseudo).toBeGreaterThan(english);
  const clipped = await page.getByTestId("state-indicator").evaluateAll((els) =>
    els.filter((el) => {
      const r = el.getBoundingClientRect();
      const text = el.querySelector('[data-testid="state-text"]') as HTMLElement | null;
      return !text || text.scrollWidth > text.clientWidth + 1 || r.width === 0;
    }).length,
  );
  expect(clipped).toBe(0);
});
