# Implementation Plan: Review UI — Stale-Awareness and Quick Comment

**Branch**: `claude/project-thread-1l3w9s` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1275-review-stale-quick-comment/spec.md` (issue #1275,
slice R6 of `docs/development/review-redesign.md`). FR-014 is planned as the whole-view hold, as the
spec records it: provisional, awaiting the user (ledger *Open escalation*).

## Summary

Three behaviours, each split into a pure core half that can be built and merged now and a client
half that draws it.

- **Stale-awareness.** Every row of the change list gains a `ChangeIdentity` (FR-001a): git object
  ids from the `--raw` read the list already makes, one `git hash-object --stdin-paths` for
  working-tree versions, size and modification time over the size limits, a patch hash for a pull
  request. A new pure module `review::stale` compares the shown list with the latest one
  (`changed_since`) and holds the latest read beside the shown one (`Held<L>`) until it is applied
  or one file is adopted by selection. The client's Changes view stops replacing its list and diff
  on a watch read: it keeps the shown snapshot, shows *n files changed since you looked* in the
  header and a dot on each changed row, and applies on a press.
- **Quick comment.** A pure `review::drag::Drag` turns a press, the rows passed and the release into
  a click or a range (FR-015–FR-018). The diff's gutter widget reports press, pass and release; the
  reducer opens or moves the composer and restores the previous pick on **Esc**.
- **Suggestion chips.** A pure `review::chips` holds the three chips and the insertion rule; a
  client-owned `Settings::suggestion_chips` (off by default) switches them on; the composer shows
  them as `ToggleChip`s that join the keyboard order.

Core lands first (M1, M2) on today's code with unit tests; they build now. The client milestones
that touch the Changes/Review view (M3 hold and notice, M4 drag to comment, M5 chips UI) wait until
#1269 (Review tab) and #1271 are merged, since #1269 reshapes that view; FR-013 (M6) also needs
#1271.

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise` (workspace MSRV)

**Primary Dependencies**: iced 0.14 (existing); no new crate. The patch hash uses
`std::collections::hash_map::DefaultHasher`, as `core::store` does: in memory only, never stored.

**Storage**: no new file. `suggestion_chips` is one more key of the local `settings.json`
(`#[serde(default)]`, false); "since you looked" is per window, in memory (spec Assumptions).

**Testing**: `mise run test-core` (core units, `crates/micold-core/tests/review_git.rs` against
real git), `scripts/gate.sh` (client render-free tests in `crates/micold-client/tests/features_changes.rs`
and the reducer's own tests, settings tests, geometry gates, layout snapshot, docs checks),
quickstart §B visual pass for the drawing glue.

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (workspace: `micold-core`, `micold-daemon`, `micold-client`)

**Performance Goals**: the notice within 2 s of a settled burst (SC-002), so the list read with
identities stays inside today's budget: one extra `git hash-object --stdin-paths` process per read,
over only the working-tree versions `--raw` reports without an id, never a per-file diff (spec
Assumptions). A drag repaints the pick per row passed with no read.

**Constraints**: nothing on screen moves while held (FR-005, FR-014, SC-001); offline and local;
no protocol change (the list, diffs, holds and drags are all client-side; the daemon is untouched).

**Scale/Scope**: one entry's change list (482's limits: rows in the hundreds, files ≤ 2 MiB for
hashing, larger ones by size and time), one pull request's diff (≤ 300 files, GitHub's cap).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
all PASS, no deviation.

- [x] **I. Test-First**: every decision is in tested render-free code written test-first: core
  `review::changes` (identity parsing, assembly), `review::git` (identity fill against real git in
  `tests/review_git.rs`), `review::stale`, `review::drag`, `review::chips`, `settings` (default and
  round trip); client `features::changes` reducer (hold, apply, adopt, user reads, drag, chips) in
  `tests/features_changes.rs`; `features::settings` for the switch. Only `ui/changes.rs` and the
  gutter's event glue in `ui/material/diff_view.rs` use the §B pass, and the gutter keeps an
  in-crate test of the messages it publishes.
