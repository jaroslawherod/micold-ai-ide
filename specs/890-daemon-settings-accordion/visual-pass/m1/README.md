# Visual pass — M1: showcase `AccordionSection` (quickstart Part B step 1)

Date 2026-10-18. Branch `claude/project-thread-rwlog8`. `micold-showcase` (dev build,
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0`, via `scripts/build-lock.sh`, pinned copy) on a private
Xvfb (:77) with `ICED_BACKEND=tiny-skia` (no Vulkan ICD in this container), driven with xdotool.
Not a real display or GPU. Animation and perceived smoothness were not judged.

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | Closed with a summary: muted one-line "Docker · Basic image" under "Network", inside the header | PASS | `light-poses.png` |
| 2 | Open: no summary | PASS | `light-poses.png` |
| 3 | Closed, required field empty: "Network*" | PASS | `light-poses.png` |
| 4 | Live entry toggles by click and by Space/Enter with focus | PASS | `light-live-click-space-enter.png` (click opens, Space closes, Enter opens, focus ring); same in dark |
| 5 | Narrow window: summary ends "…", chevron stays inside the header | PASS | `light-narrow-150px.png` (window 150 px wide: "Docker…") |
| 6 | All of the above in dark | PASS | `dark-poses.png`, `dark-live-click-space-enter.png` (click closes, Space opens, Enter closes), `dark-narrow-150px.png` |

## Notes (not failures)
- The summary reads grey and smaller than the title and the pose labels; judged by eye, not by contrast measurement.
- At 150 px the chevron on "Network*" touches the `*` (`light-narrow-required-zoom.png`); it stays inside the header.
- The static poses' titles render pale, the live one in the accent colour. That is presumably the no-`on_toggle` style; not a listed criterion.
- tiny-skia leaves stale-damage artefacts after a toggle; every capture was taken after a forced redraw (window resize nudge).
- The first click after a redraw with no prior pointer motion did not register; a pointer move first fixes it (xdotool artefact).
