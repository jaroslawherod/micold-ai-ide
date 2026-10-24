# Data Model: Review UI — Stale-Awareness and Quick Comment

Nothing here is persisted except `Settings::suggestion_chips`. Rules are in
[contracts/](./contracts/); decisions in [research.md](./research.md).

## Core (`micold-core`)

### §1 `review::changes::VersionId` (new)

One side of a row's diff (R1).

| Variant | Meaning |
|---|---|
| `Unknown` (default) | Not known: rows built before this feature, struct literals in tests |
| `Absent` | No version on this side (added file's old side, deleted file's new side) |
| `Object(String)` | A git object id (hex), from `--raw` or `hash-object` |
| `Stat { bytes: u64, modified_ns: u128 }` | Size and modification time, for a working-tree file over the size limits or not hashable |

### §2 `review::changes::ChangeIdentity` (new)

| Variant | Meaning |
|---|---|
| `Unknown` (default) | No identity; two `Unknown`s compare equal, so such rows compare by kind and counts alone |
| `Sides { old: VersionId, new: VersionId }` | Working tree or committed range |
| `Patch(u64)` | A pull request file: hash of its section of the request's diff |

Equality is structural (`PartialEq`).

### §3 `ChangedFile` (extended)

New field `identity: ChangeIdentity` with `#[serde(default)]`. `NameStatus` gains `old_id` and
`new_id: Option<String>` (`None` for an id of all zeros; ids are full length because the `--raw`
read passes `--no-abbrev`, §4). `assemble` sets `Sides` from them, leaving a
working-tree new side `None` → filled by `git.rs` (§4). Every struct literal of `ChangedFile`
(core `context_header.rs`, `runs/summary.rs`, `pr_diff.rs`, `tests/review_git.rs`, client
`tests/features_changes.rs`) gains `identity: ChangeIdentity::Unknown` or a real value.

### §4 `review::git` (extended)

- `range_rows` and `range_paths` add `--no-abbrev` to their `--raw` read (R1), so ids are full
  length and compare equal to `hash-object` output; the name-only reads are unchanged.
- `identify(dir, files: &mut [ChangedFile])` (private): for each row whose new side is a working
  tree version with no id, one `git hash-object --stdin-paths` over the hashable paths (regular
  file, ≤ `limits::MAX_VERSION_BYTES`, no newline in the path); others get `Stat`; a missing file
  `Absent`. Called by `change_list` for `BaseToWorktree` and the uncommitted range, after
  `merge_untracked`.
- `GitCli::change_list_of(dir, scope, toggles, paths: &[RelPath]) -> io::Result<ChangeList>`: the
  same read limited to `paths` (git pathspec; `ls-files` limited the same way). `change_list` is
  `change_list_of` with no paths.

### §5 `review::pr_diff` (extended)

`split_pr_diff` sets each `PrFile.file.identity = Patch(hash(section bytes))`.

### §6 `review::stale` (new)

| Item | Shape | Notes |
|---|---|---|
| `changed_since(shown: &[ChangedFile], latest: &[ChangedFile]) -> BTreeSet<RelPath>` | fn | FR-001, rename rule; contracts/core-api.md S1–S6 |
| `trait Listing: Clone` | `fn rows(&self) -> &[ChangedFile]` (or an iterator), `fn put(&mut self, path: &RelPath, from: &Self)` | `put` copies `path`'s row (and, for `PrDiff`, its patch) from `from`, or removes it when `from` lacks it; keeps the listing's order |
| `impl Listing for ChangeList`, `impl Listing for PrDiff` | | |
| `Held<L: Listing> { latest: L, changed: BTreeSet<RelPath> }` | struct, fields private | exists only with `changed` non-empty |
| `observe<L>(shown: &L, latest: L) -> Observed<L>` | fn | `Observed::Same(L)` (nothing differs: the caller may show `L`) or `Observed::Held(Held<L>)` |
| `Held::count`, `contains(path)`, `latest_lists(path)`, `latest()` | getters | FR-002, FR-008, FR-009 |
| `Held::observe(self, shown, latest) -> Observed<L>` | method | a newer background read replaces `latest`; the set is recomputed (FR-003) |
| `Held::adopt(self, shown: &mut L, path, fresh: &L) -> Option<Held<L>>` | method | FR-009: `shown` and `latest` take `fresh`'s row for `path`; if `fresh` lacks it, only `latest` drops it (AS11). `None` when the set becomes empty |
| `Held::apply(self, shown: &mut L)` | method | FR-004: `*shown = latest` |

### §7 `review::drag` (new)

