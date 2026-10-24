# Research: Keyboard Map for Reviewing Changes (#1274)

Each decision: what was chosen, why, what was rejected. Paths are relative to the repository root.
"Today" means `origin/main` on 2026-10-24, where #1269, #1270, #1271, #1273 and #1127 are not merged.

## R1 — Split: blocker-free core first, wiring after the blockers

- **Decision**: every rule a press decides (which key is which action, which file / hunk / comment
  it lands on, the wrap rules, next unreviewed, what `c` opens, which review entries the jump-to
  field lists and how a query ranks them) is written in `micold-core` over plain inputs (`RelPath`
  lists, hunk counts, a reviewed predicate, comment facts). None of it names a type from #1269,
  #1270, #1271, #1273 or #1127. That is milestone **M1** and can merge before any blocker. The
  client wiring (listener, Review tab state, sheet, tooltips, jump-to entries), docs, layout
  snapshot and visual pass are **M2** (after #1269, #1270, #1271, #1273) and **M3** (after #1127).
- **Rationale**: the issue's acceptance asks for the pure logic in core with unit tests (Principle
  I); the spec's Dependencies section forbids stand-ins for another issue's work. Inputs as plain
  slices and closures are what each blocker will hand over whatever its own types turn out to be.
- **Rejected**: (a) one milestone after all blockers: idles the feature for weeks and lets the core
  rules drift with each blocker's types. (b) Building M2 against today's Changes view and porting
  it to the Review tab later: a stand-in the spec rules out, done twice.

## R2 — Key recognition: core matches the table; the client only says which modifier is primary

