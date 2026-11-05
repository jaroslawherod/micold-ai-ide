# Research: Highlighted user messages (#1663)

## R1: Fill token
- **Decision**: `surface_container_high` for the fill.
- **Rationale**: Transcript background is the window `background`; tool and command cards use `surface_container_low` plus a shadow. `surface_container_high` is two container levels above the cards (tone 92 vs 96 light, 17 vs 10 dark) and 6 (light) and 11 (dark) tones from the background, so the user turn is the strongest tonal block, and `on_surface` text is high-contrast on it. No new colour (FR-006). A unit test pins the difference and AA ratio in both schemes.
- **Alternatives**: `primary_container` / `secondary_container`: stronger, but ties text colour to `on_*_container` and every `Text` in the turn would need recolouring, which risks selection and caption colours (FR-008). `surface_container_low`: same as cards, so a user turn would look like a tool card (fails FR-001). A fixed tinted colour: forbidden by FR-006.

## R2: Accent edge
- **Decision**: 4dp `primary` bar on the left, as a `Row` of a fixed-width filled container and the content.
- **Rationale**: Follows the user's accent (FR-006); non-text edge, `primary` against the fill meets 3:1 in the token test (AA_NON_TEXT). No shadow, so depth is not used as a border (contract §1.5 discipline) and no platform dependency (Principle VI).
- **Alternatives**: a border on all sides (`prompt_card` style): reads as a card, not as a speaker mark. Chat-bubble alignment: out of scope per spec Assumptions.

## R3: Where the primitive lives
- **Decision**: new `UserTurn` builder in `micold-material`, styles in `style.rs` beside `queue_row`/`prompt_card`.
- **Rationale**: Principle VIII; the composer or a later view may reuse it.
- **Alternatives**: a private container in `entries.rs`: a one-off, rejected by Principle VIII.

## R4: Selection safety
- **Decision**: wrap the existing `column` unchanged; do not touch the `vec![..]` unit paths.
- **Rationale**: `ctx.text(..., path, len)` registers units by path under the entry; a parent container changes pixels only. The existing #1550 tests then prove FR-008.
- **Alternatives**: restructuring the column into the primitive: would risk path changes.

## R5: Rounds with no issue
No NEEDS CLARIFICATION remained. Back-to-back separation relies on current entry spacing and is verified in the visual pass.

## R1 addendum: measured contrast (T002)
`contrast()` of `surface_container_high` against: background 1.164 (light) / 1.285 (dark); `surface_container_low`, the fill of tool and command cards, 1.106 / 1.190. `on_surface` on the fill 14.05 / 11.17; `on_surface_variant` 7.67 / 8.44; `primary` edge 5.28 / 8.43 (floor 3.0); `on_surface` over the selection tint (`primary` at 0.3 over the fill) 9.26 / 5.48 (floor 4.5). Pinned in `micold-core/tests/tokens.rs` (floor for "apart": 1.05).

## R6: Highlight probe (T001)
- **Decision**: the renderer's own pixels. `transcript_entry_view.rs` draws the view through `micold_material::test_support::Mounted` and counts pixels exactly equal to `surface_container_high` (at least 1000; a user turn is 20k-40k, incidental anti-aliasing at most 23) and to `primary` (at least 40; the 4dp edge is 224+). No predicate fallback was needed.
- **Wrap sites**: `entries::entry` has the single `EntryKind::UserMessage` arm, used by the main rows, a delivered queued message (`StructuredEvent::UserSent`, same kind) and a sub-agent thread (`thread_body` calls `entry`). No other wrap site.
- **Test target**: the client's integration tests are modules of one target (`autotests = false`): run `cargo test -p micold-client --test main <filter>`, not `--test transcript_entry_view`.
