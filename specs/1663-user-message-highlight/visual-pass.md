# Visual pass: highlighted user turns (#1663, FR-011, SC-001)

Date 2026-11-05. Component showcase (debug build of this worktree) on a private headless sway (pixman)
with lavapipe software Vulkan, virtual pointer, grim capture. Not a real display or GPU. Light, then dark.
Crops are in `visual-pass/`.

## Method notes
- `samples.rs` `transcript()` was temporarily extended with `UserSent` events (typed text, `/review the diff`,
  empty text, a long prompt, a trailing prompt after the recorded tool turn), then restored with `git checkout`.
- The dark run also needed a one-line scheme default change in `showcase/state.rs` (restored the same way),
  because the virtual pointer's clicks did not register on "Switch to dark". Neither file differs from HEAD.
- Clicks and drags never registered (scroll and hover did), so nothing needing a click could be posed.

## Verdicts

| # | Check | Light | Dark | Evidence |
|---|-------|-------|------|----------|
| 1 | `UserTurn` entry, one line and long wrapped | pass (wide, and wrapped at 520 px) | pass (wide only) | user-turn-light-entry.png, user-turn-dark-entry.png |
| 2 | Every user turn has tint and left accent, found at first look | pass | pass | user-turn-*-transcript.png, user-turn-*-transcript-end.png |
| 3 | Assistant text, tool cards (Read, Turn done), raw-event cards carry no tint or accent | pass | pass | user-turn-*-transcript-end.png |
| 4 | Text legible on the tint | pass | pass | same |
| 5 | Accent edge left corners follow the rounded surface | pass | pass | user-turn-*-transcript.png |
| 6 | Folded `/review the diff` card inside a turn reads distinct from the tint | pass (lighter raised card) | pass (darker card) | user-turn-*-transcript.png |
| 7 | Back-to-back user turns (four in a row) | pass, they do not touch | pass | user-turn-*-transcript.png |
| 8 | Empty-text prompt | pass: tinted block with "You" caption, empty body | pass | user-turn-*-transcript.png |
| 9 | Long prompt wrapped, narrow pane (520 px) | pass | pass | user-turn-*-narrow-transcript.png |
| 10 | Selection colour on the tint | NOT CHECKED | NOT CHECKED | drag did not register |
| 11 | Slash command unfolded | NOT CHECKED | NOT CHECKED | needs a click |
| 12 | Typed vs queued-delivered | not separately posed: same `UserSent` entry kind, one rendering | | |
| 13 | Sub-agent scope; thinking card | not checked: no user turn in the sub-agent sample, no thinking entry in the recorded tool turn | | |

## Findings
- Corners: the left edge is a 4dp bar with the 8dp surface radius on its left corners, which gives about 1 px of
  rounding over a 4 px width. The bar does not poke out past the tint, but the left edge reads nearly straight
  while the right corners show the full 8dp rounding. Acceptable as an accent bar.
- Back to back: turns are separated by the list gap (about 8dp) and read as separate blocks. No `spacing::XS`
  bottom padding needed.
- Each turn has a small "You" caption, which aids identification.
- Dark: the tint is low contrast against the page, but the lavender edge makes each turn identifiable at first look.

## Result
Checks 1 to 9 pass in both themes. Checks 10 and 11 are unverified and need a human or a working click.
