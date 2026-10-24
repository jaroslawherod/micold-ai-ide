# Research: changed-file tree, filter and sort (#1273)

## R1: One tree layout in core, two modes
- **Decision**: a new pure module `micold_core::review::tree` lays out any list of `/`-separated paths
  as rows. `LayoutOptions` picks the mode: `chain` (join single-child folder chains, FR-002) and
  `Openness` — `DefaultOpen { collapsed }` for the changed-file tree (FR-004: every folder open at
  first) or `DefaultClosed { open }` for the **Files** browser (today's `FilesPane::open_folders`).
  `features::changes::tree_rows` (`crates/micold-client/src/features/changes.rs:518`) becomes an
  adapter over it with `chain: false, DefaultClosed`, so the **Files** tree keeps its rows (FR-017);
  its existing tests in `crates/micold-client/tests/features_changes.rs` are the parity check and keep
  their expectations. The core orders each level's files by byte order of the name, where today's
  `tree_rows` keeps input order (`features/changes.rs:527`); the two agree because `browse::list_tree`
  sorts every tree it returns (`crates/micold-core/src/review/browse.rs:289`, `:313`), and contract L12
  includes an unsorted input to pin the core's order. M1 ports those cases into the core tests first, so parity is proven before the
  client switches (M3).
- **Rationale**: spec FR-017 and Assumptions ("reuse #1150's `tree_rows` rather than adding a second
  tree layout"); `tree_rows` today lives in the client and has no totals, chaining or sorting, and
  constitution I wants the new rules in core. Moving it to core and generalising it is the reuse.
- **Rejected**: a second, changes-only tree builder beside `tree_rows` (the duplication FR-017 and
  `mise run duplication` forbid); extending `tree_rows` in the client (rules unreachable from core
  tests, and the client file is the one #1272 is editing).

## R2: Collapsed state is a set of real folder paths, plus a filter-time set
- **Decision**: `TreeChoices.collapsed: BTreeSet<String>` holds real folder paths. A chained row
  (`a/b/c` standing for `a`, `a/b`, `a/b/c`) shows collapsed when any of its paths is in the set
  (spec edge case "A chained folder splits or merges"). Collapsing a chained row inserts its deepest
  path; expanding removes all of its paths. While the filter holds text, rows ignore `collapsed` and
  use `filter_collapsed`, which is emptied whenever the filter becomes non-empty from empty and dropped
  when it is cleared, so a fold made while filtering never changes the tree restored on clear
  (FR-010, scenario 3.2). `reveal(path)` removes every ancestor of `path` from both sets (FR-008).
  Paths of folders no longer listed are kept, so a folder that comes back stays collapsed (FR-004,
  source switch keeps state per folder path).
- **Rationale**: per-path state survives re-reads, chain splits and source switches without any
  reconciliation step; a new folder is absent from the set and so appears open (edge case).
- **Rejected**: collapsing every path of the chain (a later split would collapse the outer folder the
  reviewer never folded); folding by row index (breaks on every re-read); clicks doing nothing while
  filtering (a dead control); letting filter-time folds write `collapsed` (violates FR-010).

## R3: Sort options and the Unreviewed-first switch
- **Decision**: `TreeSort { Path, Size, UnreviewedFirst }`. Ordering within a level: folders before
  files under every sort; Path by byte order of the row's label (today's list order, which is `RelPath`
  order); Size by total `added + removed` (folders: sum over the files listed below), largest first,
  ties by Path; Unreviewed first by "has an unreviewed file" (folders) / "is unreviewed" (files), ties
  by Path. Reviewed state is a caller-supplied predicate `Fn(&str) -> bool`, so #1271 can pass its set
  in whatever type it lands with. `sort_options(marks: bool) -> [(TreeSort, Offer); 3]` with
  `Offer::{Shown, Hidden, Disabled(&'static str)}` decides what the control offers; the policy before
  marks exist is one constant, `UNREVIEWED_UNTIL_MARKS = Offer::Hidden`. If the user's decision card
  picks "disabled with a reason", the change is that constant (to `Offer::Disabled("Needs reviewed
  marks (#1271)")`-style text), its unit test; the sort chips already draw `Offer::Disabled` as a disabled `ToggleChip` with the
  reason as its tooltip (R11) — no layout, reducer or component change. A sort the control does not offer is never stored: `TreeChoices::set_sort` refuses
  `UnreviewedFirst` while it is not `Shown`.
- **Rationale**: spec 4.2, 4.3, FR-012 and Clarifications (hidden until #1271; core tested against a
  reviewed set now).
- **Rejected**: a boolean `show_unreviewed` flag in the client (the hide/disable switch would then be
  UI branching with no test); ordering folders by count of unreviewed files (the spec says "holding
  any"); flattening under Size (Clarifications Q1).

## R4: Filter reuses the Open-file-by-path matcher
- **Decision**: `Query::new(filter)` once per layout, `typeahead::match_one(path, &query)` per file
  (the function `rank` calls for the **Files** picker, `crates/micold-core/src/typeahead.rs:154`).
  A file is listed when it matches; the result order is the sort's, not the match rank (FR-009).
  Highlight: a file row takes the `Match::spans` that fall in its name, rebased to the name; a folder
  row takes the union of its listed files' spans that fall in its label's byte range of the path (the
  range is the same in every path below it), rebased to the label. Whitespace-only text is no filter
  (`Query::is_empty`).
- **Rationale**: scenario 3.5 and spec Assumptions ("the matcher Open file by path already uses").
- **Consequence for fixtures**: the matcher has approximate tiers (single edit, subsequence) behind
  its three-character floor, so SC-001's fixture must hold no other path that `tests.r` reaches by a
  single edit or a subsequence; the test asserts exactly one listed file.
- **Rejected**: a plain substring filter (a second matching rule, contradicts 3.5); ranking filtered
  files by match quality (FR-009 says the filter does not reorder).

## R5: Esc in the filter box is caught by the field
- **Decision**: the shared `TextField` gains a builder method `.on_escape(M)`: while the field holds
  focus, an Escape key press publishes `M` and is captured there. The filter box passes
  `Msg::TreeFilterCleared`, and the same message is the field's trailing clear action
  (`TextField::trailing_action`). The reducer ignores it when the filter is already empty. Tested at
  widget level with the `ui/material/test_support.rs` harness (`send` an Escape press to a focused
  field: one message; unfocused: none), and at reducer level (message empties the filter, keeps
  `collapsed`).
- **Rationale**: scenario 3.6, FR-010. The app-wide route cannot see this key: `escape_dismissal`
  (`crates/micold-client/src/ui/mod.rs:1476`) is subscribed only while `app::on_escape` has an answer
  (`ui/mod.rs:1361`), and a focused text input captures Escape before a global listener hears it (the
  same reason the composer's Escape "is its editor's, captured there", `ui/mod.rs:1354`).
- **Rejected**: an arm in `app::on_escape` / `dismiss_topmost` gated on a focus flag (never reached:
  the field captures the key); a key subscription local to the Changes view (a second Escape
  implementation).

## R6: Rows drawn on `VirtualRows`, not `TreeView`
- **Decision**: the changed-file tree keeps the list's `VirtualRows` (`ui/material/virtual_rows.rs`)
  with one fixed row height; each row is a button like today's `file_row`, indented by
  `indent_levels(depth)` steps. The **Files** browser keeps `TreeView`.
- **Rationale**: FR-015 and the 5,000-file edge case: `TreeView` builds every row; `VirtualRows`
  builds only the visible ones and is what the flat list uses today.
- **Rejected**: making `TreeView` virtual in this feature (a change to a component the sidebar uses;
  out of scope).

## R7: The bar is a shared `ChangeBar` component
- **Decision**: `ui/material/change_bar.rs`, `ChangeBar::new(parts: BarParts, roles)` with
  `.max_width(px)`; draws two adjacent rounded segments in the added and removed roles' colours
  (`r.primary`-family tokens chosen at M4 against the mockup) at the lengths core computed; `From<ChangeBar>
  for Element`. Added to `showcase/sections/review.rs` so `tests/showcase_completeness.rs` passes.
- **Rationale**: Principle VIII (a new visual primitive goes in the library, builder API); the
  review-redesign mockup puts the bar on every file row.
- **Rejected**: two coloured `container`s inline in `ui/changes.rs` (a feature-local widget);
  `ProgressBar` (one value, not two parts).

## R8: Bar maths and the indentation cap
- **Decision**: `bar_scale(files)` = the largest `added + removed` over the **whole** change list
  (not the filtered rows), so bars do not jump while typing. `bar(added, removed, scale, max_width)`
  returns `None` when `added + removed == 0` (binary, not text, Mode only, pure rename: all carry zero
  counts in `ChangedFile`); otherwise total length `max(MIN_BAR, max_width * total / scale)`, split
  into added and removed in exact proportion (`f32`), so 40/10 gives a 4:1 split (scenario 2.1) and
  the total never exceeds `max_width` (2.2). `MIN_BAR` = 2 px keeps a one-line change visible.
  Folder rows show summed counts as text, no bar (FR-007). Indentation: `max_indent_levels(list_width,
  step, min_name_width)` and `indent_levels(depth, max) = min(depth, max)`; the client passes its row
  constants, so the name keeps `min_name_width` before indentation stops growing (edge case "Very deep
  paths").
- **Rationale**: spec Assumptions ("bar scale relative to the largest change in the current list");
  FR-006.
- **Rejected**: log scale (parts would no longer read as line counts; the spec asks for proportion);
  a per-part minimum (breaks "grows with total"); GitHub-style five blocks (loses the 4:1 ratio).

## R9: SC-003 timing
- **Decision**: a core test lays out 5,000 synthetic files in 200 folders and asserts each of
  collapse, one filter keystroke and a sort change re-lays out in under 100 ms, in the style of
  `crates/micold-core/tests/command_palette.rs:255` (slowest of repeated runs). The client caches
  `OpenView::tree_rows` and recomputes it only in `update`, so drawing a frame does no layout.
- **Rationale**: SC-003 names "the core layout benchmark or a timed unit test"; the crate has no bench
  harness, and the existing timed tests are plain `#[test]`s.
- **Rejected**: adding `criterion` (a new dependency for one measurement); laying out in `view`
  (layout per frame).

## R10: Milestone order against #1272 and #1271
- **Decision**: M1–M2 are core-only and change no client file, so they can merge while #1272 is open
  and conflict with nothing it touches. M3–M5 (reducer, drawing, docs, snapshot, visual pass) start
  after #1272 merges and follow wherever it put the list pane. #1271 is not waited on: when it lands
  it passes its reviewed predicate and `marks: true` to `sort_options`.
- **Rationale**: orchestration note for this run; spec Assumptions (#1272 owns the column, #1271 the
  marks).
- **Rejected**: building the UI against today's single-column list pane (it would be rewritten when
  #1272 lands and its conflicts resolved twice).

## R11: Sort control is a row of `ToggleChip`s
- **Decision**: one `ToggleChip` per offered sort (`Path`, `Size`, later `Unreviewed first`), the
  chosen one `.active(true)`, each sending `Msg::TreeSortChosen(sort)`; an `Offer::Disabled(reason)`
  option is `.disabled(true)` inside the shared `Tooltip` (`ui/material/mod.rs`) carrying the reason;
  `Offer::Hidden` draws nothing. Same component and look as the list's Committed / Uncommitted chips.
- **Rationale**: two or three choices fit on one line at the list's width and stay visible without a
  click; `ToggleChip` already supports `disabled`, so the R3 switch needs no component change.
- **Rejected**: `Select` — it deliberately has no disabled options (`ui/material/select.rs:671`), so
  "disabled with a reason" would need a component change; a menu behind an icon (hides the current
  sort).
