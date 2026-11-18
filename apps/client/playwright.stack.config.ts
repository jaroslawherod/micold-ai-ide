// Playwright for the web client against the real stack (#1732, T038): e2e/stack/ only, in Chromium,
// Firefox and WebKit, one worker, because each browser's run starts and restarts its own
// `scripts/local-stack.sh` on the fixed ports the export was built for. `mise run client-e2e-stack`.
import { defineConfig, devices } from "@playwright/test";

const channel = process.env.PW_CHROMIUM_CHANNEL;

export default defineConfig({
  testDir: "e2e/stack",
  workers: 1,
  fullyParallel: false,
  timeout: 180_000,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never", outputFolder: "playwright-report-stack" }]] : "list",
  use: { trace: "retain-on-failure" },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], ...(channel ? { channel } : {}) } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
});
