# Tasks: React Native (Expo) web client scaffold

**Input**: `specs/1735-rn-client-scaffold/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)

**Tests**: MANDATORY and first (Constitution I). In every group the test task precedes its implementation task and must be seen failing for the right reason before the implementation is written.

**Documentation**: Constitution VII: the user-guide tasks sit in the milestone that changes what a user sees (T015 in M1, T045 in M3).

**Format**: `[ID] [P?] [Story] Description`. `[P]` = different files, no dependency.

Paths: `apps/client/` (app), `packages/protocol/` (`@micold/protocol`), `scripts/client/`.

## Phase 1: Setup (US6)

- [ ] T001 [US6] Create workspace files `package.json`, `pnpm-workspace.yaml` (`apps/*`, `packages/*`) and add `pnpm` plus tasks `client-test`, `client-web`, `client-e2e` to `mise.toml` (FR-001, FR-002; R1, R7)
- [ ] T002 [US6] Scaffold `apps/client/` (Expo managed, Expo Router, react-native-web, `app.json`, `tsconfig.json` strict, `app/_layout.tsx` placeholder index) and `packages/protocol/` (`@micold/protocol`, `tsconfig`, empty `src/index.ts`); generate `pnpm-lock.yaml`
- [ ] T003 [P] [US6] Configure `apps/client/eslint.config.js` (a11y rules, `no-restricted-syntax`/custom rule forbidding physical `left|right|margin*|padding*|border*` props per R14, no-literal-string rule per FR-014) and `jest.config.js` with coverage lcov output to `apps/client/coverage/`
- [ ] T004 [P] [US6] Edit `.gitattributes` (`micold-client` on `apps/client/**`, `packages/protocol/**`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `package.json`)

## Phase 2: Foundational CI and guardrails (US6)

- [ ] T005 [US6] TEST FIRST: add cases for the `client` verdict to `scripts/tests/classify-change.test.sh`; run it and see it fail
- [ ] T006 [US6] Edit `scripts/classify-change.sh` to emit the `client` verdict from the `micold-client` attribute (makes T005 pass)
- [ ] T007 [US6] Edit `scripts/guardrails-scope.conf` `[web]`: `apps/client/src/**`, `apps/client/app/**`, `packages/protocol/src/**` (generated files and tests excluded), per R11
- [ ] T008 [US6] Add the `client` job to `.github/workflows/ci.yml` (mise install, pnpm store cache, lint, typecheck, Jest with coverage, `scripts/diff-coverage.py --lang web --lcov apps/client/coverage/lcov.info`, web export, bundle check; Playwright added in M4), path-gated on the `client` verdict; `ci-complete` needs it (FR-018, SC-005)
- [ ] T009 [US6] Confirm `scripts/tests/tokens-ts-drift.test.sh` still passes and note it in the PR (US6 scenario 5)

## Phase 3: US4 Theme from tokens (P2) and US5 i18n core (P1), M1

- [ ] T010 [P] [US4] TEST FIRST `apps/client/tests/theme/buildTheme.test.ts`: every token colour role equals the Paper MD3 role of the same name for light and dark, and an unmapped role fails (FR-003, US4 scenario 1; R4)
- [ ] T011 [US4] Implement `apps/client/src/theme/buildTheme.ts` importing `tokens.generated.ts` (untouched), and `useColorScheme.ts` following the system scheme (US4 scenario 2); wire `PaperProvider` in `app/_layout.tsx`
- [ ] T012 [US4] Bundle Roboto and Fira Mono via `src/platform/fonts.ts` with a test that the font families are registered in the theme (US4 scenario 3)
- [ ] T013 [P] [US5] TEST FIRST `apps/client/tests/i18n/`: fallback to English, plural `one/few/many/other` for Polish, placeholder parity between `en.json` and `pl.json`, pseudo-locale (accent, +40 percent, brackets), preference stored only as a locale code and tolerant of throwing storage, default from browser language (FR-014, FR-015, US5 scenarios 2-4; R9)
- [ ] T014 [US5] Implement `src/i18n/` (`en.json`, `pl.json`, `pseudo.ts` dev/test only, `IntlProvider`, locale store, typed ids), `src/components/LocalePicker.tsx` with a component test, `src/theme/layout.ts` logical-property helpers (FR-016); make T013 pass
- [ ] T015 [P] [US4] Update `docs/user-guide/appearance-theming.md`: the web client follows the system theme and has a language picker (Constitution VII)

## Phase 4: US7 Spike (P1), M1, before any sidebar work (FR-020)

- [ ] T016 [US7] TEST FIRST: a Jest test that the spike route is registered only when `__DEV__`/mock mode and absent otherwise (US7 scenario 3)
- [ ] T017 [US7] Implement `apps/client/app/spike.tsx` (5,000 transcript entries, 2,000-line diff with Paper `Text`/`FlatList`, measuring with `performance.now`) (R12)
- [ ] T018 [US7] Run the spike in the web export in Chromium; write `specs/1735-rn-client-scaffold/spike.md` with render time, scroll, memory, selection/copy and gaps, each gap naming a follow-up or decision (SC-008, US7 scenarios 1-2)

**M1 checkpoint**: `mise run client-test` and `mise run client-web` pass; app shows themed placeholder with locale picker; spike recorded.

## Phase 5: US1 Sign-in and protocol boundary (P1), M2

- [ ] T019 [P] [US1] TEST FIRST `packages/protocol/tests/types.test-d.ts` and `api.test.ts`: branded ids not interchangeable, `ConnectionState` exhaustive, interfaces implementable by a stub (data-model.md)
- [ ] T020 [US1] Implement `packages/protocol/src/types.ts` (marked `STAND-IN`) and `src/api.ts` (`OrchestratorApi`, `AuthService`, `Transport`) per contracts/orchestrator-api.md and contracts/transport.md
- [ ] T021 [P] [US1] TEST FIRST `packages/protocol/tests/connection.test.ts`: ConnectionManager over a fake transport with injectable clock and random: backoff with full jitter, resume by last seq, correlation by id with timeout, state surfaced as connected/reconnecting(attempt)/offline (FR-009)
- [ ] T022 [US1] Implement `packages/protocol/src/connection/` (makes T021 pass)
- [ ] T023 [P] [US1] TEST FIRST `packages/protocol/tests/mock.test.ts`: mock fixture (2 daemons, 6 projects, 30 worktrees, 60 sessions), feed delivery, mock auth providers (two configured, one not), refusal and cancel (FR-005, SC-001 data)
- [ ] T024 [US1] Implement `packages/protocol/src/mock/` exporting `__MICOLD_MOCK__` sentinel, at `@micold/protocol/mock`
- [ ] T025 [P] [US1] TEST FIRST `apps/client/tests/api/select.test.ts`: mock mode selects the mock, real mode throws "real client not built (#1732)", components import only the interface (FR-004, FR-005)
- [ ] T026 [US1] Implement `src/api/ApiProvider.tsx` and `src/api/select.ts` behind `EXPO_PUBLIC_API_MODE`; `client-web` sets `real`, dev and `client-e2e` set `mock`
- [ ] T027 [P] [US1] TEST FIRST `apps/client/tests/auth/`: token held in memory only, no `toJSON`, nothing written to `localStorage`/`sessionStorage`/`IndexedDB`/cookie (FR-006); route guard redirects to sign-in and returns to the requested route (FR-007, FR-008)
- [ ] T028 [US1] Implement `src/auth/` (memory session store, guard, return-to) and route guard in `app/_layout.tsx`
- [ ] T029 [P] [US1] TEST FIRST component tests for `app/sign-in.tsx`: lists only configured providers, "not set up" state, refusal and cancellation messages, all strings from the catalog (US1 scenarios 1, 2, 5)
- [ ] T030 [US1] Implement `app/sign-in.tsx` and `src/components/ConnectionBanner.tsx` (with its test), add catalog entries (en, pl)
- [ ] T031 [US1] Add `scripts/client/bundle-check.mjs` mock-sentinel check and its self-test that a planted mock reference fails it; run in the `client` job (SC-006)

**M2 checkpoint**: signing in through the mock lands on an authenticated shell; reload signs out; bundle check passes on a real-mode export.

## Phase 6: US2 Sidebar and US3 Overview (P1), M3

- [ ] T032 [US3] Read `crates/micold-client/src/ui/overview.rs` and record card width, gap and column rule in `research.md` R13 (before layout code)
- [ ] T033 [P] [US2] TEST FIRST `packages/protocol/tests/order.test.ts` and `apps/client/tests/sidebar/order.test.ts`: key identity and index equality after 100 scripted changes and after batched changes; batch result independent of arrival order for distinct keys (property test) (FR-011, SC-001, US2 scenarios 2, 5)
- [ ] T034 [US2] Implement the snapshot store (`zustand`, `orderIndex`, no sorting in the client) in `src/features/sidebar/store.ts` and row keys per data-model.md
- [ ] T035 [P] [US2] TEST FIRST component tests: `StateIndicator` differs by glyph and text not colour alone; `LocationRow`; long names truncate; empty states (no daemons, daemon with no projects); terminal sessions listed without an emulator (US2 scenarios 1, 4, 6; FR-011)
- [ ] T036 [US2] Implement `src/components/StateIndicator.tsx`, `LocationRow.tsx` and `src/features/sidebar/Sidebar.tsx` grouped by daemon and project, keyboard reachable with visible focus (FR-010, FR-019)
- [ ] T037 [P] [US2] TEST FIRST offline/stale tests: an offline daemon or non-connected state keeps rows listed, marked stale, with last known data (FR-013, US2 scenario 3)
- [ ] T038 [US2] Implement stale marking in the store and `Sidebar` plus catalog strings (en, pl)
- [ ] T039 [P] [US3] TEST FIRST layout tests: wide arrangement at 600 px and above, one column at 599 px and below at every width, card state changes do not move the card (FR-012, SC-003, US3 scenarios 1-3)
- [ ] T040 [US3] Implement `src/theme/layout.ts` `useLayout()` (breakpoint 600), `src/components/OverviewCard.tsx`, `src/features/overview/` and `app/index.tsx` with the sidebar beside it
- [ ] T041 [P] [US2] TEST FIRST latency test: a state change from the mock feed reaches the rendered indicator within 100 ms (SC-002)
- [ ] T042 [US2] Make T041 pass (adjust subscription and render path only if it fails); no new feature code if already green
- [ ] T043 [P] [US5] Extend the i18n tests: every string on sidebar, Overview and sign-in is from the catalog and Polish plurals render for counts (SC-004); lint passes with no literal
- [ ] T044 [US3] Add dates and relative times through the intl wrapper in `src/platform/` (US5 scenario 4)
- [ ] T045 [P] [US3] Update `docs/user-guide/overview.md` (web client Overview and sidebar: stacking below 600 px, state glyphs)

**M3 checkpoint**: signed in, the sidebar and Overview show the fixture in stable order, update live, stack on narrow widths.

## Phase 7: Browser smoke, visual pass, hardening, docs (US6, US3), M4

- [ ] T046 [P] [US6] TEST FIRST `apps/client/e2e/smoke.spec.ts` (Playwright; Chromium, Firefox, WebKit): sign-in via mock, sidebar, Overview, order-stable state change, locale switch and persistence, storage holds no token, keyboard path, 100 ms latency, axe check (FR-018, FR-019, SC-002, SC-006)
- [ ] T047 [US6] Add `apps/client/playwright.config.ts` (mock build, three browsers), the `client-e2e` task body and the Playwright step in the `client` job (browser cache); make T046 pass
- [ ] T048 [P] [US5] Playwright pseudo-locale run: 40 percent growth with no clipped indicator (SC-004)
- [ ] T049 [US3] Playwright screenshot spec: 1280, 800, 390 px in light and dark (6) in Chromium to `apps/client/e2e/screenshots/` (FR-021, SC-003)
- [ ] T050 [P] [US6] Harden `scripts/client/bundle-check.mjs`: no terminal emulator package in `pnpm-lock.yaml` or the export, with a self-test that a planted dependency fails it (FR-017, SC-007)
- [ ] T051 [US3] Run the visual pass (skill `visual-pass`, quickstart Part B) against the iced app; record the result in `specs/1735-rn-client-scaffold/visual-pass/`
- [ ] T052 [P] [US6] Write `docs/development/client.md` (layout, tasks, mock switch, protocol package stand-in, how #1732 swaps in) and link it from `docs/SUMMARY.md`
- [ ] T053 [US6] Confirm SC-005 (client job under 10 minutes) and SC-009 (diff coverage at least 80 percent) from the CI run; record in the PR

## Dependencies

T001 -> T002 -> everything. T005 -> T006. T016 -> T017 -> T018 precede T032 (FR-020). T020 precedes T022, T024, T026. T024 precedes T025, T031. T034 precedes T036, T038. T032 precedes T039/T040.

## Implementation Strategy

MVP is M1 to M3 in order; each milestone is a PR that leaves `main` green. Test tasks run red before their implementation.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Workspace, CI, theme, i18n core, spike 🎯 MVP

- **Tasks**: T001–T018
- **Deliverable**: `mise run client-test` and `mise run client-web` pass, CI runs a `client` job, the app renders a Material 3 themed page from the tokens in English or Polish with a language picker, and the spike results are recorded.
- **Satisfies**: US4 scenarios 1–3; US5 scenarios 2–4; US6 scenarios 1, 2, 5; US7 scenarios 1–3; FR-001–003, 014–016, 020; SC-005, SC-008
- **Verify**: `mise run client-test && mise run client-web`; `scripts/tests/classify-change.test.sh`; `specs/1735-rn-client-scaffold/spike.md`
- **Depends on**: —
- **Tier**: full

### M2 — Protocol package, mock, reconnect, sign-in

- **Tasks**: T019–T031
- **Deliverable**: A person signs in through the mock providers, is redirected to sign-in when signed out, and the connection banner shows reconnect state; the production export contains no mock.
- **Satisfies**: US1 scenarios 1–5; FR-004–009; SC-006 (bundle part)
- **Verify**: `mise run client-test`; `mise run client-web && node scripts/client/bundle-check.mjs apps/client/dist`
- **Depends on**: M1
- **Tier**: full

### M3 — Sidebar and Overview

- **Tasks**: T032–T045
- **Deliverable**: After sign-in the sidebar and Overview list the fixture in stable order, update state live, mark offline daemons stale, and stack below 600 px; the user guide describes it.
- **Satisfies**: US2 scenarios 1–6; US3 scenarios 1–3; US5 scenario 4; FR-010–013, 019; SC-001–004
- **Verify**: `mise run client-test`; `mise run client-web` then the dev loop of quickstart Part A step 5
- **Depends on**: M2
- **Tier**: full

### M4 — Browser smoke, visual pass, bundle check, docs

- **Tasks**: T046–T053
- **Deliverable**: CI runs the Playwright smoke in Chromium, Firefox and WebKit, captures six screenshots, enforces the terminal-emulator bundle check, and `docs/development/client.md` exists with the visual pass recorded.
- **Satisfies**: US3 scenario 4; US6 scenarios 3, 4; FR-017, 018, 021; SC-003, 006, 007, 009
- **Verify**: `mise run client-e2e`; `node scripts/client/bundle-check.mjs apps/client/dist`; `specs/1735-rn-client-scaffold/visual-pass/`
- **Depends on**: M3
- **Tier**: full
