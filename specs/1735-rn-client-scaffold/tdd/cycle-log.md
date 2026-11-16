# TDD cycle log: #1735 M1 (T001-T023)

Each row: the test written first, how it failed (red), the smallest change that passed it (green).
Commands: `scripts/tests/classify-change.test.sh`; `pnpm --filter @micold/client test`.

| Task | Test (first) | Red | Green |
|---|---|---|---|
| T005/T006 | 16 `client=` cases in `scripts/tests/classify-change.test.sh` | `16 of 82 case(s) failed` (`got=<none>`: no `client=` output) | `client` verdict from attribute `micold-client`: 82 of 82 |
| T010/T011 | `tests/theme/buildTheme.test.ts` | `Cannot find module '../../src/theme/buildTheme'` | `buildTheme` maps token roles to Paper roles, extension roles, throws `UnmappedRoleError` |
| T012/T013 | `tests/theme/fonts.test.ts` | module `src/platform/fonts` missing | fonts bundled from `@expo-google-fonts`, families on the theme; needed a `.ttf` stub mapper in Jest |
| T014/T015 | `tests/platform/degrade.test.ts` | modules `platform/intl`, `colorScheme` missing | guarded wrappers: light without `matchMedia`, English without `Intl`, system fonts without a loader |
| T016/T017 | `tests/i18n/*.test.ts` | modules `i18n/*` missing; catalogs absent | catalogs en/pl, `pseudoLocalize`, `createTranslator`, locale store; ESM packages added to `transformIgnorePatterns` |
| T018/T019 | `tests/components/LocalePicker.test.tsx` | `Cannot find module`; then Paper `SegmentedButtons` forced role `button` (no `radio`) | custom radio group on `Pressable` |
| T021/T022 | `tests/app/spike.test.tsx`, `spikeFixtures`, `spikeScreen` | modules missing; first route draft read the env at import (untestable), moved the check into render | route renders the spike only when `EXPO_PUBLIC_API_MODE === "mock"` |
| M3 round-1 A F1 | `packages/protocol/tests/order.test.ts` | stale replay (seq 3 and 5 after seq 5) overwrote state | `applyChanges` drops `seq <= snap.seq` |
| M3 round-1 A F2 | `order.test.ts` (both) | location upserted after a d2 row keyed after d2; orphan keyed `undefined/...` | `rowKeys` groups daemon, project, location; `orderIndex` flattens `sidebarTree` (client test green once `rowKeys` was fixed) |
| M3 round-1 A F3, F6 / B F1 | `components.test.tsx` | no "No sessions" text; session rows not accessible | empty-sessions note; `sidebar.session` "{name}, {state}" in en and pl |
| M3 round-1 A F4 | `guard.test.ts` | `/SIGN-IN`, `/sign-in/`, `/sign%2Din` returned `null` (router 404s) | non-canonical sign-in redirects to `/sign-in` |
| M3 round-1 A F5 | `overview/layout.test.tsx` | 1000 px grid gave 322.67 px cards | `Math.floor` on card width |

Refactor while green: `chooseLocale` takes an injectable `languages` source so the locale store test needs no `navigator` spy.

## M2

