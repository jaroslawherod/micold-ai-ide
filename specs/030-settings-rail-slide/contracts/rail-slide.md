# Contract: The section rail's slide

Owns what `SectionList` promises, frame by frame, while it moves between its labelled and
icons-only states, and the one change it makes to `NavigationDrawer`. Every clause names the
requirement it serves and the test that holds it (quickstart §A.2).

This is a UI contract: the API surface is unchanged, so what is contracted is geometry, timing,
reach and input. Units are logical pixels (dp). `p` is the rail's eased progress, 1 labelled and
0 icons only; `f` is a row's fraction (data-model), equal to `p` for every row of one rail.

---

## 1. API

Unchanged. `SectionList::new(sections, roles)`, `.selected(i)`, `.badge_accent(fill, on_fill)`,
`.collapsed(bool)`, `.toggle(message)`, then `.into()` an `Element`. No caller opts in to the slide
and none can opt out: every screen built on the component slides (Principle VIII).

## 2. Time

| Clause | Value | Serves |
|---|---|---|
| Duration | `motion::duration::MEDIUM_4` (400 ms) | FR-003 |
| Curve | `motion::EMPHASIZED`, cubic-bezier(0.2, 0, 0, 1) | FR-003 |
| Longest single step | as `Progress`: the first step of a transition ≤ 16 ms, any step ≤ 64 ms | SC-002, spec Edge Cases |
| Settles | within 400 ms of the latest press + 2 × the longest frame gap, while gaps ≤ 64 ms | SC-002 |
| Reversal | retargets from the value reached; no step larger than an uninterrupted slide makes over the same interval | FR-004, SC-004 |
| At rest | no redraw or relayout request | FR-011 |
| Mount | at the final state, no slide | FR-005, SC-006 |
| State flip | on the press, before any frame | FR-010 |

**`NavigationDrawer`** takes the same duration and curve. Driven with the same frame instants from
the same press, the drawer's panel and the rail are at the same fraction of their width change at
every sample, excluding the drawer's swap to its strip once fully closed (SC-002).

## 3. Geometry

| Clause | Rule | Serves |
|---|---|---|
| Rail width | `80 + 208 · p`; strictly between 80 and 288 on at least one frame each way | FR-001, SC-001 |
| Section region | begins at the rail's right edge; its content's offset from that edge is its rest offset | FR-002 |
| At rest | every visible rectangle of the settings surface equals `origin/main`'s; the showcase rail example looks as it does on `origin/main` | FR-006, SC-005 |
| Row height | every row with an icon, and the collapse control, keeps its rest height on every frame | FR-013, SC-007 |
| No reflow | a row with an icon is only ever laid out at 272 or 64; no label of such a row wraps | FR-013 |
| No-icon row | laid out at the current width; its chip stays inside the row; its label may wrap and its height follows, moving the rows below it vertically; the column is never taller than at the taller of its two rest states (research R3a) | FR-013 exemption, SC-007 (binds rows with an icon), FR-015, spec Edge Cases |
| Icon x | `x_icons + (x_labelled − x_icons) · f`, in whichever form is drawn (today: `x_icons` 33, `x_labelled` 20 or 32 for the current row) | FR-014 |
| Icon step | between consecutive frames with no selection change: `≤ |x_labelled − x_icons| · |Δf| + 0.5`, and toward the target's rest x | SC-007 |
| Selection mid-slide | the rows whose selection changed move their icon at once by `(x_labelled′ − x_labelled) · f` — 12 expanded, 0 collapsed | FR-014 exception |
| Drawing | nothing drawn outside the row's bounds, hence nothing outside the rail's | FR-009 |

## 4. Forms

| Row | Forms (rest width) | Drawn at `f` |
|---|---|---|
| Destination with icon, no badge | Labelled (272), IconsOnly (64) | IconsOnly if `f ≤ 0.001`, else Labelled |
| Destination with icon and badge | Labelled (272), Marked (272), IconsOnly (64) | IconsOnly if `f ≤ 0.001`; Labelled if `f ≥ 0.999`; else Marked |
| Collapse control | Labelled: HideSidebar + "Collapse" (272); IconsOnly: ShowSidebar (64) | as a destination with no badge |
| Destination with no icon | Single (current width) | Single |

- *Marked* is *Labelled* with the icon tinted with the badge fill, as *IconsOnly* tints it.
- A badged row therefore shows its tinted icon on every frame strictly between the rest states, and
  its chip or tinted icon at rest (FR-015).
- A form not drawn is parked: laid out, positioned out of reach, not drawn.

## 5. Reach and input

| Clause | Rule | Serves |
|---|---|---|
| Keyboard reach | operations visit only drawn forms; each drawn control is reached exactly once; the reachable count equals the rest count; the section's order is unchanged | FR-008, SC-003 |
| Focus | a focused rail control keeps focus on every frame, including the frame its row changes form; focus lands on the new form's same-numbered focusable, and whether its indicator is shown carries over with it (pointer focus stays unshown) | FR-007, SC-003, FR-006 |
| Focus exceptions | the user moving focus, or changing section, mid-slide | FR-007 |
| Pointer | hover and press reach only the drawn form, and only while the cursor is inside the row's bounds; parked forms receive only the frame's redraw and the release/lift/lost/cursor-left events, with the cursor unavailable; their messages and captures are dropped, their redraw requests and invalidations kept, so no press is left held and no ripple frozen in a form that parked | FR-009, FR-011 |
| Overlays | forwarded from the drawn form only; none constructed | `one_overlay_implementation.rs` |

## 6. Not contracted

- The worktree sidebar's strip swap once fully closed (unchanged).
- Animating the selection marker's move, and reduced-motion preferences (spec Assumptions).
- Frame pacing and the look of the clip, which the recorded visual pass covers (quickstart §B).
