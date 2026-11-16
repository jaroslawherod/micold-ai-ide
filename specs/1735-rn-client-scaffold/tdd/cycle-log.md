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
