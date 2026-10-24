# Visual pass — feature 1299, close unit (quickstart §B)

Run 2026-10-24 by the close unit, following the `visual-pass` skill inline (no subagent in this
session), on Xvfb with the tiny-skia software renderer (`ICED_BACKEND=tiny-skia`), input via
xdotool, at origin/main `84ed10da` plus this PR's test-only changes. Client, daemon and showcase
built together and run from pinned copies, private data directory, fixture project P (Default,
`feat-1_alpha`, `feat-2_beta`, `fix-3_gamma`, `inc`, `misc`; no issue links). Screenshots:
[visual/](visual/).

**What this pass cannot show.** Under tiny-skia on Xvfb a frame took several seconds to reach the
screen, so no capture landed inside a 150 ms transition. Every check below is of an **end state**:
the fade and lift curves themselves (part-way at ~75 ms, a reversal continuing from its value,
perceived smoothness) rest on the automated tests listed in `tdd/verification.md`, not on this pass.

| Step | Result | Seen |
|---|---|---|
| B1 | not run | Tab through the showcase to the live pose: the page has hundreds of focus stops before it; the focus fade is covered on the client (B4) and by the tests |
| B2 | PASS (end states, light and dark) | Live pose *expand and collapse*: expanded with the menu shadow, *Collapse* leaves all three cards at rest with no shadow, *Expand* brings the shadow back; the expanded card's height is full in every frame captured ([light](visual/b2-seq.png), [dark](visual/b2-seq-dark.png)). Mid-collapse "not cut by the cards to its right or below": not capturable, rests on `a_settling_card_is_drawn_over_its_neighbours_and_nothing_moves` |
| B3 | PASS | The static *issue link, expanded* and *pull request rows* poses open already lifted ([shot](visual/b3-static-light.png)); the live pose too, on first sight |
| B4 | PASS, one observation | Right arrow from Default to `feat-1_alpha`: Default's ring gone, Alpha focused and expanded with its shadow, no other card moved ([shot](visual/b4-arrow-light.png)). Pointer rests on Beta, then straight onto Gamma: Gamma expanded with its shadow, Beta collapsed, no card moved ([shots](visual/b4-hover-light.png)). See the observation below |
| B5 | PASS (Escape half) | Escape on the focused, expanded Alpha: collapsed, no shadow, the ring stays ([shot](visual/b5-escape-light.png)). The issue-link half was not run: the fixture has no linked branch; `the_cards_link_is_not_faded` covers it |
| B6 | PASS | Shift+F10 on the focused Alpha: its menu opens, the card shows the target mark and no focus ring; Escape: the ring is back ([shots](visual/b6-menu-light.png)) |
| B7 | not run | A click activates the card and closes the Overview; the ripple is not capturable under software rendering (as in feature 972's pass). Covered by `a_press_on_a_faded_control_drops_its_indicator_at_once` |
| B8 | PASS (end states) | Settings → Appearance → Reduce motion on, Save (`settings.json`: `"reduce_motion": true`); reopened the Overview and repeated B4 and B5: Right arrow focuses and expands Alpha, Escape collapses it with the ring kept, hover Beta then Gamma expands each in turn ([shots](visual/b8-reduce-motion.png)). "Complete on the first frame" is not observable here; it rests on `reduce_motion_makes_card_focus_and_expansion_instant` and `the_overview_grid_follows_reduce_motion` |
| B9 | PASS (light and dark, at rest) | Focused, expanded and collapsed cards at rest look as in features 1121 and 1122: ring and tint on the focused card, menu shadow on the expanded one, flat collapsed cards, in both schemes (B2 and B4 shots). All five states at rest in the showcase's *five states* pose, light |

## Observation: a stale shadow outline after a hover move (tiny-skia only, unconfirmed cause)

With Reduce motion **off**, after the pointer rested on Beta and then moved straight onto Gamma,
the outline of Beta's expanded shadow stayed on screen below the collapsed Beta, twice in a row
([shot](visual/b4-hover-ghost.png)). A forced redraw (a one-pixel window resize) cleared it
([after](visual/b4-hover-after-forced-redraw.png)), so the application's state was right and the
last frame presented was not fully repainted. With Reduce motion **on** the same moves left nothing
behind. The tiny-skia renderer repaints only damaged regions, and in `iced_tiny_skia` 0.14.1 a
quad's damage is its bounds plus one pixel (`layer.rs`, `Layer::damage`), not its shadow, so a
shadow that eases away can leave pixels outside the repainted region. Whether this shows on the
default wgpu renderer, which redraws whole frames, was not checked: no GPU here. Recorded as a
follow-up in the ledger; not fixed in this PR.

Also seen, not this feature: the expanded live-pose card is drawn over the next section's heading in
the showcase (feature 1121's draw-over behaviour), and Settings' Reduce motion supporting text still
reads "Switch the Overview's full-screen zoom in one step", without the cards (the user guide,
FR-013, does name them).
