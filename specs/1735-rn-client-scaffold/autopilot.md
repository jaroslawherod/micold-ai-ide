# Autopilot ledger — #1735 rn-client-scaffold

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1735 — issue #1735 "feat: React Native (Expo) web client scaffold, Material 3 theme from the tokens, sign-in, sidebar and overview"; build against a mocked orchestrator API behind a TypeScript interface; i18n from the start; use #1950 tokens; plan 3-4 milestones. labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1735
- **Worktree branch**: claude/project-thread-9il8pd
- **Started**: 2026-11-16
- **Phase**: M4 implemented, review ready
- **Next step**: orchestrator runs Review A and B on M4 (origin/main...HEAD), then PR; T062 (SC-005, SC-009) is confirmed from that PR's CI run

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #2009 | Design: spec, plan, tasks | merged | f6cdbed6c06b5cefb34b4b5cac8a64b0af6f44ff |
| #2015 | M2 | merged | b863fd3393735e9d0ae6a4ac38299e31f30a5855 |
| (M3) | M3: body in `pr-M3.md` | not opened | - |
| #2018 | M3 | merged | dfd448cec7c5e1f317fc8092407381dc2ed11859 |
| #2013 | M1 workspace, CI, theme, i18n, spike | merged | 18aadedf61d2a5c3d512447f30e3a9f33eb12262 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T023 | full | themed app, client-test/client-web, CI job, i18n core, spike recorded | #2013 | merged |
| M2 | T024–T039 | full | protocol package, mock, reconnect, sign-in, mock-free prod export | #2015 | merged |
| M3 | T040–T053 | full | sidebar and Overview in stable order, stale marking, user guide | - | implemented |
| M4 | T054–T062 | full | Playwright 3 browsers, screenshots, visual pass, bundle hardening, docs | - | implemented (T054–T061), T062 waits on CI |

## Decisions

- M1: Jest runs two projects. `web` (`jest-expo/web`, jsdom) for logic touching `window`/`matchMedia`/`navigator`; `components` (`jest-expo`, native host) for Testing Library, whose renderer rejects the react-native-web DOM host. Component tests need no DOM, so `tests/support/nativeSetup.js` gives them an in-memory `localStorage`.
- M1: the `client` CI job installs through `corepack` + `actions/setup-node` (no `mise` action is pinned in the repo), so R7's "one source of truth" is kept by `scripts/tests/client-pins.test.sh` (node and pnpm majors vs `mise.toml`).
- M1: R14's lint rule is `no-restricted-syntax` selectors in `apps/client/eslint.config.js`, not a plugin (research R14 allows either). `react-native-a11y/has-accessibility-hint` is off: it demands a hint beside every label.
- M1: LocalePicker is a custom radio group on `Pressable`, not Paper `SegmentedButtons` (which forces `accessibilityRole="button"`).
- M1: the production export keeps a `/spike` route file that redirects home; the spike module is folded out by Metro (checked: no `__MICOLD_MOCK__` in `dist/`). A route file cannot be wholly absent in Expo Router without a build-time file switch.
- M1: Jest writes lcov paths relative to `apps/client`; the CI step rewrites `SF:` to repository paths before `scripts/diff-coverage.py --lang web`.
- M1: `client-e2e` exists as a task whose script fails with "arrives with M4 (T055)"; CI does not run it yet.

