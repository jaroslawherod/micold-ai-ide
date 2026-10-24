# Research: Review UI — Stale-Awareness and Quick Comment

Each decision with what was rejected. Code references are to the tree at the plan's commit.

## R1 — Where the change identity comes from (FR-001a)

**Decision**: per row, `ChangeIdentity::Sides { old, new }` of two `VersionId`s.

- The old side, and a new side that is a commit (`BaseToHead`), come from the ids `git diff -z -M
  --raw` already prints (`:<mode> <mode> <old id> <new id> <status>`), which
  `review::changes::parse_name_status` skips today (`[old_mode, new_mode, _, _, status]`). Those
  ids are abbreviated by default (`DIFF` in `review/git.rs` has no `--no-abbrev`), and git lengthens
  an abbreviation as the object count grows, so `range_rows`/`range_paths` add `--no-abbrev` to the
  `--raw` read: every id is then full length (40 hex, 64 in a SHA-256 repository), the form
  `hash-object` prints, so a listed id and a hashed one compare equal. An id of all zeros (full
  length) means *absent* on an added/deleted side.
- A working-tree new side: `--raw` prints zeros when the file is not stat-clean against the index,
  and the index blob's id when it is. Zeros are filled by one `git hash-object --stdin-paths` over
  those paths (untracked files too), with the default filters, so a file edited back to its
  indexed content hashes to the same id the stat-clean case reports (US1 AS4, FR-003).
- A working-tree file over `limits::MAX_VERSION_BYTES`, or one that is not a regular file:
  `VersionId::Stat { bytes, modified_ns }` from `symlink_metadata` (Edge *Large files*).
- A deleted working-tree side: `Absent`. A path with a newline (which `--stdin-paths` cannot carry):
  `Stat`.
- A pull request row: `ChangeIdentity::Patch(u64)`, a `DefaultHasher` hash of the file's section of
  the request's diff, set by `split_pr_diff`.

**Rationale**: the list read already runs `--raw`; one extra process per read keeps the 2 s budget
(SC-002) and needs no per-file diff (spec Assumptions). Ids make an equal-counts edit visible (AS12).

**Alternatives rejected**:
- *Compare `+/-` counts only*: misses an edit that keeps the counts (AS12); the spec's F1 finding.
- *Read every file's diff each time*: N git processes per burst; breaks the budget on large lists.
- *Hash files in Rust (`sha1` crate)*: a new dependency, and git's filters (CRLF, clean) would not
  apply, so a file back to its indexed content would not compare equal to the index id.
- *mtime for every working-tree file*: a touch or a save with no change would count; acceptable
  only where hashing is too costly, which is the over-limit case.
- *Stable hash (FNV, SHA) for the patch*: nothing is stored; `DefaultHasher` is enough within one
  process, and `core::store` already relies on it the same way.

## R2 — Where the comparison and the held snapshot live (FR-001–FR-004, FR-028)

**Decision**: a new pure module `review::stale` with `changed_since(shown, latest)`, a `Listing`
trait (implemented for `ChangeList` and `PrDiff`), and `Held<L> { latest, changed }`, created only
by `observe(shown, latest) -> Option<Held<L>>`. The shown snapshot stays where the view keeps it
today (`OpenView.list`, `PullRequestView.diff`); `Held` keeps the latest read and the set.

**Rationale**: FR-028 puts the set and count in the core. Keeping the shown list where it is leaves
every existing reader (`shown_list`, `list_body`, `request_diff`) unchanged. `observe` returning
`None` for an empty set makes "held but nothing changed" unrepresentable (Principle V).

**Alternatives rejected**:
- *Reducer-local comparison in `features::changes`*: testable, but FR-028 asks for the core, and the
  PR path would duplicate it.
- *Move the shown list into `Held`*: every reader of `view.list` would change, for no behaviour.
- *Two separate types for working tree and pull request*: same rules twice; `Listing` costs one
  trait with two small impls.

## R3 — Telling a watch read from a user's read (FR-005, FR-009 first sentence, FR-012)

