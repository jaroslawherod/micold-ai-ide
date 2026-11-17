# Tasks: React Native (Expo) web client scaffold

**Input**: `specs/1735-rn-client-scaffold/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)

**Tests**: MANDATORY and first (Constitution I). In every group the test task precedes its implementation task and must be seen failing for the right reason before the implementation is written.

**Documentation**: Constitution VII: the user-guide tasks sit in the milestone that changes what a user sees (T020 in M1, T053 in M3).

**Format**: `[ID] [P?] [Story] Description`. `[P]` = different files, no dependency.

Paths: `apps/client/` (app), `packages/protocol/` (`@micold/protocol`), `scripts/client/`.

## Phase 1: Setup (US6)

- [x] T001 [US6] Create workspace files `package.json`, `pnpm-workspace.yaml` (`apps/*`, `packages/*`) and add `pnpm` plus tasks `client-test`, `client-web`, `client-e2e` to `mise.toml` (FR-001, FR-002; R1, R7)
- [x] T002 [US6] Scaffold `apps/client/` (Expo managed, Expo Router, react-native-web, `app.json`, `tsconfig.json` strict, `app/_layout.tsx` placeholder index) and `packages/protocol/` (`@micold/protocol`, `tsconfig`, empty `src/index.ts`); generate `pnpm-lock.yaml`
- [x] T003 [P] [US6] Configure `apps/client/eslint.config.js` (a11y rules, custom rule forbidding physical `left|right|margin*|padding*|border*` props per R14, no-literal-string rule per FR-014) and `jest.config.js` with coverage lcov output to `apps/client/coverage/`
- [x] T004 [P] [US6] Edit `.gitattributes` (`micold-client` on `apps/client/**`, `packages/protocol/**`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `package.json`)

## Phase 2: Foundational CI and guardrails (US6)

- [x] T005 [US6] TEST FIRST: add cases for the `client` verdict to `scripts/tests/classify-change.test.sh`; run it and see it fail
- [x] T006 [US6] Edit `scripts/classify-change.sh` to emit the `client` verdict from the `micold-client` attribute (makes T005 pass)
- [x] T007 [US6] Edit `scripts/guardrails-scope.conf` `[web]`: `apps/client/src/**`, `apps/client/app/**`, `packages/protocol/src/**` (generated files and tests excluded), per R11
- [x] T008 [US6] Add the `client` job to `.github/workflows/ci.yml` (mise install, pnpm store cache, lint, typecheck, Jest with coverage, `scripts/diff-coverage.py --lang web --lcov apps/client/coverage/lcov.info`, web export), path-gated on the `client` verdict; `ci-complete` needs it (FR-018, SC-005). Bundle check added in M2 (T039), Playwright in M4 (T055)
- [x] T009 [US6] Confirm `scripts/tests/tokens-ts-drift.test.sh` still passes and note it in the PR (US6 scenario 5)

## Phase 3: US4 Theme, US5 i18n core, platform fallbacks (M1)

- [x] T010 [P] [US4] TEST FIRST `apps/client/tests/theme/buildTheme.test.ts`: every token colour role equals the Paper MD3 role of the same name for light and dark, and an unmapped role fails (FR-003, US4 scenario 1; R4)
- [x] T011 [US4] Implement `apps/client/src/theme/buildTheme.ts` importing `tokens.generated.ts` (untouched), and `useColorScheme.ts` following the system scheme (US4 scenario 2); wire `PaperProvider` in `app/_layout.tsx`
- [x] T012 [P] [US4] TEST FIRST `apps/client/tests/theme/fonts.test.ts`: Roboto and Fira Mono families are registered in the theme and bundled as assets (US4 scenario 3, FR-003)
- [x] T013 [US4] Implement `src/platform/fonts.ts` bundling Roboto and Fira Mono (makes T012 pass)
- [x] T014 [P] [US5] TEST FIRST `apps/client/tests/platform/degrade.test.ts`: with `Intl` feature, `matchMedia` or the font-loading API missing, the Client falls back to English, light theme and system fonts without throwing (Edge Cases, Browser differences; Principle VI)
- [x] T015 [US5] Implement the guarded wrappers in `src/platform/` (Intl, matchMedia, font loading) with those fallbacks (makes T014 pass)
- [x] T016 [P] [US5] TEST FIRST `apps/client/tests/i18n/`: fallback to English, plural `one/few/many/other` for Polish, placeholder parity between `en.json` and `pl.json`, pseudo-locale (accent, +40 percent, brackets), preference stored only as a locale code and tolerant of throwing storage, default from browser language, nearest-language fallback (FR-014, FR-015, US5 scenarios 2-4; R9)
- [x] T017 [US5] Implement `src/i18n/` (`en.json`, `pl.json`, `pseudo.ts` dev/test only, `IntlProvider`, locale store, typed ids) and `src/theme/layout.ts` logical-property helpers (FR-016); make T016 pass
- [x] T018 [P] [US5] TEST FIRST component test for `src/components/LocalePicker.tsx`: lists locales, switches without reload, keyboard operable, named control (FR-015, FR-019)
- [x] T019 [US5] Implement `src/components/LocalePicker.tsx` (makes T018 pass)
- [x] T020 [P] [US4] Update `docs/user-guide/appearance-theming.md`: the web client follows the system theme and has a language picker (Constitution VII)

## Phase 4: US7 Spike (M1), before any sidebar work (FR-020)

- [x] T021 [US7] TEST FIRST: a Jest test that the spike route is registered only in dev/mock mode and absent otherwise (US7 scenario 3)
- [x] T022 [US7] Implement `apps/client/app/spike.tsx` (5,000 transcript entries, 2,000-line diff with Paper `Text`/`FlatList`, measuring with `performance.now`) (R12)
- [x] T023 [US7] Run the spike in the web export in Chromium; write `specs/1735-rn-client-scaffold/spike.md` with render time, scroll, memory, selection/copy and gaps, each gap naming a follow-up or decision (SC-008, US7 scenarios 1-2)

**M1 checkpoint**: `mise run client-test` and `mise run client-web` pass; app shows themed placeholder with locale picker; spike recorded.

## Phase 5: US1 Sign-in and protocol boundary (M2)

- [x] T024 [P] [US1] TEST FIRST `packages/protocol/tests/types.test-d.ts` and `api.test.ts`: branded ids not interchangeable, `ConnectionState` exhaustive, interfaces implementable by a stub (data-model.md)
- [x] T025 [US1] Implement `packages/protocol/src/types.ts` (marked `STAND-IN`) and `src/api.ts` (`OrchestratorApi`, `AuthService`, `Transport`) per contracts/orchestrator-api.md and contracts/transport.md
- [x] T026 [P] [US1] TEST FIRST `packages/protocol/tests/connection.test.ts`: ConnectionManager over a fake transport with injectable clock and random: backoff with full jitter, resume by last seq, correlation by id with timeout, state surfaced as connected/reconnecting(attempt)/offline (FR-009)
- [x] T027 [US1] Implement `packages/protocol/src/connection/` (makes T026 pass)
- [x] T028 [P] [US1] TEST FIRST `packages/protocol/tests/mock.test.ts`: mock fixture (2 daemons, 6 projects, 30 worktrees, 60 sessions), feed delivery, mock auth providers (two configured, one not), refusal and cancel (FR-005, SC-001 data)
- [x] T029 [US1] Implement `packages/protocol/src/mock/` exporting the `__MICOLD_MOCK__` sentinel, at `@micold/protocol/mock`
- [x] T030 [P] [US1] TEST FIRST `apps/client/tests/api/select.test.ts`: mock mode selects the mock, real mode throws "real client not built (#1732)", components import only the interface (FR-004, FR-005)
- [x] T031 [US1] Implement `src/api/ApiProvider.tsx` and `src/api/select.ts` behind `EXPO_PUBLIC_API_MODE`; `client-web` sets `real`, dev and `client-e2e` set `mock`
- [x] T032 [P] [US1] TEST FIRST `apps/client/tests/auth/`: token held in memory only, no `toJSON`, nothing written to `localStorage`/`sessionStorage`/`IndexedDB`/cookie (FR-006); route guard redirects to sign-in and returns to the requested route (FR-007, FR-008); session expiry mid-use returns to sign-in keeping the open route; two stores (tabs) signed in separately never share a token (Edge Cases, Auth)
- [x] T033 [US1] Implement `src/auth/` (memory session store, guard, return-to, expiry handling) and the route guard in `app/_layout.tsx` (makes T032 pass)
- [x] T034 [P] [US1] TEST FIRST component tests for `app/sign-in.tsx`: lists only configured providers, "not set up" state, refusal and cancellation messages, all strings from the catalog (US1 scenarios 1, 2, 5)
- [x] T035 [US1] Implement `app/sign-in.tsx` and add catalog entries (en, pl) (makes T034 pass)
- [x] T036 [P] [US1] TEST FIRST component test for `src/components/ConnectionBanner.tsx`: connected, reconnecting with attempt count, offline (FR-009)
- [x] T037 [US1] Implement `src/components/ConnectionBanner.tsx` (makes T036 pass)
- [x] T038 [P] [US1] TEST FIRST self-test of `scripts/client/bundle-check.mjs` (`scripts/tests/bundle-check.test.sh`): an export containing the mock sentinel fails, a clean one passes (SC-006)
- [x] T039 [US1] Implement the mock-sentinel check in `scripts/client/bundle-check.mjs` and add the bundle-check step to the `client` job in `.github/workflows/ci.yml` (makes T038 pass)

**M2 checkpoint**: signing in through the mock lands on an authenticated shell; reload signs out; bundle check passes on a real-mode export.

## Phase 6: US2 Sidebar and US3 Overview (M3)

- [x] T040 [US3] Read `crates/micold-client/src/ui/overview.rs` and record card width, gap and column rule in `research.md` R13 (before layout code)
- [x] T041 [P] [US2] TEST FIRST `packages/protocol/tests/order.test.ts` and `apps/client/tests/sidebar/order.test.ts`: key identity and index equality after 100 scripted changes and after batched changes; batch result independent of arrival order for distinct keys (property test) (FR-011, SC-001, US2 scenarios 2, 5)
- [x] T042 [US2] Implement the snapshot store (`zustand`, `orderIndex`, no sorting in the client) in `src/features/sidebar/store.ts` and row keys per data-model.md
- [x] T043 [P] [US2] TEST FIRST component tests: `StateIndicator` differs by glyph and text not colour alone and changes without animation under `prefers-reduced-motion`; `LocationRow`; long names truncate with full text available and the indicator does not move; empty states; terminal sessions listed without an emulator (US2 scenarios 1, 4, 6; FR-011; Edge Cases)
- [x] T044 [US2] Implement `src/components/StateIndicator.tsx` (honours reduced motion), `LocationRow.tsx` and `src/features/sidebar/Sidebar.tsx` grouped by daemon and project, keyboard reachable with visible focus (FR-010, FR-019)
- [x] T045 [P] [US2] TEST FIRST offline/stale tests: an offline daemon or non-connected state keeps rows listed, marked stale, with last known data (FR-013, US2 scenario 3)
- [x] T046 [US2] Implement stale marking in the store and `Sidebar` plus catalog strings (en, pl)
- [x] T047 [P] [US3] TEST FIRST layout tests: wide arrangement at 600 px and above, one column at 599 px and below, widths below 320 px and a 200 percent zoom equivalent and resize while open stay reachable with no horizontal scroll, card state changes do not move the card (FR-012, SC-003, US3 scenarios 1-3; Edge Cases, Viewport)
- [x] T048 [US3] Extend `src/theme/layout.ts` with `useLayout()` (breakpoint 600), add `src/components/OverviewCard.tsx`, `src/features/overview/` and `app/index.tsx` with the sidebar beside it (makes T047 pass)
- [x] T049 [P] [US2] TEST FIRST latency test: a state change from the mock feed reaches the rendered indicator within 100 ms (SC-002)
- [x] T050 [US2] Make T049 pass (adjust subscription and render path only if it fails); no new feature code if already green
- [x] T051 [P] [US5] Extend the i18n tests: every string on sidebar, Overview and sign-in is from the catalog and Polish plurals render for counts (SC-004); lint passes with no literal
- [x] T052 [US3] Add dates and relative times through the intl wrapper in `src/platform/` (US5 scenario 4)
- [x] T053 [P] [US3] Update `docs/user-guide/overview.md` (web client Overview and sidebar: stacking below 600 px, state glyphs)

**M3 checkpoint**: signed in, the sidebar and Overview show the fixture in stable order, update live, stack on narrow widths.

## Phase 7: Browser smoke, visual pass, hardening, docs (M4)

- [x] T054 [P] [US6] TEST FIRST `apps/client/e2e/smoke.spec.ts` (Playwright; Chromium, Firefox, WebKit): sign-in via mock, sidebar, Overview, order-stable change, locale switch and persistence, storage holds no token, keyboard path, 100 ms latency, axe check, no horizontal scroll at 300 px and 200 percent zoom, reduced-motion emulation (FR-018, FR-019, SC-002, SC-006)
- [x] T055 [US6] Add `apps/client/playwright.config.ts` (mock build, three browsers), the `client-e2e` task body and the `client-e2e` CI job (own timeout, `ci-complete` needs it, Playwright browser cache keyed on the lockfile); make T054 pass
- [x] T056 [P] [US5] Playwright pseudo-locale run: 40 percent growth with no clipped indicator (SC-004)
- [x] T057 [US3] Playwright screenshot spec: 1280, 800, 390 px in light and dark (6) in Chromium to `apps/client/e2e/screenshots/` (FR-021, SC-003)
- [x] T058 [P] [US6] TEST FIRST extend `scripts/tests/bundle-check.test.sh`: a planted terminal-emulator dependency in `pnpm-lock.yaml` or the export fails the check (FR-017, SC-007)
- [x] T059 [US6] Harden `scripts/client/bundle-check.mjs` for terminal emulator packages (makes T058 pass)
- [x] T060 [US3] Run the visual pass (skill `visual-pass`, quickstart Part B) against the spec (FR-011, FR-021, SC-003); record the result in `specs/1735-rn-client-scaffold/visual-pass/`
- [ ] T060a [US3] Follow-up, deferred: side-by-side comparison of the web client with the iced app's Overview and sidebar (needs a desktop session; the iced app is being sunset, 2026-11-15)
- [x] T061 [P] [US6] Write `docs/development/client.md` (layout, tasks, mock switch, protocol package stand-in, how #1732 swaps in) and link it from `docs/SUMMARY.md`
- [ ] T062 [US6] Confirm SC-005 (client job under 10 minutes) and SC-009 (diff coverage at least 80 percent) from the CI run; record in the PR

## Dependencies

T001 -> T002 -> everything. T005 -> T006; T006, T007 -> T008. T021 -> T022 -> T023 precede T040 (FR-020). T025 precedes T027, T029, T031. T029 precedes T030, T039. T031 -> T033, T035. T042 precedes T044, T046. T040 precedes T047/T048. T044 -> T048. T054 -> T055, T056.

## Implementation Strategy

MVP is M1 to M3 in order; each milestone is a PR that leaves `main` green. Test tasks run red before their implementation.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Workspace, CI, theme, i18n core, spike 🎯 MVP

- **Tasks**: T001–T023
- **Deliverable**: `mise run client-test` and `mise run client-web` pass, CI runs a `client` job, the app renders a Material 3 themed page from the tokens in English or Polish with a language picker, and the spike results are recorded.
- **Satisfies**: US4 scenarios 1–3; US5 scenarios 2–4; US6 scenarios 1, 2, 5; US7 scenarios 1–3; FR-001–003, 014–016, 020; SC-008
- **Verify**: `mise run client-test && mise run client-web`; `scripts/tests/classify-change.test.sh`; `specs/1735-rn-client-scaffold/spike.md`
- **Depends on**: —
- **Tier**: full

### M2 — Protocol package, mock, reconnect, sign-in

- **Tasks**: T024–T039
- **Deliverable**: A person signs in through the mock providers, is redirected to sign-in when signed out, and the connection banner shows reconnect state; the production export contains no mock.
- **Satisfies**: US1 scenarios 1–5; FR-004–009; SC-006 (bundle part)
- **Verify**: `mise run client-test`; `mise run client-web && node scripts/client/bundle-check.mjs apps/client/dist`
- **Depends on**: M1
- **Tier**: full

### M3 — Sidebar and Overview

- **Tasks**: T040–T053
- **Deliverable**: After sign-in the sidebar and Overview list the fixture in stable order, update state live, mark offline daemons stale, and stack below 600 px; the user guide describes it.
- **Satisfies**: US2 scenarios 1–6; US3 scenarios 1–3; US5 scenario 4; FR-010–013, 019; SC-001–004; edge cases Viewport, Reduced motion
- **Verify**: `mise run client-test`; `mise run client-web` then the dev loop of quickstart Part A step 5
- **Depends on**: M2
- **Tier**: full

### M4 — Browser smoke, visual pass, bundle check, docs

- **Tasks**: T054–T062
- **Deliverable**: CI runs the Playwright smoke in Chromium, Firefox and WebKit, captures six screenshots, enforces the terminal-emulator bundle check, and `docs/development/client.md` exists with the visual pass recorded.
- **Satisfies**: US3 scenario 4; US6 scenarios 3, 4; FR-017, 018, 021; SC-003, 005, 006, 007, 009
- **Verify**: `mise run client-e2e`; `node scripts/client/bundle-check.mjs apps/client/dist`; `specs/1735-rn-client-scaffold/visual-pass/`
- **Depends on**: M3
- **Tier**: full

## Phase 8: TDD remediation

The TDD verification (`tdd/verification.md`) is FAIL: the feature is not done until T063-T065 are cleared.

- [x] T063 [US1] Finding 1: in `packages/protocol/tests/connection.test.ts:62` assert `{ kind: "reconnecting", attempt: 2 }` after the `maxAttempts`-th drop and `offline` only after the next; prove by mutating `ConnectionManager.ts:132` `>` to `>=` (test fails), restore, `pnpm --dir apps/client test` green
- [x] T064 [US2] Finding 2: remove the `if (c.kind === "session-state")` in `packages/protocol/tests/order.test.ts:53` (filter the map to session-state changes first and assert unconditionally); `pnpm --dir apps/client test` green
- [ ] T065 [US6] Finding 3: add `## M4` to `tdd/cycle-log.md` with a recorded red for T054, T056, T058 (and a bundle-check red for T058 via `scripts/tests/bundle-check.test.sh` against the pre-hardening script), and record that T045 landed green and was shown to bite; `grep -c "T058" specs/1735-rn-client-scaffold/tdd/cycle-log.md` is 1 or more
- [ ] T066 [US2] Finding 4: replace the wall-clock assertion in `apps/client/tests/sidebar/latency.test.tsx:85` with a render-count or fake-timer check, leaving the 100 ms bound to the Playwright spec; `pnpm --dir apps/client test` green
- [ ] T067 [US1] Finding 5: drop the source-grep test at `apps/client/tests/auth/sessionStore.test.ts:44`, relying on the behavioral test at line 12
- [ ] T068 [US6] Finding 7: run `/speckit-tdd-setup` for the pnpm/Jest stack (command, exemplar, helpers `apps/client/tests/support/renderWithApi.tsx`, `packages/protocol/tests/fakes.ts`) and `/speckit-tdd-plan` to create `tdd/test-list.md`; re-run `/speckit-tdd-verify`
- [ ] T069 [US4] Finding 6: rename or complete the test at `apps/client/tests/app/layout.test.tsx:56` so its name matches what it asserts

