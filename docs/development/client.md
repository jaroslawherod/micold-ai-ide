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
| `mise run client-test` | install, lint, typecheck, Jest with coverage |
| `mise run client-web` | the production web export into `apps/client/dist` |
| `mise run client-e2e` | the mock export into `apps/client/dist-mock`, then Playwright in Chromium, Firefox and WebKit |

Screenshots for the visual pass (1280, 800 and 390 px, light and dark) land in
`apps/client/e2e/screenshots/`; CI uploads them as the `client-e2e` artifact. Where the Playwright
browsers cannot be downloaded, `PW_CHROMIUM_CHANNEL=chrome pnpm --filter @micold/client exec
playwright test --project=chromium` runs against an installed Chrome.

CI's `web client` job runs all of this, then the bundle check on the production export, and checks
that a mock export fails it.

## The mock switch

`EXPO_PUBLIC_API_MODE` is inlined at build time. `mock` (`pnpm start`, `web:export:mock`) loads
`@micold/protocol/mock`, an in-memory orchestrator and auth service with a fixed fixture, and exposes
its control as `globalThis.__micoldMock` for Playwright (push a change, refuse a sign-in, drop the
connection). Any other value leaves the mock out of the bundle; the bundle check proves it by the
`__MICOLD_MOCK__` sentinel. The pseudo-locale is offered only in mock and dev builds.

## The protocol package stand-in

`@micold/protocol` is hand-written for now: the types follow the orchestrator API contract
(`specs/1735-rn-client-scaffold/contracts/`), and the app reaches the API only through the `Api`
interface from `useApi()`.

## How #1732 swaps in

When the real orchestrator client (#1732) and the generated protocol types (#1733) land:

1. Replace the hand-written types in `packages/protocol/src/types.ts` with the generated ones.
2. Implement `Api` over the real transport in `@micold/protocol` and return it from the non-mock
   branch of `apps/client/src/api/select.ts`; today that branch throws and `ApiProvider` shows why.
3. Keep the mock for dev, Jest and Playwright; the bundle check keeps it out of the production export.