| Tasks | Test | Red (why it failed) | Green |
|---|---|---|---|
| T024/T025 | `packages/protocol/tests/api.test.ts`, `types.test-d.ts` | modules `src/api`, `src/types` missing | branded ids, `ConnectionState` union with `assertNever`, `createAuthSession` with a non-enumerable token |
| T026/T027 | `packages/protocol/tests/connection.test.ts` | module `src/connection` missing; review round: gap snapshot sent twice, unknown error code passed through, retryNow reopened while connected | `ConnectionManager`: full jitter, offline after maxAttempts, resume, one in-flight snapshot retried on next open, codes validated, `onUnauthenticated` |
| T028/T029 | `packages/protocol/tests/mock.test.ts` | module `src/mock` missing; then `structuredClone` undefined in Jest; review round: sentinel not used at runtime | fixture 2/6/30/60, feed, mock auth; JSON clone; sentinel set on `globalThis` |
| T030/T031 | `apps/client/tests/api/select.test.ts`, `ApiProvider.test.tsx` | module `src/api/select` missing | synchronous guarded `require` of the mock; real mode throws and the app shows `api.unavailable` |
| T032/T033 | `apps/client/tests/auth/*` | modules `src/auth/*` missing; review round: protected child rendered before the redirect, `/sign-in/` and control characters accepted as `returnTo` | memory store, `(app)` group gate returning `Redirect`, URL-parsed `returnTo` |
| T034/T035 | `apps/client/tests/app/signIn.test.tsx` | `app/sign-in` missing; catalog test refused `signIn.*` ids; review round: a rejected sign-in left the buttons disabled | sign-in page, `signin.*` messages en/pl, `signin.failed` with retry |
| T036/T037 | `apps/client/tests/components/ConnectionBanner.test.tsx` | component missing; then a `View` with a role but not `accessible` was not found by role | banner over `useSyncExternalStore` |
| T038/T039 | `scripts/tests/bundle-check.test.sh` | `scripts/client/bundle-check.mjs` missing | sentinel scan, exit 0/1/2; CI checks the real export and that a mock export fails |

Refactor while green: `renderWithApi` test helper shared by the sign-in and banner tests.

## M3

| Tasks | Test | Red (why it failed) | Green |
|---|---|---|---|
| T040 | - | - | R13 numbers measured from `card_grid.rs` (280 min, 16 gap, 5 max, iced column rule) and recorded in research.md |
| T041/T042 | `packages/protocol/tests/order.test.ts`, `apps/client/tests/sidebar/order.test.ts` | `applyChanges`/`rowKeys` not functions; client module `features/sidebar/store` missing | pure `applyChanges` in the protocol package (the mock now uses it), `rowKeys`, zustand snapshot store, `orderIndex`, `sidebarTree` |
| T043/T044 | `apps/client/tests/sidebar/components.test.tsx` | components missing; then unawaited async `unmount`/`fireEvent` of Testing Library 14 broke later renders | `StateIndicator` (glyph + text, pulse off under reduced motion), `LocationRow` with focus ring, `Sidebar` with loading, failed and empty states |
| T045/T046 | `apps/client/tests/sidebar/stale.test.tsx` | green on arrival (stale marking landed with the store and Sidebar); shown to bite in round-1 fixes: with `isStale` stubbed to `false` all 3 stale tests and the store stale test fail (4 failed) | rows of an offline daemon or a dropped connection stay, labelled "last known", with the update time |
| T047/T048 | `apps/client/tests/overview/layout.test.tsx` | `gridColumns`, `Overview`, `OverviewCard` missing; then a pulsing indicator kept `act` from settling in the index test | `gridColumns`, wide row at 600 px, stacked column at 599 px, measured grid width |
| T049/T050 | `apps/client/tests/sidebar/latency.test.tsx` | 112 ms: one change re-rendered all 90 rows | `LocationRow` memoised on entity identity (`applyChanges` keeps unchanged references): 13-24 ms |
| T051 | `apps/client/tests/i18n/screens.test.tsx` | pseudo-locale refused by `setLocale` outside mock builds (test set state directly) | 0 untransformed strings on sidebar, Overview and sign-in; Polish plurals on cards |
| T052 | `apps/client/tests/platform/relative.test.ts` | `relativeParts`, `formatSince` missing | unit choice in `platform/intl.ts`, `formatSince` on the translator |
| T053 | - | - | user guide: "In the web client" in `docs/user-guide/overview.md` |
| M2 minors | `connection.test.ts`, `guard.test.ts` | stop during open blocked `retryNow`; `error: null` rejected; `/Sign-In` and `/sign%2Din` not treated as sign-in | `stop()` resets `opening`; `error != null`; `isSignInPath` (case, escapes, trailing slash) |

Refactor while green: `renderWithApi` returns `rerender`; the mock's own `apply` removed for `applyChanges`.
