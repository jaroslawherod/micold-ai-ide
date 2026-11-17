// Playwright for the web client (#1735, FR-018, T055): the mock web export (`pnpm web:export:mock`)
// served statically, in Chromium, Firefox and WebKit. The screenshot spec runs in Chromium only, the
// reference browser of the visual pass (FR-021). Locally, PW_CHROMIUM_CHANNEL=chrome uses an
// installed Chrome when the Playwright browsers cannot be downloaded.
import { defineConfig, devices } from "@playwright/test";

const PORT = 8091;
const channel = process.env.PW_CHROMIUM_CHANNEL;

export default defineConfig({
  testDir: "e2e",
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: { baseURL: `http://127.0.0.1:${PORT}`, trace: "retain-on-failure" },
  webServer: {
    command: `node e2e/serve.mjs dist-mock ${PORT}`,
    url: `http://127.0.0.1:${PORT}/`,
    reuseExistingServer: !process.env.CI,
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], ...(channel ? { channel } : {}) } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] }, testIgnore: /screenshots\.spec/ },
    { name: "webkit", use: { ...devices["Desktop Safari"] }, testIgnore: /screenshots\.spec/ },
  ],
});
