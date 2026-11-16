# Spike: long transcript and diff on Paper and react-native-web (#1735, SC-008)

Recorded 2026-11-16 for FR-020 and US7. The spike page is `apps/client/app/spike.tsx`; it renders
fixed fixtures (`apps/client/src/spike/fixtures.ts`: 5,000 transcript entries, 2,000 diff lines)
with Paper `Text`, a `FlatList` (transcript) and a `ScrollView` (diff), and measures with
`performance.now` and `performance.memory`.

## Method

- Build: `EXPO_PUBLIC_API_MODE=mock expo export --platform web --output-dir dist-mock` (the route is
  absent from the production export; see *Production exclusion*).
- Browser: Google Chrome for Testing 153.0.8010.12, headless, 1000 x 800 viewport, driven through
  `playwright-core` from a script kept outside the repository, served as a static site from
  `dist-mock/`. The machine is a cloud VM (shared CPU); absolute numbers move with the host, the
  shape does not.
- Render time is the time from pressing the button to the list's first `onLayout` (first commit and
  layout of the surface), three cold page loads each. It is not a paint-to-pixels measure.
- Scroll: `requestAnimationFrame` loop adding 1,500 px per frame for 300 frames over the transcript
  (and 3,000 px per frame, up to 120 frames, over the diff); frame time is the gap between frames.
- Memory: `performance.memory.usedJSHeapSize` (Chromium only).

## Results

| Surface | Render time (3 cold loads) | Heap after first render | DOM nodes | Scroll (frame gap) |
|---|---|---|---|---|
| Transcript, 5,000 entries, `FlatList` (`initialNumToRender` 30) | 36.5, 29.9, 24.5 ms | 13.8 MB | 121 at rest, 754 after scrolling past about 2,000 entries (377 rows kept mounted) | median 16.7 ms (60 fps), p95 33.3 ms, max 33.5 ms over 299 frames; the viewport held 32 filled rows at the end, no blank area |
| Diff, 2,000 lines, `ScrollView`, Fira Mono, token diff colours | 191.1, 157.6, 174.5 ms | 39.5 MB | 2,040 (every line mounted) | median 16.7 ms, p95 16.7 ms, max 16.7 ms over 11 frames (34,000 px tall) |

Selection and copy: with the diff mounted, a select-all over the list selected all 2,000 lines
(99,779 characters, first line intact, `user-select: text`), so the browser's native selection and
copy work on Paper `Text` with `selectable`. The headless run read the selection string; it did not
exercise the system clipboard.

Fonts: Roboto and Fira Mono load from the export's own assets (no third-party fetch).

## Verdict

- **Transcript: pass with a follow-up.** A `FlatList` of Paper `Text` rows renders the first screen
  in about 30 ms and scrolls at 60 fps median on 5,000 entries. It never unmounts rows already
  rendered (377 stayed mounted), so the DOM grows with how far the person scrolls.
- **Diff: pass at 2,000 lines, not beyond.** Mounting every line costs 160 to 190 ms and 2,040 DOM
  nodes; scrolling is smooth. A diff of ten thousand lines would cost five times that, so the real
  diff view must virtualize.

## Gaps and where each goes

| Gap | Follow-up or decision |
|---|---|
| The `FlatList` keeps every row it has rendered mounted; recycling would cap the DOM. | #1736 (transcript) evaluates FlashList or a windowed list (research R12 candidates) when it builds the real transcript. Decision here: no virtualization library is added in #1735. |
| A native select-all over a virtualized transcript selects only the mounted rows, so copying a whole transcript needs an explicit "copy transcript" action. | Decision for #1736: provide a copy action; recorded here so the design starts from it. |
| The diff is a plain `ScrollView`; its cost is linear in lines. | The diff view issue of the #1722 plan (M6) virtualizes it; the 2,000-line figure above is its baseline. |
| Only Chromium was measured, and `performance.memory` exists only there. | Firefox and WebKit are covered by the M4 Playwright smoke test (T055), which does not measure timings; a cross-browser timing pass is a decision for #1736 when the real transcript exists. |
| Render time is first layout, not paint. | Accepted for a spike: scroll frame gaps cover the visible smoothness. #1736 measures paint with the real rows. |

## Production exclusion

`apps/client/app/spike.tsx` loads `src/spike/SpikeScreen.tsx` only when
`process.env.EXPO_PUBLIC_API_MODE === "mock"`. Checked on both exports of this tree: the production
export (`EXPO_PUBLIC_API_MODE=real`) holds no `__MICOLD_MOCK__` sentinel and its `/spike` route
chunk is a 268-byte redirect to `/`; the mock export holds the sentinel in the spike chunk. A Jest
test (`tests/app/spike.test.tsx`) pins the route's behaviour per mode; the bundle check of M2 (T039)
will scan the export for the sentinel.
