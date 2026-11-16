# Implementation Plan: React Native (Expo) web client scaffold

**Branch**: `claude/project-thread-hzt36f` | **Date**: 2026-11-16 | **Spec**: [spec.md](spec.md)

**Input**: `specs/1735-rn-client-scaffold/spec.md` (issue #1735, part of #1722, M4a)

## Summary

Create the TypeScript client around the existing `apps/client/src/theme/tokens.generated.ts`: an Expo
(managed, Expo Router, web only) app with a Paper v5 Material 3 theme built from the tokens, a
mocked-but-swappable orchestrator boundary, sign-in, a stable-order sidebar, the Overview grid, runtime
i18n (English, Polish, test pseudo-locale) and a CI job. The one seam is the `OrchestratorApi`
interface in `packages/protocol/`; a mock (dev, test, CI only) and later the real WebSocket client
(#1732) both satisfy it, and components import only the interface through a React context. A spike
(transcript and diff on Paper/web) comes first.

**Delivery**: the work ships in **3-4 milestones**, each its own PR; the tasks unit cuts them. The
candidate cut, from the dependency order, is: M1 workspace, tooling, CI, theme, i18n core and the spike
(FR-001..003, 014..016, 017..018 skeleton, 020); M2 protocol package, mock, transport and reconnect
logic, auth and routing (FR-004..009); M3 sidebar and Overview with stable order, offline and stale
rows (FR-010..013); M4 (may merge into M3 if small) Playwright three-browser smoke, screenshots and
visual pass, bundle check hardening, docs (FR-018, 019, 021, SC-003, SC-006, SC-007). The spike
precedes any sidebar work (FR-020).

## Technical Context

**Language/Version**: TypeScript 5.x `strict`; Node 24 (already pinned in `mise.toml` for the docs axe check; reused, not re-pinned) and pnpm 10 (the one new `[tools]` entry).

**Primary Dependencies** (all new, none exist in the repo yet): `expo` (SDK current stable), `expo-router`, `react-native-web`, `react-dom`, `react-native-paper` v5, `react-native-safe-area-context`, `@expo-google-fonts/roboto` and `@expo-google-fonts/fira-mono` (bundled font files, no CDN), `@formatjs/intl` + `intl-messageformat` for ICU messages (research R2), `zustand` for the in-memory stores (R5). Dev: `jest-expo`, `@testing-library/react-native`, `eslint` with `eslint-plugin-i18next` (`no-literal-string`) and `eslint-plugin-react-native-a11y`, `@playwright/test`, `typescript`.

**Storage**: none persisted except the locale preference (`localStorage` key `micold.locale`, a preference, not a token). Access token in memory only (FR-006). Constitution IV: nothing here needs a remote service; the mock runs offline.

**Testing**: Jest (`jest-expo`, jsdom/RN testing library) for unit and component tests; a fake-transport harness for reconnect; Playwright in Chromium, Firefox and WebKit against the static export; a Node bundle-check script. No Rust change, so no core unit tests; the existing `scripts/tests/tokens-ts-drift.test.sh` stays the first failing check for drift (US6 AC5).

**Target Platform**: static web export (`expo export --platform web`), current Chromium, Firefox, Safari. Native out of scope.

**Project Type**: web client (TypeScript) in a pnpm workspace, beside the Rust workspace.

**Performance Goals**: state change visible within 100 ms of feed delivery (SC-002); the spike records 5,000-entry transcript and 2,000-line diff numbers (SC-008).

**Constraints**: access token memory only; no terminal emulator dependency (FR-017); mock absent from production export (SC-006); logical (start/end) layout only (FR-016); CI client job under 10 minutes (SC-005).

**Scale/Scope**: fixture of 2 daemons, 6 projects, 30 worktrees, 60 sessions (SC-001); roughly 6 screens/routes (sign-in, overview, locale control, spike (dev only), connection banner, empty/error states).

## Constitution Check

| Principle | Verdict | Note |
|---|---|---|
| I. Test-First | PASS | Each unit below lists its tests first in [research.md](research.md) R10 and the tasks unit orders test tasks before code (tdd-plan). Config-only files (tsconfig, eslint) are covered by CI checks that fail without them. |
| II. Multi-Session | PASS | Rows keyed by daemon, project, location, session identity; per-session state in one keyed map; a change to one key never re-keys another (data-model.md). |
| III. Worktree Integration | PASS | The Client only lists locations (worktrees and the project's Default location); it creates and removes none (out of scope). |
| IV. Local-First (NON-NEGOTIABLE) | PASS | Nothing is added to local mode. The orchestrator and its sign-in are the opt-in network mode; the mock needs no service and is dev/test/CI only. |
| V. TypeScript + RN, Rust core | PASS with tracked deviation | The interface types are hand-written, not generated; justified in Complexity Tracking, replaced by #1732's generated types. |
| VI. Linux daemons, web reach | PASS | Playwright smoke in Chromium, Firefox, WebKit; `Intl`, `matchMedia` and font-loading access sit behind small wrappers (`src/platform/`) that degrade to English, light, system fonts. |
| VII. Documentation | PASS | `docs/development/client.md` (developer loop, mock switch, adding a locale) and a user-guide note on language and theme are in the same change; docs are linked from `docs/SUMMARY.md` and verified by the existing docs job. |
| VIII. Reusable UI Foundation | PASS | State indicator, location row, overview card, connection banner and locale picker are shared components in `apps/client/src/components/` with typed props objects, theme-driven, reused by sidebar and Overview. |

Re-check after design: unchanged.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Hand-written protocol types (Principle V asks for generated) | #1732/#1733 have no spec or code; the Client needs a typed boundary now | Waiting blocks M4a. Mitigation: types live only in `packages/protocol/src/types.ts`, marked `STAND-IN`, listed in ledger follow-ups, and the interface is the seam so replacement touches one package. |
| Second package (`packages/protocol/`) beside `apps/client/` | The spec fixes it; the real client and generated types will land there, shared by the Client and tests | A folder inside the app would make the later swap a move across package boundaries. |

## Requirement map

| FR / SC | Where |
|---|---|
| FR-001, 002 | workspace layout below; research R1, R7; `mise.toml` tasks `client-test`, `client-web` |
| FR-003 | `src/theme/` (`buildTheme.ts` imports `tokens.generated.ts`); R3, R4 |
| FR-004, 005 | `packages/protocol/src/api.ts`, `src/api/ApiProvider.tsx`, `src/api/select.ts`; contracts/orchestrator-api.md; R6 |
| FR-006, 007, 008 | `src/auth/`; contracts/orchestrator-api.md (AuthService); R8 |
| FR-009 | `packages/protocol/src/connection/`; contracts/transport.md |
| FR-010..013 | `src/features/sidebar/`, `src/features/overview/`, `src/components/`; data-model.md (Row identity, ordering) |
| FR-014..016 | `src/i18n/`; contracts/i18n-catalog.md; R2, R9 |
| FR-017, 018 | `scripts/client/bundle-check.mjs`, `.github/workflows/ci.yml` job `client`; R7, R11 |
| FR-019 | a11y lint rules, Playwright axe check, keyboard tests; R10 |
| FR-020 | `apps/client/app/spike.tsx` (dev only), `spike.md`; R12 |
| FR-021, SC-003 | Playwright screenshot spec, quickstart Part B |
| SC-001, 002 | Jest fixture order test over 100 scripted changes; Playwright latency assertion |
| SC-004 | eslint `no-literal-string`; pseudo-locale Playwright run |
| SC-005 | CI job timing budget; pnpm store cache |
| SC-006, 007 | bundle check; storage assertions in Playwright |
| SC-009 | Jest `--coverage` lcov fed to `scripts/diff-coverage.py` (R11) |

## Project Structure

### Documentation (this feature)

```text
specs/1735-rn-client-scaffold/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
├── contracts/orchestrator-api.md  contracts/transport.md  contracts/i18n-catalog.md
├── spike.md             # written during implementation (FR-020)
└── tasks.md             # tasks unit
```

### Source Code (repository root)

```text
package.json                     # new: private workspace root, scripts only
pnpm-workspace.yaml              # new: apps/*, packages/*
pnpm-lock.yaml                   # new
mise.toml                        # edit: add `pnpm` tool (node = "24" exists); tasks client-test, client-web, client-e2e
.github/workflows/ci.yml         # edit: `client` job, path-gated on apps/client, packages/protocol; runs `scripts/diff-coverage.py --lang web --lcov apps/client/coverage/lcov.info`; `ci-complete` needs the client job
.gitattributes                   # edit: `micold-client` attribute on apps/client/**, packages/protocol/**, pnpm-lock.yaml, pnpm-workspace.yaml, package.json (classify-change.sh reads it)
scripts/client/bundle-check.mjs  # new
scripts/guardrails-scope.conf    # edit: [web] gets apps/client/src/**, apps/client/app/**, packages/protocol/src/** (mock/ stays IN scope: it is test-support shipped in src and its logic is what SC-001 relies on, so it is covered like the rest; only generated files and tests are out)
scripts/classify-change.sh       # edit: new `client` verdict for apps/client/**, packages/protocol/**, pnpm files
scripts/tests/classify-change.test.sh  # edit: cases for the client verdict
docs/development/client.md       # new; linked from docs/SUMMARY.md
docs/user-guide/appearance-theming.md  # edit: language and theme-follows-system note

packages/protocol/               # new (name @micold/protocol)
├── src/types.ts                 # STAND-IN entity and enum types
├── src/api.ts                   # OrchestratorApi, AuthService, Transport interfaces
├── src/connection/              # ConnectionManager: backoff+jitter, resume by seq, correlation
├── src/mock/                    # in-memory fixture, feed, mock auth (exported at `@micold/protocol/mock`)
└── tests/                       # fake transport, order, reconnect, mock tests

apps/client/
├── app/                         # Expo Router: _layout, sign-in, index (Overview), spike (dev only)
├── src/theme/tokens.generated.ts   # exists, untouched
├── src/theme/{buildTheme,useColorScheme}.ts
├── src/api/{ApiProvider,select}.ts # the only place the mock may be named, behind the build switch
├── src/auth/                    # session store (memory), route guard, return-to
├── src/i18n/                    # catalogs en.json, pl.json, pseudo.ts; IntlProvider, locale store
├── src/components/              # StateIndicator, LocationRow, OverviewCard, ConnectionBanner, LocalePicker
├── src/features/{sidebar,overview}/
├── src/platform/                # Intl, matchMedia, font-loading wrappers
├── tests/                       # Jest
├── e2e/                         # Playwright (three browsers), screenshots
├── app.json  tsconfig.json  eslint.config.js  jest.config.js  playwright.config.ts
```

**Structure Decision**: pnpm workspace with one app and one shared package; the protocol package
owns types, interfaces, connection logic and the mock so that the real client replaces files inside it
and no app import path changes.

## Design notes

- **Mock switch**: `EXPO_PUBLIC_API_MODE=mock|real` read in `src/api/select.ts` through a dynamic
  `import('@micold/protocol/mock')` guarded by `process.env.EXPO_PUBLIC_API_MODE === 'mock'`; Metro/webpack
  inlines the env and dead-code-eliminates the branch in production. `client-web` sets `real`, `client-e2e`
  and dev set `mock`; a labelled `EXPO_PUBLIC_DEMO=1` build may keep it. Real mode has no implementation yet:
  `select.ts` throws a clear "real client not built (#1732)" error, covered by a test, so production
  export never silently uses the mock. The bundle check greps the export for a sentinel string
  `__MICOLD_MOCK__` that only mock modules contain, and the spike route likewise.
- **Stable order**: the Client does not sort. It renders the order of the list the API returns
  and keeps an `orderIndex` map; an update reuses existing keys. Tests assert identity-by-key and index
  equality after 100 changes and after batched changes (data-model.md).
- **Reconnect**: `ConnectionManager` over `Transport`, exponential backoff with full jitter (injectable
  clock and random), resume by last seen sequence, request/response correlation by id with timeout;
  exposes `ConnectionState` via subscribe. The sidebar keeps the last list on drop and marks it stale.
- **Responsive**: one breakpoint constant (600) in `src/theme/layout.ts`, used by a `useLayout()` hook over
  `useWindowDimensions`; wide columns follow the iced Overview arrangement, recorded as measured
  card width and gap from `ui/overview.rs` in research R13.
- **i18n**: ICU messages through `@formatjs/intl`; message ids are typed from `en.json`; a Jest test
  asserts every id in `pl.json` exists in `en.json` with matching placeholders and that Polish plurals have
  `one/few/many/other`. The pseudo-locale is generated from English (accent mapping, +40 percent padding,
  brackets) and exists only in test and dev builds.

## Test strategy

| Layer | Covers |
|---|---|
| Core (Rust) unit | none: no Rust change; drift check stays |
| Jest unit | theme equals tokens (US4); i18n fallback, plural, pseudo (US5); ConnectionManager with fake transport (FR-009); mock feed and order (FR-011, SC-001); auth store memory-only (FR-006); route guard and return-to (FR-007/008); `select.ts` real-mode error |
| Jest component | sign-in states, sidebar rows and empty states, offline/stale marking, card layout at widths, long names, indicator glyph/text (not colour alone) |
| Lint | no user-visible literal (FR-014, SC-004); a11y rules; logical properties (custom rule forbidding `left/right/marginLeft/...`) |
| Playwright smoke (3 browsers) | sign-in via mock, sidebar, Overview, order-stable change, locale switch and persistence, storage empty of tokens, keyboard path, 100 ms latency |
| Bundle check | no mock sentinel, no terminal emulator in `pnpm-lock`/bundle (FR-017, SC-006/007), with a self-test that a planted dependency fails it |
| Docs | `docs/development/client.md` and the user-guide edit are linked from `docs/SUMMARY.md`; the existing `docs` CI job verifies the links and the page build |
| Geometry/visual | Playwright screenshots (6) and quickstart Part B visual pass vs iced; the repo's iced geometry gates do not apply |
