# Quickstart: Overview full-screen mode

## A — Automated

```bash
mise run gate      # in the cloud container: scripts/gate.sh
cargo test -p micold-client navigation_drawer card_grid overview
```

Expected: the gates in [contracts/ui.md](contracts/ui.md), the `features::overview` state tests, the
settings round-trip of `reduce_motion`, `tests/settings_sections.rs` and `tests/icons.rs` pass.

## B — Visual pass (visual-pass skill)

Prerequisite: a project with at least twelve locations, or the showcase's `CardGrid` and
`NavigationDrawer` entries. Window 1400 px wide, sidebar expanded at its default width.

1. Open the Overview; press *Full screen*: the sidebar slides out at the left edge while the cards
   widen and reflow; one motion, ≤ 500 ms; no frame with a gap between sidebar and Overview, no
   overlapping cards, a card changing row slides along the row ends (record frames).
2. The header button reads *Exit full screen*; its tooltip names Ctrl+Shift+F. Tab reaches it; Space
   toggles it. The top bar's Overview button, badge and project selector work.
3. Press Ctrl+Shift+F mid-zoom: the motion reverses from where it is.
4. In full screen: open Settings and close it (Overview returns full screen, no zoom); switch project
   (mode kept).
5. Leave by card, by Escape, by the top-bar button, by Ctrl+Shift+O: each time the sidebar returns
   with the zoom out as it was. Repeat with the sidebar collapsed, and expanded at 420 px.
6. Settings → Appearance → *Reduce motion* on, Save: entering and leaving switch in one frame; the
   sidebar's own hide/show still slides.
7. Scroll to card 20 of 30, enter full screen: card 20 is still visible after.
8. Showcase: drawer "retracted"/"retracting", grid "zoom" poses.

Record the pass in `specs/970-overview-fullscreen/visual-pass.md`.

## C — Platforms

CI runs the gate on Linux, macOS and Windows; the chord label shows Cmd on macOS.
