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