| Item | Shape | Notes |
|---|---|---|
| `gutter_line(old: Option<u32>, new: Option<u32>) -> Option<(Side, u32)>` | fn | C1: new number first, else old; `None` for neither. The reducer's `GutterPressed` uses it too |
| `Drag { side, pressed, last, moved }` | struct, fields private | `Drag::press(side, line)` |
| `Drag::over(&mut self, old, new)` | method | moves `last` when the row has a number on `side` (FR-016, FR-017); sets `moved` when `last != pressed` |
| `Drag::span(&self) -> (Side, LineRange)` | getter | what to draw as picked during the drag |
| `Drag::release(self) -> Release` | method | `Click { side, line }` when never moved, else `Range { side, anchor: pressed, head: last }` (FR-015, FR-018) |

### §8 `review::chips` (new)

| Item | Shape |
|---|---|
| `SuggestionChip { label: &'static str, text: &'static str }` | struct |
| `CHIPS: [SuggestionChip; 3]` | *Rename* → "Rename this to ", *Add test* → "Add a test that covers this.", *Why?* → "Why is this needed?" (spec Assumptions) |
| `insert(text: &str, chip: &SuggestionChip) -> String` | FR-024; contracts/core-api.md K1–K4 |

### §9 `settings::Settings` (extended)

`suggestion_chips: bool`, `#[serde(default)]` (false). Client-owned: the daemon never reads it.
Settings are saved through the private `StoredSettings` (`settings.rs`), so the field is added
there too, with `#[serde(default)]`, and carried by both conversions, `StoredSettings::from_settings`
and the conversion back to `Settings`, as `reduce_motion` is.

## Client (`micold-client`, render-free `features/`)

### §10 `features::changes::OpenView` (extended)

| Field | Type | Meaning |
|---|---|---|
| `held` | `Option<Held<ChangeList>>` | the latest working-tree read and the changed set while held |
| `apply_next` | `bool` | the read in flight (or queued) was caused by the user: apply it (R3) |
| `drag` | `Option<DragState>` | a gutter drag in progress |

`DragState { drag: Drag, before_pick: Option<Pick>, before_composer: Option<Composer> }` — what
**Esc** restores (FR-020).

### §11 `PullRequestView` (extended)

`held: Option<Held<PrDiff>>`, set when a head refresh answers with differences (R6). The shown
snapshot stays `diff: PrDiffLoad::Ready { files, patches }`; the reducer converts between it and
`PrDiff` at the hold boundary, or `PrDiffLoad::Ready` holds a `PrDiff` directly (tasks decide).

### §12 Messages and effects

| Item | Change |
|---|---|
| `Msg::StaleApplied` | new: the notice was pressed (FR-007) |
| `Msg::GutterDragOver { old, new }` | new: the pointer entered a gutter cell during a drag |
| `Msg::GutterDragReleased` | new: the button was released anywhere |
| `Msg::GutterDragCancelled` | new: **Esc** during a drag |
| `Msg::GutterPressed` | unchanged shape; starts a `DragState` as well as picking (a click is a drag that never moves) |
| `Msg::ChipPressed(usize)` | new: index into `CHIPS` |
| `Msg::SuggestionChipsInForce(bool)` | new: the applied setting, at start and after a save (FR-026) |
| `Msg::DiffRead` | gains `row: DiffRow`: the diff-time row (R5) |
| `DiffRow` | new enum: `NotRead` (a read that is not of the working tree, or `change_list_of` failed; the diff itself still shows), `Unlisted` (`change_list_of` succeeded and lists no row for the path), `Row(ChangedFile)` |
| `Effect::ReadDiff` | unchanged shape; the shell also runs `change_list_of` |
| `State.suggestion_chips` | new `bool`: the setting in force |

### §13 Read-only helpers (for the glue and #1273's tree)

| Helper | Returns |
|---|---|
| `stale_notice(view) -> Option<String>` | *1 file changed since you looked* / *n files changed since you looked*, `None` when nothing is held |
| `changed_since_you_looked(view, path) -> bool` | whether `path`'s row carries the dot |
| `no_longer_changed(view) -> bool` | the selected file is in the set and the latest read no longer lists it (AS11) |
| `chips_shown(state) -> bool` | the setting is on and a composer is open (FR-023) |

### §14 `features::settings` (extended)

`suggestion_chips` on the Appearance part of the draft and on the applied state, with a
`SuggestionChipsToggled(bool)` message, following `reduce_motion`.

## State transitions — the hold (working tree)

```text
            watch read, list on screen, differences
 Showing ──────────────────────────────────────────▶ Held(n)
   ▲  ▲                                               │  │ watch read: recompute n (0 → Showing,
   │  │ user read / notice / no list on screen        │  │   latest becomes shown)
   │  └───────────────────────────────────────────────┘  │ select file in set: adopt row (n-1;
   │                                                     │   0 → Showing)
   └─────────────────────────────────────────────────────┘
```
