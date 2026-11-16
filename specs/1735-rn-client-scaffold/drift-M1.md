# Plan drift, M1

`plan-drift.sh` matches only paths that tasks.md spells out in full. M1's task lines name the
directories and the first files of each group, so the files below were created as part of the named
task. None changes scope.

## Unplanned paths

- .gitattributes, .gitignore: T001 workspace setup (lockfile marked generated, node artefacts ignored).
- apps/client/app/_layout.tsx, apps/client/app/index.tsx: T018 and T021 root layout and home route (Expo Router).
- apps/client/app/spike.tsx, apps/client/src/spike/SpikeScreen.tsx, apps/client/src/spike/fixtures.ts: T021 to T023 transcript spike and its fixtures.
- apps/client/eslint.config.js: T004 lint rules (R14).
- apps/client/jest.config.js: T005 Jest projects `web` and `components`.
- apps/client/src/components/LocalePicker.tsx: T020 LocalePicker.
- apps/client/src/i18n/IntlProvider.tsx, catalog.ts, localeStore.ts, pseudo.ts, translator.ts, useTitle.ts: T016 to T019 i18n core, split by concern.
- apps/client/src/platform/colorScheme.ts, fonts.ts, intl.ts: T014 and T015 platform fallbacks.
- apps/client/src/theme/buildTheme.ts, layout.ts, useColorScheme.ts: T009 to T013 theme from tokens.
- apps/client/tests/app/*, tests/components/LocalePicker.test.tsx, tests/i18n/*, tests/platform/degrade.test.ts, tests/theme/*: the test-first tasks paired with each implementation task above.
- apps/client/tests/support/fileStub.js, apps/client/tests/support/nativeSetup.js: T005 Jest support (asset stub; in-memory localStorage for the native-host project, see the ledger's Decisions).
- docs/development/ci-pipeline.md: T008 documents the new `client` job.
- packages/protocol/src/index.ts: T001 placeholder so the workspace member typechecks (real content in M2).
- scripts/guardrails-scope.conf: T008 `[web]` globs for diff coverage.
- scripts/tests/client-pins.test.sh: T007 keeps CI's node and pnpm pins equal to mise.toml (ledger Decisions).

## Planned but not touched

- scripts/diff-coverage.py: T008 names it as the tool CI calls; it already supports `--lang web`, so no edit was needed.
- scripts/tests/tokens-ts-drift.test.sh: planned for the tokens-to-TypeScript drift check; it is not part of M1's diff (the theme reads the committed tokens module directly). Tracked in the ledger's follow-ups.

## Remaining unplanned paths, listed one by one

- apps/client/src/i18n/catalog.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/i18n/localeStore.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/i18n/pseudo.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/i18n/translator.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/i18n/useTitle.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/platform/fonts.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/platform/intl.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/theme/layout.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/src/theme/useColorScheme.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/app/layout.test.tsx: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/app/spike.test.tsx: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/app/spikeFixtures.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/app/spikeScreen.test.tsx: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/components/LocalePicker.test.tsx: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/i18n/catalog.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/i18n/localeStore.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/i18n/pseudo.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/i18n/translator.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/i18n/useTitle.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/platform/degrade.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/theme/buildTheme.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/theme/fonts.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.
- apps/client/tests/theme/layout.test.ts: created by the M1 task of that area (theme, i18n, platform fallback, spike or app routes) and its paired test task.

## lcov path normalisation (T008)

Jest writes `SF:` paths relative to `apps/client` (`../../packages/protocol/...` once M2 adds the
sibling package). The CI step normalises each relative `SF:` path against `apps/client` with
`os.path.normpath`, giving repository paths for `scripts/diff-coverage.py --lang web`.