**Decision**: `OpenView.apply_next: bool`. Every read the user causes (toggles, source change,
reopen, base change through a reopen) sets it before `request_read`; a queued read (`again`) keeps
it set; `list_read` consumes it. A read that ends with `apply_next` set, or with no list on screen
(`Idle`, `Loading { last: None }`, `Failed`, or a ready list with no rows), replaces the list as
today. Any other read goes through `observe`.

**Rationale**: the only watch-driven read is `Msg::Changed → request_read`; marking the others
explicitly is safer than inferring from message kinds, and a user read queued behind a watch read
still applies (H3).

**Alternatives rejected**:
- *A cause field inside `Load::Loading`*: `Load` is shared by the header counts, tree and diff,
  which have no such notion.
- *Two effects (`ReadList` vs `ReadListHeld`)*: the shell would carry a reducer decision.

## R4 — What a background read no longer does

**Decision**: a held (watch) read does not re-read the selected file's diff (today `list_read`
calls `request_diff(state, true)`), does not change `selected`, `pick` or the composer, and does not
re-judge anchors. If `observe` returns `None` (nothing differs), the latest list silently replaces
the shown one: by construction no row's kind, counts or identity changed, so nothing visible moves,
and `large`/`origin` stay current. `refresh_browsed` still runs: the **Files** browser keeps today's
behaviour (spec Edge *Whole file and browsed files*).

**Alternatives rejected**: *keep re-reading the diff into a hidden buffer*: a read per burst for a
diff the user is told about anyway; selecting or applying reads it.

## R5 — The diff-time row (FR-009, Edge *Selected between reads*)

**Decision**: the shell answers a working-tree `ReadDiff` with the diff and the file's current row,
read in the same off-thread job by `GitCli::change_list_of(dir, scope, toggles, &[path, from])`, a
pathspec-limited `change_list` that shares its code (identity included). The reducer adopts that
row through `Held::adopt` (or, with nothing held, writes it into the shown list): the shown row and
the held latest row both take it, so the file leaves the set and the next background read compares
against what the user now sees.

`Msg::DiffRead.row` is a `DiffRow`: `Row(ChangedFile)` as above; `Unlisted` when `change_list_of`
succeeds with no row for the path (the reducer shows "no changes any more" as H9 does and leaves
the shown and held rows to the next watch read); `NotRead` for a read that is not of the working
tree and when `change_list_of` fails (its error is not shown: the diff still shows, and nothing is
adopted, so the file keeps its dot until the next watch read settles it).

**Rationale**: the diff read on selection may be newer than the last background read; without the
row, the next background read would count a change the user is already looking at.

**Alternatives rejected**:
- *Take the row from the held latest read*: may be older than the diff shown (the edge case).
- *Compute counts from the diff's hunks*: no identity, and binary or large files have no hunks.

## R6 — Pull requests (US1 AS8, FR-009 last sentence)

**Decision**: `follow_head` keeps reading the new head's diff; when that refresh answers, a ready
diff with a non-empty set becomes `PullRequestView.held` instead of replacing `pr.diff`. Selecting a
file in the set adopts its row and patch from the held latest; others show the held snapshot's
patch, as today. The notice applies `held.latest`.

**Alternatives rejected**: *apply PR refreshes at once (exempt from the hold)*: AS8 requires the
same hold.

## R7 — The drag gesture (FR-015–FR-021)

**Decision**: core `review::drag::Drag { side, pressed, last, moved }`. `Drag::press` fixes the side
by today's C1 rule (moved to core as `gutter_line(old, new)`: new number first, else old);
`over(old, new)` moves `last` only when the row has a number on that side; `release()` returns
`Release::Click { side, line }` when the pointer never reached another line, else `Release::Range
{ side, anchor, head }`. Moving away and back to the pressed line is still a range of one line
(it opens the composer: the user dragged).

