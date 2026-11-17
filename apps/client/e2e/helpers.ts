// Shared steps of the Playwright specs (#1735, M4): sign in through the mock and drive its feed.
import { expect, type Page } from "@playwright/test";

/** Opens the app and signs in with the mock's first provider; lands on the home route. */
export async function signIn(page: Page): Promise<void> {
  await page.goto("/");
  await page.getByRole("button", { name: /GitHub/ }).click();
  await expect(page.getByTestId("sidebar")).toBeVisible();
}

/** Pushes one change through the mock feed (`globalThis.__micoldMock`, mock builds only). */
export async function pushChange(page: Page, change: Record<string, unknown>): Promise<void> {
  await page.evaluate((c) => (globalThis as unknown as { __micoldMock: { pushChange(c: unknown): void } }).__micoldMock.pushChange(c), change);
}
