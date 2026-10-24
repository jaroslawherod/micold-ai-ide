# Implementation Plan: Keyboard Map for Reviewing Changes

**Branch**: `claude/project-thread-m7gq7k` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1274-review-keyboard-map/spec.md`

## Summary

Ten single keys (`j k n p ] [ v g c ?`) and the send chord (Ctrl/Cmd+Shift+Enter) act on the shown
Review tab while it holds the keyboard; `?` opens a shortcut sheet; the jump-to field lists the
review's files and comments. Every rule a press decides lives in `micold-core` over plain inputs,
in a public key table `micold_core::review::keys::REVIEW_KEYS` that the client binds from, the sheet
renders and #1276's guide test reads, plus navigation rules that reuse #803's `navigation::step`.
That core is milestone **M1** and depends on no blocker. The client wiring, sheet, tooltips, docs,
layout snapshot and visual pass come in **M2** (after #1269, #1270, #1271, #1273) and the jump-to
entries in **M3** (after #1127) ([research R1](./research.md)). Ledger D4 (keys on, no setting) is
provisional; a setting would be one argument on `resolve` and one filter on the sheet (R4).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`keyboard::listen`, `KeyPressed { text, repeat, .. }`); no new
crate

**Storage**: none; positions are per-window view state of the Review tab (Principle IV)

**Testing**: `mise run test-core` (M1); `mise run gate` (M2, M3: client `tests/keymap.rs`,
`tests/review_keys.rs` new, `tests/quick_switch.rs`, `tests/overlay_registry.rs`, docs test, layout
snapshot)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: a press resolved well inside SC-003's 100 ms on a 200-file, 500-comment review
(core test bound 50 ms, generous for loaded CI)

**Constraints**: no daemon or wire change; no key reaches or leaves a focused terminal; core has no
OS branch (Principle VI); no stand-in for a blocker's work

**Scale/Scope**: M1: 3 new core modules (`review/keys.rs`, `review/navigate.rs`, `review/jump.rs`),
2 extended (`quick_switch.rs`, `review/diff.rs`). M2: `keymap.rs`, `features/changes.rs` (or #1269's
Review tab module), `ui/mod.rs` listener, `ui/material/text_area.rs`, a sheet surface, 3 docs. M3:
`features/quick_switch.rs`, the switcher's row view, 1 doc section.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Red first in core (M1) for every rule: table and `resolve` (holders, modifiers, AltGr, no text, repeat), file/hunk/comment steps with wrap and empty/none cases, pending edge, `has_hunks`, `next_unreviewed`, `comment_target`, `hunk_change_range`, `header_rows`, `filter` with `detail`, `review_candidates`, timing. Client decisions (holder gate, press → effect, select clears hunk, chord ≡ `SendPressed`, list position) in render-free modules with tests (M2/M3). Only view glue (sheet layout, hunk mark drawing) is covered by quickstart §B. |
| II. Multi-Session | PASS | Positions are per Review tab view; keys act on the shown entry only and read files, marks and comments at each press (FR-014); no session started or stopped beyond what **Send review** does. |
| III. Worktree Integration | PASS | Read-only over the entry's review; no worktree operation. |
| IV. Local-First | PASS | No persistence, no network; marks are stored by #1271. |
| V. Rust + iced | PASS | iced only; closed enums `ReviewAction`, `Holder`, `HunkMove`, `CommentOpen`, `Unreviewed`; a pending edge is `Option<(RelPath, Edge)>`, keyed to the file it was set for, so a diff that arrives for another file cannot apply it; `select_file` also clears it, covered by a test. |
| VI. Cross-Platform | PASS | Single keys by produced character on all three; the primary modifier decided only in `keymap::primary_held`; AltGr rule; quickstart §C; CI on three platforms. |
| VII. Documentation | PASS | `reviewing-changes.md`, `terminal-panes.md`, `worktrees-and-sessions.md` in the PR that ships each key (M2) and jump-to entry (M3); docs test tied to `REVIEW_KEYS` labels (SC-004). |
| VIII. Components | PASS | Sheet built from `ui/material` parts on the overlay registry (as `QuickSwitcherDialog`); `TextArea` gains a builder option `.on_press(..)` rather than a composer-local text area; hunk buttons (if added) are the shared icon button. |

Post-design re-check: unchanged, all PASS.

## Milestones (tasks unit cuts them in this order)

| ID | Needs merged | Delivers | User-visible |
|---|---|---|---|
| **M1 core** | nothing | D1–D3: `REVIEW_KEYS`, `resolve`, navigation rules, jump-to candidates and ranking, `header_rows`; unit tests | no (core only; the client gains `detail: None` in existing `Candidate` literals, no behaviour change) |
| **M2 keys** | #1269, #1270, #1271, #1273 | D4–D7: press building, listener and holder, Review tab reducer, composer chord, current-hunk mark, sheet, overflow item, tooltips, docs, layout snapshot, showcase, visual pass | US1–US5 |
| **M3 jump-to** | #1127 (and M2) | D8: review entries in the quick-switcher, choosing, docs section, visual pass rows B14 | US6 |

If #1127 merges before M2 starts, M2 and M3 may ship as one PR; M1 never waits.

## Design

### D1 — Key table and resolution (core, M1) — R2, R3, R4, R7

New `crates/micold-core/src/review/keys.rs` (in `review/mod.rs`): `ReviewAction`, `KeyName`,
`KeyMods`, `ReviewKey`, `pub const REVIEW_KEYS`, `Press`, `PressKey`, `Holder`, `resolve`,
`ReviewKey::labels` ([contracts/keys.md](./contracts/keys.md) K1–K4). Public and stable for #1276.

### D2 — Navigation rules (core, M1) — R5, R9

New `crates/micold-core/src/review/navigate.rs`: `NavFile`, `has_hunks`, `step_file`, `SelectedDiff`,
`Edge`, `HunkMove`, `step_hunk`, `resolve_edge`, `CommentFact`, `comment_order`, `CommentMove`,
`Show`, `step_comment`, `Unreviewed`, `next_unreviewed`, `hunk_change_range`, `CommentOpen`,
`comment_target`, the two note strings. Reuses `navigation::step`. `review/diff.rs` gains
`header_rows(shaped, layout)` using `settings::DiffLayout` and the existing unified / side-by-side
row builders.

### D3 — Jump-to rules (core, M1) — R6

`crates/micold-core/src/quick_switch.rs`: `EntryKey::ReviewFile`, `EntryKey::ReviewComment`,
`Candidate.detail`, `Via`, `Ranked.via`, `filter` sorting `(via, rank, index)`. New
`crates/micold-core/src/review/jump.rs::review_candidates`
([contracts/jump-to.md](./contracts/jump-to.md) J1–J3). Existing literals in
`crates/micold-client/src/features/quick_switch.rs::candidates` gain `detail: None`.

### D4 — Presses (client, M2) — R2, R7

`crates/micold-client/src/keymap.rs`: `primary_held`, `review_press` (K5); no key match.
`ui/mod.rs`: a `review_key_listener` subscribed while the holder is `Review` or `Sheet`, mapping
`KeyPressed` → `Press` → `resolve` → one Review-tab message. The holder function is render-free
beside the Review tab's state (#1269's module, else `features/changes.rs`), reading
`window.focused_field`, `overlay::registry::topmost`, quick-switcher, Settings, Overview and
terminal focus. `ui/material/text_area.rs`: `.on_press(..)`, and Ctrl/Cmd+Enter submit excludes
Shift (today it saves on Ctrl+Shift+Enter); the composer in `ui/changes.rs` sets
`.on_press` to `resolve(_, Holder::Composer)` → `SendPressed`.

### D5 — Review tab reducer (client, M2) — R5, R9

The Review tab view (#1269) gains `current_hunk`, `pending_edge`, `last_comment`. One `select_file`
path (tree click, `j`/`k`, `g`, `n`/`p` crossing, `]`/`[`, jump-to) clears the hunk and the pending
edge, brings the row into view and expands a collapsed directory (#1273's tree). Effects per
action as K7. Hunk scroll reuses `jump_offset`'s arithmetic and the pending-jump flag; `v` sends
#1271's **Mark reviewed** message; `Send` sends `Msg::SendPressed`.

### D6 — Current hunk mark and hunk controls (client, M2) — R9

`ui/changes.rs` (or #1270's diff header): the current hunk's header row drawn with a theme-role
accent; previous / next hunk icon buttons in the diff header if #1270/#1273 did not add them, sending
the `n`/`p` messages. Tooltips on those, **Mark reviewed** and **Send review (n)** read
`ReviewKey::labels` (FR-013).

### D7 — Shortcut sheet (client, M2) — R8

`ShortcutSheet` (`floating_surface!` on `Layer::Dialog`, `Registered`) in the Review tab feature;
one `REGISTERED` line in `overlay/registry.rs`; view `ui/shortcut_sheet.rs` renders `REVIEW_KEYS` in
order with the platform's labels from `ui/material` parts. Overflow menu (#1270) item **Keyboard
shortcuts** sends the same open message. Showcase `sections/review.rs` gains the sheet.

### D8 — Jump-to entries (client, M3) — R6

`crates/micold-client/src/features/quick_switch.rs`: `candidates` inserts `review_candidates` for
the current entry's Review tab after the active project's locations and sessions; the U3 choice
resolution gains the two keys, returning a Review-tab step (show tab, `select_file` or the comment
effect). The row view shows the reviewed state, or status and `path:line`.

### D9 — Docs, snapshot, visual pass (M2; M3 for jump-to)

`docs/user-guide/reviewing-changes.md` new section *Reviewing from the keyboard* (table of
`REVIEW_KEYS`, sheet, jump-to entries in M3); send chord row in
`docs/user-guide/terminal-panes.md#keyboard-shortcuts` and bullet in
`docs/user-guide/worktrees-and-sessions.md#interacting-with-the-terminal`. Layout snapshot states
`review-current-hunk` and `review-shortcut-sheet` in `tests/support/covered_states.rs`, listed in
`docs/development/layout-snapshot.md`. Visual pass at 1280×800, both themes (quickstart §B).

## Requirement map

| FR / SC | Where | Research |
|---|---|---|
| FR-001 | D1 table + `resolve`; D4 listener | R2, R3, R4 |
| FR-002, SC-002 | D1 `Holder::Other`; D4 holder gate, captured presses | R7 |
| FR-003 | D2 `step_file`; D5 `select_file` | R5 |
| FR-004 | D2 `step_hunk`, `resolve_edge`, `has_hunks`; D5 pending edge | R5, R9 |
| FR-005 | D6 mark | R9 |
| FR-006 | D2 `comment_order`, `step_comment`; D5 card jump / drawer | R5 |
| FR-007 | D5 → #1271 control message | R1 |
| FR-008 | D2 `next_unreviewed`; D5 note | R5 |
| FR-009 | D2 `comment_target`, `hunk_change_range`; D5 | R5 |
| FR-010, SC-006 | D1 `Send` entry; D4 listener + composer `.on_press`; `SendPressed` | R2, R7 |
| FR-011 | D4 conflict test in `tests/keymap.rs` | R2 |
| FR-012 | D1 `Holder::Sheet`; D7 | R8 |
| FR-013 | D6 tooltips from `labels` | R3 |
| FR-014 | D5 per-tab state, shown entry only | R5 |
| FR-015 | D1 modifiers, no text, repeat flag; D4 `review_press` | R2 |
| FR-016, FR-017 | D3; D8 | R6 |
| FR-018 | D8 choice → Review-tab step only | R6 |
| FR-019 | D3 `detail: None` unchanged; D8 no tab → none | R6 |
| FR-020, SC-004 | D9 + docs test over `REVIEW_KEYS` | R3 |
| FR-021 | D9 snapshot states, visual pass | R10 |
| SC-001 | `j`/`g` + `v` per file + chord; quickstart B1, B5, B6, B10 | R5 |
| SC-003 | D2 timing test; quickstart | R10 |
| SC-005 | D3 ranking; quickstart B14 | R6 |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Core unit (M1) | `review/keys.rs` tests | every table row resolves under `Review`; `Other` nothing; `Sheet` only `?`; `Composer` only `Send`; Ctrl/Alt/Cmd alone refuse a single key, Ctrl+Alt (AltGr) allows; Shift ignored for chars, `J` unbound; no text → none; repeat only for steppers; chord needs primary+shift, not Alt; labels both forms; table has 12 unique rows | 001, 002, 010, 012, 015 |
| Core unit (M1) | `review/navigate.rs` tests | spec scenarios US1 s1–s5, US2 s3–s5, US3 s1–s6 and edge cases (empty review, nothing selected, no comments, file without hunks, diff loading/failed, selection no longer listed, only-unreviewed is selected, unlisted file's comment → drawer); 200 files / 500 comments under 50 ms (generous, not flaky on loaded CI) | 003, 004, 006, 008, 009, SC-003 |
| Core unit (M1) | `review/diff.rs`, `quick_switch.rs`, `review/jump.rs` tests | `header_rows` per layout with gaps; name before detail; detail spans; `detail: None` keeps 968 order (existing tests untouched); candidate texts and order | 004, 016, 017, 019 |
| Client keys (M2) | `tests/keymap.rs` | `primary_held` per platform; `review_press` from text/Enter/dead key; send chord conflicts (803 FR-006 lists, app chords); docs list every `REVIEW_KEYS` label | 011, 015, 020, SC-004 |
| Client state (M2) | `tests/review_keys.rs` (new) | holder per surface/field/Settings/Overview/terminal; each action's effect on a Review tab state; `select_file` clears hunk + edge; pending edge resolved on `DiffRead`; chord ≡ `SendPressed` effect; composer text kept; hidden tab of another entry untouched | 001–015, SC-006 |
| Client state (M2) | `tests/features_composer.rs` | Ctrl/Cmd+Enter saves, with Shift it does not; `.on_press` message | 010 |
| Registry (M2) | `tests/overlay_registry.rs` | `ShortcutSheet` identity, Dialog layer, Escape / outside click | 012 |
| Client state (M3) | `tests/quick_switch.rs` | entries present only with a Review tab, position, choice → step, no session change | 016–019 |
| Snapshot (M2) | `tests/support/covered_states.rs` + gates | `review-current-hunk`, `review-shortcut-sheet` | 021 |
| Visual pass | quickstart §B (visual-pass skill), 1280×800, both themes | B1–B15 | all |
| Platforms | CI on three; quickstart §C | primary modifier, AltGr layouts, terminal untouched | VI, 015 |

## Project Structure

### Documentation (this feature)

```text
specs/1274-review-keyboard-map/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── keys.md
│   └── jump-to.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── review/{keys.rs (new), navigate.rs (new), jump.rs (new), diff.rs, mod.rs}   # M1
└── quick_switch.rs                                                             # M1
crates/micold-client/src/
├── keymap.rs                                   # M2: primary_held, review_press
├── features/{changes.rs or #1269's review tab module, quick_switch.rs (M1 literals; M3)}
├── overlay/registry.rs                         # M2: ShortcutSheet
├── ui/{mod.rs, changes.rs, shortcut_sheet.rs (new)}, ui/material/text_area.rs   # M2
└── showcase/sections/review.rs                 # M2
crates/micold-client/tests/{keymap.rs, review_keys.rs (new), quick_switch.rs, overlay_registry.rs, support/covered_states.rs}
docs/user-guide/{reviewing-changes.md, terminal-panes.md, worktrees-and-sessions.md}
docs/development/layout-snapshot.md
```

**Structure Decision**: existing workspace layout. Rules and the key table in `micold-core`
(blocker-free, M1); press building in the client's render-free `keymap.rs`; state and effects in the
Review tab's render-free feature module; the sheet on the overlay registry; drawing from the shared
`ui/material` library.

## Risks

- **Blocker shapes**: M2 names #1269's view module and #1270/#1271/#1273 controls by role, not by
  type; the tasks unit for M2 re-reads them once merged. M1 is insulated by plain inputs.
- **AltGr on Linux/macOS**: Linux reports AltGr as neither Ctrl nor Alt; macOS Option produces
  characters but is Alt, so Option+`[` is refused there (layouts that need Option for `[` lose
  `[`/`]` on macOS). Checked in quickstart §C; a follow-up if a user reports it.

## Complexity Tracking

No violations.
