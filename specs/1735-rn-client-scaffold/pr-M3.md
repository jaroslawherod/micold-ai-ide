# #1735: feat: React Native (Expo) web client scaffold — M3 sidebar and Overview

## Milestone M3 of 4 — rn-client-scaffold (specs/1735-rn-client-scaffold)

**Deliverable**: After sign-in the sidebar and Overview list the fixture in stable order, update state live, mark offline daemons stale, and stack below 600 px; the user guide describes it.
**Verify**: `mise run client-test`; `mise run client-web` then the dev loop of quickstart Part A step 5
**Tasks**: T040–T053 · **Satisfies**: US2 scenarios 1–6; US3 scenarios 1–3; US5 scenario 4; FR-010–013, FR-019; SC-001–004; edge cases Viewport, Reduced motion

- `@micold/protocol`: pure `applyChanges` and `rowKeys` (stable order, reference-preserving); the mock uses them.
- Client: zustand snapshot store and `SnapshotProvider`; `Sidebar` grouped by daemon and project with loading, failed and empty states; `LocationRow` (one-line names, full name in the label, focus ring, selectable); `StateIndicator` (glyph and text, pulse off under reduced motion); `Overview` and `OverviewCard` with the iced column rule (280 min, 16 gap, at most 5), wide from 600 px, one stacked column below; stale marking for offline daemons and a dropped connection with "updated <relative time>".
- i18n: every new string in en and pl; pseudo-locale test finds 0 untransformed strings on sidebar, Overview and sign-in; Polish plurals on cards.
- User guide: "In the web client" in `docs/user-guide/overview.md`.
- M2 round-2 minors folded in: `ConnectionManager.stop()` resets `opening`; `error: null` is success; `isSignInPath` ignores case, percent-escapes and a trailing slash.

## Agent review
- Review A (code-review, high): pending (orchestrator)
- Review B (conformance, sonnet): pending (orchestrator)

## Local gate
Local gate: scoped only, full gate left to CI. By hand at 18bf049c9: Jest 173/173 (32 suites), eslint clean, tsc clean, diff coverage 98.6% (217/220), `web:export` plus `bundle-check.mjs` clean, `scripts/tests/*.test.sh` all pass. No visual pass yet (M4).

Refs #1735
https://claude.ai/code/session_01NCdYvMW4jW7WkvtvnpRVmD