Client: `DiffView` gains `.on_gutter_drag(press, over)`; its `Gutter` widget publishes a press as
today (Shift still extends, FR-018) and, while the reducer says a drag is on (`.dragging(true)`),
publishes `over` when the cursor enters it. Release and **Esc** are caught anywhere in the window by
an iced event subscription that is active only while `OpenView.drag` is `Some` (release outside the
gutter or the window ends the drag at the last line passed, Edge *Drag leaves the diff*).

**Rationale**: per-cell widgets already know their numbers; a window-wide listener is the only way
to see a release outside the diff. The subscription exists only during a drag, so it costs nothing
otherwise.

**Alternatives rejected**:
- *One widget over the whole gutter column that maps y to rows*: duplicates the diff's row geometry
  (slots, gaps, side-by-side halves) that the per-row cells already have.
- *Treat every press as a drag start and a click as a zero-length drag that opens the composer*:
  breaks FR-018 (a click only picks and offers **Add comment**).
- *Auto-scroll at the edge*: out of scope.

## R8 — Suggestion chips and their setting (FR-022–FR-027)

**Decision**: `review::chips::{SuggestionChip, CHIPS, insert}`; `insert` sets an empty (or
whitespace-only) box to the chip's text and otherwise appends it on a new line (no extra newline
when the text already ends with one). `Settings::suggestion_chips: bool`, `#[serde(default)]`,
client-owned and saved with the Settings form like `reduce_motion`, shown in **Appearance** under
**Reduce motion**. The composer's text is the reducer's (`Composer.text`); the shell's existing
`sync_composer` puts it in the editor and moves the cursor to the end, then a focus operation on the
composer's text area keeps the focus there. Chips are `ToggleChip`s (never active), extended to join
the keyboard traversal through `TakesTheKeyboard`; their visible text is their label.

**Alternatives rejected**:
- *A new **Review** section in Settings*: a rail page for one switch.
- *Service-owned setting (`SettingsSet` on the wire, like `diff_layout`)*: a protocol bump for a
  per-user display preference; FR-026 asks only that boxes opened after a save follow it.
- *A new chip widget*: `ToggleChip` already draws a pill with a press; Principle VIII says extend.

**Known limit**: iced 0.14 exposes no accessibility tree, so "announce their label" (FR-027) is met
the way every control in the app is: the chip's label is its visible text, and keyboard focus shows
the ring. No control in the app announces more than that today.

## R9 — #1269's re-read when the Review tab is shown again

**Decision**: when #1269 lands, its show-time re-read is a watch read (no `apply_next`), so a user
coming back to a tab with a list on screen gets the notice, not a moved diff.

**Rationale**: the redesign's wording ("changed since you looked") is the same for the hidden tab
and for this hold; the reader returns to what they left. Whoever merges second makes the call site
not set `apply_next`.

**Alternative rejected**: *apply on show*: the tab would reflow exactly when the user looks back at
it, which item 8 exists to stop.

## R10 — FR-013 with #1271

**Decision**: M6, after #1271. The view treats a reviewed file in the held set as unseen: the row
shows the dot and no reviewed mark, and *n of m reviewed* counts `reviewed ∖ changed`. Whether the
mark is cleared in the service when the change is applied is #1271's rule ("a changed file drops
back to unseen"); this feature only makes a held change count as a change for it.

**Alternative rejected**: *clear the reviewed mark in the service as soon as a change is held*: a
write to shared state from one window's in-memory hold, which other windows would see without the
change being applied there (Edge *Two windows*).

## R11 — Milestone order

**Decision**: the micold-core milestones build now (M1 identity + `review::stale`; M2
`review::drag`, `review::chips`, the settings key). The client milestones that touch the
Changes/Review view (M3 hold and notice, M4 drag to comment, M5 chips UI) wait until #1269 (Review
tab) and #1271 are merged, because #1269 reshapes that view. M6 (FR-013) waits for #1271. See plan
*Delivery order*.

**Rationale**: the core is independent of every blocker and carries FR-028, so it lands with its
unit tests now; the client parts are written once, in the view they will live in.

**Alternative rejected**: *build M3–M5 now in today's Changes view*: #1269 then reshapes the view
under them, and the hold, gutter and composer glue would be rebased or rewritten (rework cost).
