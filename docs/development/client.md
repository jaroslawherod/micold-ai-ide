# The web client

The browser client (#1735) is a React Native app built with Expo and exported for the web. It
lives beside the Rust workspace as a pnpm workspace (`package.json`, `pnpm-workspace.yaml`).

## Layout

| Path | What it holds |
|---|---|
| `apps/client/app/` | expo-router routes: `sign-in`, the signed-in group `(app)/` (sidebar and Overview), the dev `spike` |
| `apps/client/src/api/` | `ApiProvider` and `select.ts`, the one place that picks the mock or the real client |
| `apps/client/src/auth/` | the in-memory session store and the route guard; no token reaches web storage |
| `apps/client/src/features/` | the sidebar (snapshot store, change feed) and the Overview |
| `apps/client/src/i18n/` | the catalogs (`en.json`, `pl.json`), the locale store and the computed pseudo-locale |
| `apps/client/src/theme/` | the Material 3 theme built from the generated tokens ([client theme](client-theme.md)) |
| `apps/client/tests/` | Jest (component and unit tests) |
| `apps/client/e2e/` | Playwright: smoke, pseudo-locale and screenshot specs, and the static server they use |
| `packages/protocol/` | `@micold/protocol`: the API types and interface, and the mock under `@micold/protocol/mock` |
| `scripts/client/bundle-check.mjs` | fails on the mock or a terminal emulator in the export or `pnpm-lock.yaml` |

## Tasks

| Task | Does |
|---|---|
| `mise run client-start` (alias `mise run web`) | install, then the Expo dev server for the web against a real orchestrator (`EXPO_PUBLIC_ORCHESTRATOR_URL`, default `ws://127.0.0.1:8730/api/ws`); open the URL it prints |
| `mise run client-web-mock` | the same dev server against the in-memory mock, no orchestrator |
| `mise run local-stack` | the orchestrator, a seeded daemon and the web export together, [user guide](../user-guide/web-client.md); `Ctrl+C` stops all |
| `mise run client-test` | install, lint, typecheck, Jest with coverage |
| `mise run client-web` | the production web export into `apps/client/dist` |
| `mise run client-e2e` | the mock export into `apps/client/dist-mock`, then Playwright in Chromium, Firefox and WebKit |

`mise run client-e2e` installs the browsers without their system libraries. On a fresh Linux
machine, run `pnpm --filter @micold/client exec playwright install --with-deps chromium firefox
webkit` once (it needs `sudo` for the apt packages); CI's `web client e2e` job does this every run.

Screenshots for the visual pass (1280, 800 and 390 px, light and dark) land in
`apps/client/e2e/screenshots-out/` (gitignored); CI uploads them as the `client-e2e` artifact.
`apps/client/e2e/screenshots/` holds the recorded set of the last visual pass; copy a new run over
it only when you record a new pass. Where the Playwright
browsers cannot be downloaded, `PW_CHROMIUM_CHANNEL=chrome pnpm --filter @micold/client exec
playwright test --project=chromium` runs against an installed Chrome.

CI's `web client` job runs the tests, the bundle check on the production export, and checks that a
mock export fails it. The Playwright specs run in their own `web client e2e` job, with the browsers
cached under `~/.cache/ms-playwright` keyed on `pnpm-lock.yaml`, so the download stays out of the
`web client` job's ten minutes (SC-005).

Two checks differ by browser: the 100 ms state-change bound (SC-002) is asserted in Chromium only,
the reference browser, while Firefox and WebKit check that every row keeps its place. The keyboard
test presses Option+Tab in WebKit on macOS, where plain Tab skips buttons; on Linux (CI) it presses
Tab in all three browsers.

## The mock switch

`EXPO_PUBLIC_API_MODE` is inlined at build time. `mock` (`pnpm start:mock`, `web:export:mock`) loads
`@micold/protocol/mock`, an in-memory orchestrator and auth service with a fixed fixture, and exposes
its control as `globalThis.__micoldMock` for Playwright (push a change, refuse a sign-in, drop the
connection). Any other value (the default) selects the real client over `/api/ws` and leaves the mock out of the bundle; the bundle check proves it by the
`__MICOLD_MOCK__` sentinel. The pseudo-locale is offered only in mock and dev builds.

## The protocol package stand-in

`@micold/protocol` is hand-written for now: the types follow the orchestrator API contract
(`specs/1735-rn-client-scaffold/contracts/`), and the app reaches the API only through the `Api`
interface from `useApi()`.

## The real client (#1732)

`@micold/protocol/real` implements `OrchestratorApi` over `/api/ws`: `BrowserTransport` (the one
browser-specific seam: close codes, `visibilitychange`), `RealOrchestratorApi` (subscribe, resume from
`{epoch, seq}`, ping every 20 s, backoff of 500 ms to 15 s with full jitter, `offline` after 8
attempts) and a local auth stand-in, since the API has no sign-in until #1733. `select.ts` returns it
in every build but `EXPO_PUBLIC_API_MODE=mock`; the types are generated from the Rust ones
(`mise run api-types`). The mock stays for Jest and Playwright; the bundle check keeps it out of the
production export.
