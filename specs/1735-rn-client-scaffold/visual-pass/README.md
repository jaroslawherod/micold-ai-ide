# Visual pass, M4 (T060, quickstart Part B)

Screenshots: `apps/client/e2e/screenshots/` (the recorded set; a run writes to the gitignored `e2e/screenshots-out/`) from `e2e/screenshots.spec.ts` (Chromium, the mock export,
reduced motion so the Working pulse is still). Taken in the cloud container with an installed Chrome
(`PW_CHROMIUM_CHANNEL=chrome`), since the Playwright browser download was blocked; CI's Chromium
run re-takes them and uploads them as the `client-e2e` artifact.

**Limit of this pass:** the iced app was not launched (no cargo build in the cloud container), so the
side-by-side comparison against the iced Overview and sidebar is still to do. The checks below
compare against the spec (FR-011, FR-021, SC-003) and the M3 measurements in research R13.

| Screenshot | M3 colours and type | Arrangement | Glyph plus text | Truncation | Verdict |
|---|---|---|---|---|---|
| home-1280-light | ok: tonal surfaceVariant cards, Roboto | sidebar beside, 3 card columns | ok | no clipping | pass |
| home-1280-dark | ok: dark roles, readable contrast | as light | as light | ok | pass |
| home-800-light | ok | sidebar beside, 1 column (wide layout, R13 width) | as above | ok | pass |
| home-800-dark | ok | as light | as above | ok | pass |
| home-390-light (+ -overview) | ok | one column, sidebar stacked above the Overview | as above | ok | pass |
| home-390-dark (+ -overview) | ok | as light | as above | ok | pass |

Checked by the Playwright specs, not by eye: the 600 px stacking (screenshots spec asserts the panes'
geometry), the focus ring (smoke spec), pseudo-locale growth with no clipped indicator (pseudo spec).

## Findings

- **F1 (fixed):** the Working glyph `◐` rendered as a thin sliver in the container's Chrome (the
  font stack lacks it). It is now `●` (U+25CF), drawn full size, still beside the word Working
  (FR-011); the screenshots were re-taken with it. The iced app keeps `◐`.

## Verdict

Pass against the spec. The side-by-side comparison with the iced app is deferred to T060a (needs a desktop session; the iced app is being sunset).
