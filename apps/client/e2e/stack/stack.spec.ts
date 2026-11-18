// The web client against the real stack (#1732, T038, SC-001, SC-006, US1.3, US3.3, FR-018):
// `scripts/local-stack.sh` in process mode (the orchestrator, one daemon serving the seeded catalog,
// the web export), no AI CLI. Run by `mise run client-e2e-stack`, which builds the daemon and the
// export once and passes them in MICOLD_LOCAL_DAEMON_BIN and MICOLD_LOCAL_WEB_DIR. The stack is
// started by this file, once per browser (workers: 1), so the restart test owns its processes.
//
// Live change: the client API is read-only and nothing outside the daemon's own protocol creates,
// renames or ends a session, so the change pushed here is the whole stack going away and coming
// back: the open page shows the outage and then the seeded sessions again, with no reload. The
// per-session create/rename/end pushes are covered by the daemon's tests/orchestrator/ws_*.rs.
import { expect, test, type Page } from "@playwright/test";
import { STACK_WEB_PORT, startStack, type Stack } from "./stackProcess";

let stack: Stack;

test.beforeAll(async () => {
  stack = await startStack();
});
test.afterAll(async () => {
  await stack?.stop({ removeDir: true });
});

async function signIn(page: Page): Promise<void> {
  await page.goto(`http://127.0.0.1:${STACK_WEB_PORT}/`);
  await page.getByRole("button", { name: /Local orchestrator/ }).click();
  await expect(page.getByTestId("sidebar")).toBeVisible();
}

const seeded = (page: Page) => page.getByTestId("session-row").filter({ hasText: "Fix the login bug" });

test("the seeded sessions are listed within 60 s, with no hand-edited allow-list", async ({ page }) => {
  const start = Date.now();
  await signIn(page);
  await expect(seeded(page)).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("session-row").filter({ hasText: "Add dark mode" })).toBeVisible();
  expect(Date.now() - start).toBeLessThan(60_000);
  await expect(page.getByTestId("connection-banner")).toHaveCount(0);
});

test("opening a session shows its content", async ({ page }) => {
  await signIn(page);
  await seeded(page).click({ timeout: 60_000 });
  await expect(page.getByTestId("session-content")).toBeVisible({ timeout: 10_000 });
  await expect(page.getByTestId("session-unreachable")).toHaveCount(0);
});

test("a stopped and restarted orchestrator reaches the open page with no reload", async ({ page }) => {
  await signIn(page);
  await expect(seeded(page)).toBeVisible({ timeout: 60_000 });
  // A marker on the window: a reload would drop it.
  await page.evaluate(() => ((globalThis as { __noReload?: boolean }).__noReload = true));

  await stack.stop({ removeDir: false });
  await expect(page.getByTestId("connection-banner")).toBeVisible({ timeout: 15_000 });

  stack = await startStack(stack.dir);
  const retry = page.getByRole("button", { name: "Retry" });
  if (await retry.isVisible()) await retry.click();
  await expect(page.getByTestId("connection-banner")).toHaveCount(0, { timeout: 60_000 });
  // Within 2 s of the connection being back, the list is there (SC-006).
  await expect(seeded(page)).toBeVisible({ timeout: 2_000 });
  expect(await page.evaluate(() => (globalThis as { __noReload?: boolean }).__noReload)).toBe(true);
});
