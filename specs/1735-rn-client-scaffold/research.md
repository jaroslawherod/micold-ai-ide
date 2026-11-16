# Research: #1735 client scaffold

Each entry: decision, rationale, rejected alternatives.

**R1 Toolchain.** Expo managed + Expo Router, web only, pnpm workspace, `react-native-web`. Rejected: Vite + react-native-web (spec fixes Expo, Router file routes, later native reach); npm/yarn workspaces (pnpm gives a strict store and a fast CI cache); bare React Native (no benefit on web).

**R2 i18n library.** `@formatjs/intl` (`intl-messageformat`) with ICU messages and typed ids. Rejected: `i18next` (ICU needs a plugin; plural rules via its own format); `lingui` (macro/compile step adds Babel config to Expo); `expo-localization` alone (detects locale only; used just through the `platform/` wrapper for the browser language).

**R3 UI kit.** `react-native-paper` v5 `MD3Theme`, as the spec requires. Rejected: custom components from tokens (re-implements what Paper provides; spike R12 checks gaps).

**R4 Theme build.** `buildTheme(scheme)` maps each token colour role to the MD3 role of the same name and fails a test on a role without a match, so the file is imported, never copied. Fonts bundled through `@expo-google-fonts` packages (files in the export). Scheme from `matchMedia('(prefers-color-scheme: dark)')` with a listener; absent API means light. Rejected: copying tokens to JSON (drifts); fetching Google Fonts (third-party fetch, forbidden by US4 AC3).

**R5 State.** `zustand` stores (session, locale, list snapshot) fed by the API subscription. Rejected: Redux Toolkit (more ceremony for three stores); React context only (re-renders whole tree per state change, which risks SC-002).

**R6 Mock seam.** Interface in `@micold/protocol`, mock in `@micold/protocol/mock`, selected by env in `select.ts` (plan, design notes). Rejected: MSW/network mocking (the real transport is a socket, not HTTP; the interface is the contract); runtime query flag (would ship the mock in production).

**R7 Pins and CI.** `mise.toml` already pins `node = "24"` (docs axe check), reused; it gains only `pnpm`; CI installs through `mise` like other jobs, caches the pnpm store, runs `client-test`, `client-web`, Playwright (`--project chromium firefox webkit`), bundle check, path-gated by the existing `classify` job (new `client` output). Rejected: separate `setup-node` (two sources of version truth).

**R8 Auth in the client.** In-memory `AuthSession` (access token held in a module closure inside the store, not serialisable, not in persisted state). Return-to route kept in a router param, not storage. Rejected: `sessionStorage` for return-to or token (violates FR-006 and US1 AC4).

**R9 Locale persistence.** `localStorage` key `micold.locale` holds only a locale code, read in a wrapper that tolerates a throwing storage. Pseudo-locale selectable only when `EXPO_PUBLIC_API_MODE=mock` or dev. Rejected: cookie (sent to server, not needed).

**R10 Test ordering (Principle I).** For each unit the failing test is written first: theme, i18n, connection, mock/order, auth/guard, sidebar, overview, layout, bundle check. Tool: Jest + Testing Library; Playwright for browser. `speckit-tdd-setup` records the commands.

**R11 Coverage and bundle check.** `scripts/guardrails-scope.conf` `[web]` is empty today, so `diff-coverage.py --lang web` checks nothing: the plan adds the client and protocol source globs there, and the CI client job runs `scripts/diff-coverage.py --lang web --lcov apps/client/coverage/lcov.info` on Jest's lcov (the tasks unit verifies lcov paths resolve to repo paths). Bundle check is a Node script scanning `dist/` and `pnpm-lock.yaml` for a denylist (`xterm`, `@xterm/`, `node-pty`, `hterm`, `term.js`) and the mock sentinel.

**R12 Spike.** A dev-only route renders 5,000 transcript entries and a 2,000-line diff with Paper `Text`/`FlatList`, measured with `performance.now`, Chromium trace and `performance.memory`; written to `spike.md`. Production exclusion: the route file re-exports a stub unless the mock/dev flag is set, and the bundle check covers the sentinel. Candidates if it fails: FlashList, `react-window` through `react-native-web` view; the finding names #1736 as owner.

**R13 Overview arrangement.** Task 1 of M3 reads `crates/micold-client/src/ui/overview.rs` (card width, gap, column rule) and records the numbers below (*Measured*); the grid uses `flexWrap` with the measured card min-width. Rejected: CSS grid (not portable to RN style objects).

  *Measured (M3, T040)* from `crates/micold-client/src/ui/material/card_grid.rs` and `ui/overview.rs`: card min-width `MIN_WIDTH = 280`, gap `spacing::MD = 16`, at most `MAX_COLUMNS = 5` per row, column rule `clamp(floor((width + gap) / (min + gap)), 1, 5)` with the row's width shared equally (wider panes make wider cards, not more of them); the pane is padded `spacing::LG = 24` and its header sits `spacing::SM = 8` above the grid. The web client's `gridColumns()` uses the same rule and numbers; below 600 px it is one column regardless.

**R14 Logical layout.** Custom ESLint rule (or `no-restricted-syntax`) forbids `left|right|marginLeft|marginRight|paddingLeft|paddingRight|borderLeft*|borderRight*|textAlign: 'left'|'right'` in `apps/client/src`; use `start/end` equivalents.