- M2: protocol tests run as a third Jest project (`protocol`, rootDir `packages/protocol`, babel-preset-expo) of `apps/client/jest.config.js`, so one lcov covers both packages. babel-plugin-istanbul instruments only files under the process cwd, so the client's `test` script runs Jest from the repository root; lcov paths are now repository paths and the ci.yml `SF:` rewrite of M1 is gone.
- M2: `select.ts` uses a guarded synchronous `require("@micold/protocol/mock")` (the spike's pattern), not a dynamic `import()`, which Metro would split into an async chunk that still ships. Checked: the `real` export passes `bundle-check.mjs`, the `mock` export fails it.
- M2: without a real client (`real` mode) `ApiProvider` renders the catalog message `api.unavailable` instead of crashing; the error text "real client not built (#1732)" stays the thrown error.
- M2: the route guard is `AuthGate` in the `(app)` group layout (`app/(app)/_layout.tsx`), returning `Redirect` before any protected route renders (review A F2); `app/index.tsx` moved into the group. Sign-in and the spike stay outside. `returnTo` is honoured only for a same-origin path that is not sign-in.
- M2: catalog area ids must be lowercase (M1 catalog test), so the sign-in messages are `signin.*`.
- M3: `applyChanges`/`rowKeys` live in `@micold/protocol` (pure, reference-preserving); the mock and the client store share them. `buildFixture` is exported from `@micold/protocol/mock` for tests.
- M3: SC-002 is met by memoising `LocationRow` on entity identity, not by a new subscription path.
- M3: "Sidebar session tree is deferred" read as no expand/collapse tree and no session selection; sessions are listed flat under their worktree (FR-010, US2 scenario 6). Worktree rows are selectable (local state) for focus and selection only; no route opens yet.
- M3: folded in M2 round-2 minors: `ConnectionManager.stop()` resets `opening`; `error: null` is success; `isSignInPath` ignores case, percent-escapes and a trailing slash.
- M3 (cloud): no mise, gh or Rust gate; JS checks run by hand, CI is the gate.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec.md | 7180aecdec89025cc8196b00a2f59e84ddce08e2 | e4eb498403faa61c433088be59dc8d505a145b5f:f6aa5b80601f9ce5ef93caddda0c35d1c1c1435e | CLEAN (3 minor F1-F3 applied in clarify) |
| Plan | 1 | plan.md, research.md, data-model.md, contracts/ | 7d38edbc4 | - | CHANGES (F1-F2 major, F3-F4 minor), all applied |
| Plan | 2 | plan.md, research.md, data-model.md, contracts/ | - | - | CLEAN with minors (sonnet re-review; both applied: `.gitattributes` entry, merged duplicate ci.yml entries) |
| Tasks | 1 | tasks.md | - | - | CHANGES (F1-F3 major, F4-F6 minor), all applied: bundle check moved to M2 (T039), test/impl split for fonts, platform fallbacks, LocalePicker, ConnectionBanner, bundle-check, edge-case tests (viewport, reduced motion, auth expiry, tab isolation), SC-005 moved to M4, dependencies added; tasks renumbered T001-T062 |
| Review A (opus) | 1 | M1 | dc3f3cd2b7fdc41559cc2aea0866aaea888545ba | - | CLEAN minors F1 (ci.yml client condition) and F2 (lcov SF paths) both fixed in 9b64585a458f441d7632f65b0f6a7c3532f1d467 |
| Review B (conformance) | 1 | M1 | dc3f3cd2b7fdc41559cc2aea0866aaea888545ba | - | CHANGES F1 major (plan drift, drift-M1.md written, plan-drift.sh clean), F2 minor (lcov normalisation noted); fixed in 9b64585a458f441d7632f65b0f6a7c3532f1d467 |
| Review A+B fix diff | 2 | M1 | 9b64585a458f441d7632f65b0f6a7c3532f1d467 | - | CLEAN (merged in #2013) |
| Review A (opus, code-review high) | 1 | M2 origin/main...HEAD | 19e4e86b3429e7b1e9122e499ac96909cdb57f2f | f49871e05eb829d67e1c0f7b77394b75d79d4ae4:19e4e86b3429e7b1e9122e499ac96909cdb57f2f | CHANGES (F1-F3 major, F4-F7 minor), all fixed in the M2 fix commit |
| Review B (sonnet, conformance) | 1 | M2 origin/main...HEAD | 19e4e86b3429e7b1e9122e499ac96909cdb57f2f | f49871e05eb829d67e1c0f7b77394b75d79d4ae4:19e4e86b3429e7b1e9122e499ac96909cdb57f2f | CHANGES (F1 major, F2-F5 minor), all fixed in the M2 fix commit (F5 documented in drift-M2.md) |
| Review A+B fix diff | 2 | M2 19e4e86b3..3f48434a4 | 3f48434a464c4534f048e3f5feea21b84711c7a1 | - | CLEAN (merged in #2015) |
| Review A (opus, code-review high) | 1 | M3 origin/main...HEAD | 18bf049c9e354945271b7fd277996bc089233a33 | b863fd3393735e9d0ae6a4ac38299e31f30a5855:18bf049c9e354945271b7fd277996bc089233a33 | CHANGES (F1-F2 major, F3-F6 minor), all fixed in acc4180b1 |
| Review B (sonnet, conformance) | 1 | M3 origin/main...HEAD | 18bf049c9e354945271b7fd277996bc089233a33 | b863fd3393735e9d0ae6a4ac38299e31f30a5855:18bf049c9e354945271b7fd277996bc089233a33 | CLEAN with minors F1-F3: F1 with A F6, F2 research R13 reworded, F3 stale tests shown to fail with `isStale` stubbed (cycle-log) |
| Review A+B fix diff | 2 | M3 b3028a1a9..acc4180b1 | acc4180b108d139c82e52966ce36e7ee221c86d1 | b3028a1a9:acc4180b108d139c82e52966ce36e7ee221c86d1 | CLEAN (sonnet re-review; minor groupBy O(n²) noted, not blocking) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M4 (prev #2018, dfd448cec7c5e1f317fc8092407381dc2ed11859) implemented on claude/project-thread-9il8pd: T054–T061 done, T062 open until the PR's CI run. Playwright ran locally only in Chromium via installed Chrome (17/17); Firefox/WebKit downloads are blocked in the cloud, so CI is their first run. Fixed for axe: LocalePicker `aria-checked`, focusable Overview ScrollView. Visual pass recorded with F1 open (Working glyph sliver) and the iced comparison outstanding. Next: reviews, then PR.

## Open escalation

None.

## Follow-ups not done

- M4: CI installs the Playwright browsers each run with no cache (no pinned `actions/cache` SHA reachable from the cloud session); add one if SC-005 gets tight.
- M4 visual pass: side-by-side with the iced app not done (no desktop/cargo build in the cloud); F1 Working glyph `◐` renders as a sliver in the container Chrome.

- M1 spike gaps (spike.md): FlatList never unmounts rendered rows (#1736 evaluates FlashList); select-all over a virtualized transcript needs a copy action (#1736); the diff is an unvirtualized ScrollView (diff view issue, M6 of #1722); only Chromium measured.
- T020 wording is for the web client only; the desktop `appearance-theming.md` sections are untouched.
- M2 local gate (also on 3f48434a4): `scripts/gate.sh` passed fmt and clippy, then stopped at `cargo nextest` (not installed in the cloud container; installing it was refused). No Rust changed; every `scripts/tests/*.test.sh` passes by hand, Jest 127/127, diff coverage 99.3%, lint and typechecks clean. The full gate is left to CI.
- #1732 and #1733 not built: the mock must be swapped for the real client and generated types (constitution V) in a follow-up once they land.
