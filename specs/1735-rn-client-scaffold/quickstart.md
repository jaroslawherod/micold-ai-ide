# Quickstart: #1735 client scaffold

## Part A: automated

1. `mise install` (node, pnpm), `pnpm install --frozen-lockfile`.
2. `mise run client-test` (lint, typecheck, Jest with coverage).
3. `mise run client-web` (static export to `apps/client/dist`), then `node scripts/client/bundle-check.mjs apps/client/dist`.
4. `mise run client-e2e` (Playwright, three browsers, mock build; writes screenshots to `apps/client/e2e/screenshots/`).
5. Dev loop: `pnpm --filter client start` with `EXPO_PUBLIC_API_MODE=mock`; sign in with the fixture identity.
6. Drift: `scripts/tests/tokens-ts-drift.test.sh` still passes.

## Part B: visual pass (skill `visual-pass`)

Compare the six screenshots (1280, 800, 390 px; light and dark) with the iced app's Overview and sidebar:
M3 colours and type, card arrangement (measured in R13), 600 px stacking, state glyph plus text, focus ring,
long-name truncation, pseudo-locale 40 percent growth without clipping the indicator. Record the result in
`specs/1735-rn-client-scaffold/visual-pass/` with the checks and verdict per screenshot.

## Spike (FR-020)

Run the spike route in the mock web export in Chromium, record render time, scroll smoothness,
memory, selection and copy and gaps in `spike.md` before starting sidebar work.