- [x] **II. Multi-Session**: the hold is per window and per open view (Edge *Two windows*); no
  session state changes; changes from any session count the same (Edge *Several sessions*); a
  change in one entry never reaches another entry's view, because the hold lives in that entry's
  `OpenView` (and in its tab once #1269 keys the views by entry).
- [x] **III. Worktree Integration**: every read runs in the entry's own directory, as today; no
  worktree is created or changed; `hash-object` writes nothing (no `-w`).
- [x] **IV. Local-First**: local git only; the pull request hold compares diffs the view already
  read; nothing new leaves the device.
- [x] **V. Rust + iced**: Rust and iced only. `Held<L>` exists only with a non-empty changed set
  (`observe` returns `None` otherwise), so "held with nothing changed" is unrepresentable; a drag's
  side is fixed at the press by `Drag`'s constructor.
- [x] **VI. Cross-Platform**: core is platform-agnostic; mtime is read through `std::fs::Metadata`
  on every platform; paths stay `RelPath`; the drag uses iced's pointer events, which carry mouse and
  touchpad alike on all three; the save shortcut is the existing one. CI runs the core and client
  tests on all three.
- [x] **VII. Documentation**: each client milestone updates `docs/user-guide/reviewing-changes.md`
  (*Keeping up with changes*, *Picking lines*, *Writing a comment*) or `docs/user-guide/settings.md`
  in the same PR, with the layout snapshot (FR-029).
- [x] **VIII. Reusable UI**: the notice is the shared `Button` (text variant) with a count label;
  the row dot is a new `ChangedDot` primitive in `ui/material/` with a builder, also usable by #1273's
  tree rows; chips reuse `ToggleChip`, extended with keyboard focus through `TakesTheKeyboard` (as
  `AttachmentChip` does). The gutter drag extends `DiffView`'s builder (`.on_gutter_drag(..)`), not
  a new diff widget. New and extended primitives appear in the showcase.

## Project Structure

### Documentation (this feature)

```text
specs/1275-review-stale-quick-comment/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/  core-api.md  changes-view-hold.md  quick-comment.md
└── tasks.md   (tasks unit)
```

### Source Code (repository root)

```text
crates/micold-core/src/review/
├── changes.rs     ChangeIdentity, VersionId; NameStatus ids from --raw; assemble fills them;
│                  ChangedFile.identity (#[serde(default)])
├── git.rs         identities for working-tree versions (hash-object --stdin-paths, size+mtime over
│                  the limits); GitCli::change_list_of (pathspec-limited list, for a diff-time row)
├── pr_diff.rs     split_pr_diff sets ChangeIdentity::Patch(hash of the file's section)
├── stale.rs       NEW changed_since, Listing, Held<L>, observe
├── drag.rs        NEW gutter_line, Drag, Release
├── chips.rs       NEW SuggestionChip, CHIPS, insert
└── mod.rs         pub mod stale, drag, chips
crates/micold-core/src/settings.rs          Settings::suggestion_chips (default false)
crates/micold-core/src/{context_header.rs,runs/summary.rs}  struct literals gain identity (tests)
crates/micold-core/tests/review_git.rs      identity against real git
crates/micold-client/src/features/changes.rs   OpenView.held, PullRequestView.held, apply_next,
                                               drag; Msg::{StaleApplied, GutterDrag*, ChipPressed};
                                               DiffRead carries the diff-time row
crates/micold-client/src/shell/changes.rs      ReadDiff also runs change_list_of; drag release and
                                               Esc listener while a drag is on
crates/micold-client/src/features/settings.rs  suggestion_chips on the draft and the applied state
crates/micold-client/src/ui/changes.rs         notice in the header, dot on file_row, chips under
                                               the composer, gutter drag wiring (glue)
crates/micold-client/src/ui/material/{diff_view.rs,toggle_chip.rs,changed_dot.rs (new),mod.rs}
crates/micold-client/src/ui/settings/appearance.rs   the switch
crates/micold-client/src/showcase/sections/review.rs  dot, focusable chip
crates/micold-client/tests/{features_changes.rs,layout_snapshot.rs,fixtures/layout_snapshot.txt}
docs/user-guide/{reviewing-changes.md,settings.md}
```

**Structure Decision**: the existing three-crate split. Every decision in `micold-core`
(`review::{changes,git,pr_diff,stale,drag,chips}`, `settings`), render-free view state in
`micold-client/src/features/`, drawing in `ui/`. The daemon and the protocol are not touched.

## What can be built now, and what waits for the redesign

| Part | Layer | Depends on | Build now? |
|---|---|---|---|
| Change identity in the list read (FR-001a) | core `changes`, `git`, `pr_diff` | nothing | yes (M1) |
| Stale comparison and held snapshot (FR-001–FR-004) | core `stale` | M1 | yes (M1) |
| Drag pick, chip insertion, setting default (FR-015–FR-018 decisions, FR-022, FR-024, FR-028) | core `drag`, `chips`, `settings` | nothing | yes (M2) |
| Hold, notice, dot, selection under hold, PR hold (FR-005–FR-012, FR-014) | client `features::changes`, `ui/changes.rs` | M1, #1269, #1271 merged | no: after #1269 and #1271 (M3) |
| Drag gesture and composer opening (FR-015–FR-021) | client reducer, `DiffView` gutter | M2, #1269, #1271 merged | no: after #1269 and #1271 (M4) |
| Chips in the composer and the Settings switch (FR-022–FR-027) | client reducer, settings, `ui` | M2, #1269, #1271 merged | no: after #1269 and #1271 (M5) |
| Reviewed file returns to unseen while held (FR-013) | client, #1271's reviewed set | #1271 merged | no (M6) |

Client parts that #1269–#1273 reshape:

- **#1269 (Review tab)** turns `State.open` into a map keyed by entry. The hold fields live on
  `OpenView` (and `PullRequestView`), so they move with the view unchanged; the merge cost is
  textual (whichever lands second rebases the reducer). #1269's re-read when a hidden tab is shown
  goes through the watch path, so it is held like any other watch read (research R9). The notice
  moves from the Changes view header to the Review tab header with the header itself.
- **#1270 (review bar, drawer)** moves *Send review* and the counts to a bottom bar; the notice
  stays in the view header (spec Assumptions), the composer stays inline, so chips are unaffected.
- **#1272 / #1273 (source dropdown, file tree)** replace `file_row` with tree rows. The dot is the
  shared `ChangedDot`, so the tree row places the same primitive; `features::changes` exposes
  `changed_since_you_looked(view, path) -> bool`, which the tree row reads instead of the flat row.
- **#1271 (reviewed marks)**: FR-013 only (M6). Nothing else here reads reviewed marks.
- **Drag (M4) and chips (M5)** touch the diff pane and the composer inside the Changes/Review view,
  which #1269 reshapes; like M3 they wait until #1269 and #1271 are merged.

Only the micold-core milestones (M1 identity + `review::stale`; M2 `review::drag`, `review::chips`,
the settings key) build now. M3–M5 build in the Review tab once #1269 and #1271 have merged;
nothing in the core changes for that.

## Test strategy by layer

| Layer | Where | Covers |
|---|---|---|
| Core unit | `review/changes.rs` | `--raw` ids parsed (zero id → none), `assemble` sets `Sides`, rename keeps both ids |
| Core integration | `tests/review_git.rs` (real git) | FR-001a: same counts, different content → different identity (US1 AS12); content back → equal identity (AS4); untracked hashed; over-limit file by size+mtime; committed range needs no hash-object |
| Core unit | `review/pr_diff.rs` | patch hash differs on any byte of the section, equal for equal sections |
| Core unit | `review/stale.rs` | contracts/core-api.md S1–S10: FR-001–FR-004, rename counted once, add/remove counted, `observe` → `None` when equal, `adopt` both ways (AS10, AS11), `apply` empties |
| Core unit | `review/drag.rs` | contracts/core-api.md D1–D8: FR-015–FR-018 decisions, either direction, rows of the other side ignored, click vs range |
| Core unit | `review/chips.rs`, `settings.rs` | FR-024 empty/with text/trailing newline; the three chips' labels and texts; FR-022 default false, missing key reads false, round trip |
| Client state | `tests/features_changes.rs` + reducer tests | contracts/changes-view-hold.md H1–H16 (FR-005–FR-012, FR-014, US1 AS1–AS11, edges), contracts/quick-comment.md Q1–Q12 (FR-015–FR-021, FR-023–FR-026 reducer side) |
| Client state | `features/settings.rs` tests | the switch on the draft, Save applies, Cancel discards (FR-022, FR-026) |
| Geometry / snapshot | `ui/material` in-crate gates; `tests/layout_snapshot.rs` | `ChangedDot` and focusable `ToggleChip` geometry; the notice, the dot and the chips recorded (FR-029) |
| Manual | quickstart §A (scripted edits), §B visual pass 1280×800 light and dark | SC-001, SC-002, the gesture with mouse and touchpad, the look of notice, dot and chips |

Requirements mapped to where they are built and tested:

| FR | Built in | Tested in |
|---|---|---|
| FR-001–FR-004 | core `review::stale` | core unit `stale.rs` |
| FR-001a | core `review::{changes,git,pr_diff}` | core unit + `tests/review_git.rs` |
| FR-005, FR-006, FR-014 | `features::changes` `list_read` / PR refresh with `Held` | client state H1–H4, H12 |
| FR-007, FR-008 | `features::changes::{stale_notice, changed_since_you_looked}`, `ui/changes.rs` glue | client state H5, H6; §B |
| FR-009 | `features::changes` `FileSelected`, `DiffRead` adoption, user reads | client state H7–H11 |
| FR-010, FR-011 | `Msg::StaleApplied` reuses the refresh path's pick/composer rules | client state H13, H14 |
| FR-012 | `list_read` with no list on screen | client state H15 |
| FR-013 | M6, after #1271 | client state H16 (written in M6) |
| FR-015–FR-021 | core `review::drag`; reducer `GutterDrag*`; `DiffView` gutter | core D1–D8; client Q1–Q9; gutter in-crate message test; §B |
| FR-022, FR-026 | core `settings`; `features::settings`; `ui/settings/appearance.rs` | core unit; settings tests; §B |
| FR-023–FR-025, FR-027 | core `review::chips`; reducer `ChipPressed`; `ToggleChip` focus | core unit; client Q10–Q12; §B (keyboard order) |
| FR-028 | the core modules above | `mise run test-core` |
| FR-029 | docs, layout snapshot, §B | M3–M5 reviews against the FR-029 list |

## Delivery order

Milestones, each leaving the app working (tasks.md cuts them into tasks):

1. **M1 — core stale tracking**: identity in the list read and the PR split, `review::stale`. No
   visible change.
2. **M2 — core quick comment and chips**: `review::drag`, `review::chips`, `Settings::suggestion_chips`.
   No visible change.
3. **M3 — hold and notice in the Review tab, after #1269 and #1271** (US1, FR-005–FR-012, FR-014): reducer, shell,
   `ChangedDot`, header notice, docs *Keeping up with changes*, snapshot.
4. **M4 — drag to comment, after #1269 and #1271** (US2): gutter events, reducer, docs *Picking lines*, *Writing a comment*.
5. **M5 — suggestion chips, after #1269 and #1271** (US3): setting, composer chips, focusable `ToggleChip`, docs
   `settings.md` and *Writing a comment*, snapshot, visual pass of M3–M5 together.
6. **M6 — reviewed files while held** (FR-013): after #1271 merges.

M1 and M2 are independent, build now, and may run in parallel; M3 needs M1; M4 and M5 need M2;
M3, M4 and M5 also wait until #1269 (Review tab) and #1271 are merged. If the user's
answer to FR-014 is B or C, only M3 changes (the list stays live; `Held` holds only the open file's
row), and M1's `changed_since` serves all three answers unchanged.

## Complexity Tracking

No constitution violation to justify.
