# #1735: feat: React Native (Expo) web client scaffold, Material 3 theme from the tokens, sign-in, sidebar and overview

## Milestone M4 of 4: rn-client-scaffold (specs/1735-rn-client-scaffold)

**Deliverable**: Playwright smoke, pseudo-locale and screenshot specs against the mock export in Chromium, Firefox and WebKit in CI; terminal emulators denied by the bundle check; the visual pass recorded; developer docs for the web client.
**Verify**: `mise run client-e2e`; `node scripts/client/bundle-check.mjs apps/client/dist`; `scripts/tests/bundle-check.test.sh`
**Tasks**: T054–T062 (T060a deferred) · **Satisfies**: FR-018, FR-019, FR-021; SC-002, SC-003, SC-005–SC-007

- Playwright (`apps/client/e2e/`): sign-in, sidebar and Overview, an order-stable state change, locale switch and persistence, no token in web storage, keyboard path with visible focus, axe (WCAG 2 A/AA), 300 px and 200% zoom, reduced motion, pseudo-locale with no untransformed strings, screenshots at 1280/800/390 px light and dark.
- CI: new `web client e2e` job (`client-e2e`, 20 min, gated by `ci complete`), Playwright browsers cached under `~/.cache/ms-playwright` keyed on `pnpm-lock.yaml` (`actions/cache` v6.1.0, pinned). The `web client` job keeps its ten minutes (SC-005).
- `scripts/client/bundle-check.mjs`: also fails on xterm, `@xterm/*`, node-pty, hterm or term.js in the export (.js/.css/.html, word-bounded) or in `pnpm-lock.yaml`; self-test with near-miss cases.
- Working state glyph is now `●` (the earlier `◐` drew as a sliver for lack of font coverage), still beside its name (FR-011).
- Docs: `docs/development/client.md`.

## Deviations
- SC-002's 100 ms bound is asserted in Chromium only, the reference browser; Firefox and WebKit check row order.
- The keyboard test presses Option+Tab in WebKit on macOS only (Tab skips buttons there); CI on Linux uses Tab in all three browsers. Firefox and WebKit first run in CI (browser downloads are blocked in the authoring container).
- T060a deferred: side-by-side comparison with the iced app (no desktop session; the iced app is being sunset).

## Agent review
- Review A (opus), round 1 at 44515efaa: CHANGES (F1-F2 major, F3-F6 minor), all fixed in a300053.
- Review B (sonnet, conformance), round 1 at 44515efaa: CHANGES (F1-F2 major, F3 minor), all fixed in a300053.
- Round 2 (fix diff 44515efaa..a300053): pending (orchestrator).

## Local gate
Local gate: scoped only, full gate left to CI. By hand: Jest 179/179, eslint and tsc clean, Playwright Chromium 17/17 (installed Chrome), `scripts/tests/*.test.sh` all pass, bundle check clean on the production export.

Refs #1735

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvMY9kPSt8BLZNRgrtg