- **Decision**: core `review::keys::resolve(press, holder)` matches a `Press` against the
  `REVIEW_KEYS` table (R3). The client builds the `Press` from iced's
  `keyboard::Event::KeyPressed { text, key, modifiers, repeat, .. }`: the key (`Char(c)` for the
  produced character, `text` one char; `Enter`; else `Other`), `ctrl`, `alt`, `logo`, `shift`,
  `repeat`, and `primary`: whether the platform primary modifier is held (Cmd on macOS, Ctrl
  elsewhere), from a new `keymap::primary_held(mods)` beside the existing `is_primary_shift`. That
  one `cfg!(target_os)` stays in `keymap.rs` with the other chords' (Principle VI: core does not
  branch on the OS). No client code matches a key by hand: the listener and the composer both call
  `resolve` (coordinator requirement from #1276).
- **Modifiers** (FR-015): an entry with `primary: false` (the single keys) matches only with none
  of Ctrl, Alt, Cmd/logo held, except Ctrl **and** Alt together without logo, which is AltGr on
  Windows and is taken as unmodified so `[`, `]` and `?` work on layouts that need AltGr (spec edge
  case). Shift is ignored for single keys because the character already includes it (`?`; `J` is
  not `j` and is unbound). An entry with `primary: true` matches when `primary` is held, its
  `shift` equals the press's, and Alt and the non-primary of Ctrl/logo are not held.
- **Composing** (FR-015): a dead key or IME pre-edit produces no `text`, so its key is `Other` and
  resolves to nothing. iced 0.14 only enables an input method for a focused text widget, which
  never leaves the review holding the keyboard.
- **Auto-repeat** (FR-015): `repeat: true` resolves only for entries with `repeats` (`j k n p ] [`);
  `v g c ?` and the chord need a fresh press. The rule is data in the table, tested once.
- **Rejected**: physical-key matching as #803 does for its chords (spec says character; physical
  `[` is `ü` on a German layout); matching `Key::Character` (iced gives the unshifted key on some
  platforms, so `?` would read as `/`); a hand-written `is_review_send_chord` in `keymap.rs` (a
  second key match beside the table, which #1276's guide test could not see); an `is_macos` flag
  in core (the caller would still branch on the OS).

## R3 — One public table, `REVIEW_KEYS`: resolution, sheet, tooltips and docs read it

- **Decision**: `pub const REVIEW_KEYS: [ReviewKey; 12]` in `micold_core::review::keys`, public and
  stable (#1276's guide test compares the user guide's shortcut table against it). Each entry:
  `key: KeyName` (`Char(char)` or `Enter`), `mods: KeyMods { primary: bool, shift: bool }`
  (`primary` = Cmd on macOS, Ctrl elsewhere), `action: Option<ReviewAction>`, `repeats: bool` and
  `sheet: &'static str` (the `?` sheet's text). Rows: the ten single keys of FR-001, the send chord
  (`Enter`, primary + shift, FR-010) and the save-comment chord (`Enter`, primary; `action: None`:
  listed on the sheet only, the composer's text area keeps owning it). `ReviewKey::labels()`
  returns `(linux_windows, macos)` strings, as `keymap::nav_chord_labels` does; the client picks
  one with its existing `cfg!` pattern. The `?` sheet renders the table in order; tooltips (FR-013)
  read the label of the entry whose action they duplicate. A client docs test checks every label
  appears in `reviewing-changes.md`, and the send chord's two forms in `terminal-panes.md` and
  `worktrees-and-sessions.md` (SC-004, FR-020), as `tests/keymap.rs` does for #803/#968. It ships
  in M1 with `resolve`.
- **Rationale**: SC-004 asks the sheet and the guide to list the same set, and #1276 tests the guide
  against the table; one public table is the only way both stay true by construction.
- **Rejected**: a `match` per surface (resolver, sheet, tooltip, docs) that can drift; a table in
  the client (not reachable by #1276's core-side test, and not in the core-only milestone);
  deriving labels in the view (untestable glue with a rule in it).

## R4 — D4 provisional: keys on by default, no setting; how a setting would land

- **Decision**: no setting now (ledger D4, spec FR-001). The extension point is named so that the
  user's other answer costs one small change: `resolve` is the one function every single key
  passes, so a setting adds one argument (`single_keys: bool` from `Settings`) and one early
  return there, and the sheet filters `REVIEW_KEYS` on the same bool. No dormant code is written for
  it now (Principle I: no untested branch, no unused option).
- **Rejected**: adding the switch now, off or on (contradicts FR-001 as clarified); a global
  key-binding settings model (out of scope, spec Assumptions).

## R5 — Navigation rules: reuse `navigation::step`; hunks cross files with a pending edge

- **Decision**: new `micold-core/src/review/navigate.rs`.
  - Files (`j`/`k`, FR-003): `navigation::step` (#803's wrap rule) over the file order. No
    selection or a selection not listed gives first / last, which `step` already does.
  - Hunks (`n`/`p`, FR-004): `step_hunk` takes the file order with a `has_hunks` flag per file,
    the selected file, its diff state (`Loading` or `Hunks(count)`), and the current hunk. Inside
    the file it returns `Current(i)`; past an end it returns `File { path, edge }` for the next /
    previous file **with hunks** (wrapping; wrapping back to the same file gives `Current`), whose
    diff is not loaded yet, so the edge (`First` / `Last`) is kept as *pending* and resolved by
    `resolve_edge(edge, count)` when that diff arrives. Diff loading or failed: `Nothing` (edge
    case). No selection: first / last file with hunks, edge `First` / `Last`.
  - `has_hunks` for a working-tree file is `review::navigate::has_hunks(&ChangedFile, shown)`:
    text, some lines changed, and not a large file whose diff is not shown; binary, not-UTF-8,
    mode-only and pure renames have none. A pull request file passes the same flag from its
    `PrFile` patch (no hunks when the patch is empty).
  - Comments (`]`/`[`, FR-006): `comment_order` sorts comment facts by the file's index in file
    order, then the comment's located first line, then `created`, then id; comments of files not in
    the file order (filtered out, or no longer changed) go last, by path. `step_comment` uses
    `navigation::step` over that order from the last comment reached and returns
    `CommentMove { id, select, show }`: `select` is the file when it is listed and not selected;
    `show` is `Card` when the comment is in the diff, else `Drawer`.
  - Next unreviewed (`g`, FR-008): `next_unreviewed(order, selected, is_reviewed)` scans from
    after the selected file, wrapping, and returns `Select(path)` or `AllReviewed`; an empty order
    gives `Nothing`. With the selected file the only unreviewed one it returns that file (the
    selection stays).
  - `c` (FR-009): `comment_target(composer_open, picked, current_hunk)` returns `FocusComposer`
    (an open composer keeps its text), `Picked`, `Hunk { side, range }` or `Refused`. The hunk's
    range is first to last changed line on the new side, or on the old side when the hunk only
    removes (`hunk_change_range(&Hunk)`).
- **Why a line, not a diff row, orders comments within a file**: the row index is only known for
  the selected file's loaded diff; ordering other files by line and the selected one by row would
  reorder comments whenever a diff loads.
- **Rejected**: storing a separate "comment index" position (goes stale when comments change;
  `step` from an id re-reads the order at each press, as the spec's Principle II edge case asks);
  loading every file's diff to know hunk counts (cost grows with the review; `has_hunks` from the
  change list is enough).

## R6 — Jump-to entries: extend `quick_switch` with keys and a detail text

- **Decision**: `micold-core/src/quick_switch.rs`: `EntryKey` gains `ReviewFile(RelPath)` and
  `ReviewComment(CommentId)`. `Candidate` gains `detail: Option<String>`, a second text matched
  only when the name does not match. `Ranked` gains `via: Via { Name, Detail }`; `filter` sorts by
  `(via, rank, input index)`, so every name match ranks before every detail-only match (FR-017: a
  file name match before a directory-only match) and existing candidates (`detail: None`) keep
  their order exactly (FR-019). Spans index the text that matched. New
  `micold-core/src/review/jump.rs::review_candidates(files, is_reviewed, comments)` builds the
  candidates: a file's name is its file name, its detail its full path; a comment's name is its
  text (lines joined by spaces), its detail its file path. The client's
  `features/quick_switch.rs::candidates` inserts them after the active project's locations and
  sessions and before the other projects (FR-016), only when the current entry has a Review tab
  (FR-019).
- **Rejected**: matching the joined `path + text` as one name (a directory hit would outrank a name
  hit); a separate switcher list for review entries (FR-016 puts them in one list); a new `Rank`
  tier (changes the derived `Ord` existing callers rely on).

## R7 — Who holds the keyboard

- **Decision**: core `review::keys::Holder { Review, Composer, Sheet, Other }`. The client computes it in a
  render-free function beside the listener's gate: `Review` when the Review tab is shown (#1269),
  `window.focused_field` is `None`, no overlay is open (`overlay::registry::topmost`), the
  quick-switcher, Settings and the Overview are closed and the terminal is not focused; `Sheet`
  when the shortcut sheet is the topmost overlay; else `Other`. The listener is subscribed only
  while the holder is not `Other`, and iced's `keyboard::listen()` only yields presses no widget
  captured, so a focused field or a button's Enter/Space keeps its keys (FR-002, spec edge case
  "Focus on a button").
- **The composer** (FR-010, US4 s4): it holds the keyboard, so the listener never hears the chord.
  The client calls `resolve(press, Holder::Composer)`, which yields only `Send`. The shared
  `TextArea` gains a builder option `.on_press(fn(Press) -> Option<Message>)` consulted before its
  own bindings, which the composer sets to that call mapped to `SendPressed`; and its Ctrl/Cmd+Enter
  submit stops matching when Shift is held. Today `text_area.rs` submits on any
  `modifiers.command()` + Enter, so Ctrl+Shift+Enter would **save** the comment: this change is
  required, not cosmetic.
- **Rejected**: one global listener that peeks at the composer's text (it never sees captured
  keys); a feature-local text area (Principle VIII); the existing `.key_binding(…)` (feature 941):
  it *replaces* the default bindings, so the composer would lose `on_submit`'s Ctrl/Cmd+Enter save
  and the editor's defaults and have to re-create them; `.on_press` runs first and falls through to
  them.

## R8 — Shortcut sheet: a registered Dialog-layer surface

- **Decision**: a `ShortcutSheet` surface on the overlay registry's Dialog layer, like
  `QuickSwitcherDialog` (#968), listing `REVIEW_KEYS` with the platform's labels, built from shared
  `ui/material` parts. Escape and a click outside are the registry's dismissal; `?` closes it
  through `resolve` (`Holder::Sheet`). The review bar's overflow menu (#1270) gets a **Keyboard
  shortcuts** item sending the same open message (US5 s4).
- **Rejected**: a tooltip-like popover (no focus, no Escape route); a section in Settings (not
  reachable with `?`, off-surface).

## R9 — Hunk scroll and the current hunk mark

- **Decision**: core `review::diff::header_rows(shaped, layout)` returns each hunk header's row
  index in the unified or side-by-side rows (beside `UnifiedIndex` / `SideIndex`). The client turns
  the row into a scroll offset with the existing `jump_offset` arithmetic (row × `DIFF_ROW_HEIGHT`
  plus measured slot heights above) and the existing pending-jump mechanism (`panel.to_show`), so a
  pending edge resolves once `Load::Ready` arrives. The current hunk's header row is drawn with a
  theme-role accent bar (FR-005); colours come from `ui/material` roles, both themes.
- **Hunk controls** (FR-013): the redesign gives the diff header previous / next hunk buttons. If
  #1270 / #1273 do not deliver them, M2 adds two icon buttons there that send the same action as
  `n` / `p` (one message, one reducer path).
- **Rejected**: scrolling by line number (hunk headers have no line number; sections differ per
  layout).

## R10 — Tests by layer

- Core unit (M1): every rule of R2–R6 and R9 (`mise run test-core`), including a 200-file,
  500-comment timing test with a generous 50 ms bound so loaded CI does not flake (SC-003's 100 ms
  is the view's budget).
- Client render-free (M2/M3): `tests/keymap.rs` (send chord decode, near misses, conflicts with
  #803 FR-006 reserved lists and every existing app chord: FR-011; labels in docs: SC-004),
  `tests/review_keys.rs` (new: holder gate, press → reducer effects on a Review tab state, send
  chord ≡ `SendPressed`: SC-006, selection change clears current hunk), `tests/quick_switch.rs`
  (review entries present / absent, position in the list, choice → Review tab step).
- Registry: `tests/overlay_registry.rs` (`ShortcutSheet` identity, dismissal).
- Layout snapshot, showcase, visual pass at 1280×800 in both themes: quickstart §B (FR-021).
