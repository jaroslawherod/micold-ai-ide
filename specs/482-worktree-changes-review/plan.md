# Implementation Plan: Review a Worktree's Changes and Send Comments to Its Session

**Branch**: `claude/project-thread-v1va8z` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/482-worktree-changes-review/spec.md` (issue #482).

## Summary

Each sidebar entry gets a Changes view in the main area: the files that differ from the entry's
base (merge-base with the default branch; uncommitted only for the Default entry), with committed
and uncommitted toggles, and a unified or side-by-side, syntax-coloured diff of the selected file.
Lists and diffs come from the user's `git`, parsed by pure functions in a new
`micold_core::review` module, and are read by the window off the UI thread; a `notify` watcher
refreshes them. Comments on line ranges are owned by the daemon (single writer, persisted per
project, pushed to every window), and **Send to session** has the daemon build one prompt from the
pending comments (pure `review::prompt::build`) and type it into the entry's most recently active
running session, or into a new session it starts, reusing feature 034's submission path. Comments
become sent only once the prompt is written.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`), MSRV 1.97

**Primary Dependencies**: iced 0.14 with its `highlighter` feature enabled (new: `iced_highlighter`
0.14 → `two-face` 0.4, syntect `fancy-regex`, research R10); `notify` 8.2 added to `micold-client`
(already a workspace dependency of the daemon, R9); the user's `git` CLI (R1). No diff crate.

**Storage**: new `reviews/<project id>.json` beside `projects/`, written by the daemon (R3); new
setting `Settings::diff_layout` (R12)

**Testing**: `mise run test-core` (core unit tests + real-git integration tests in
`crates/micold-core/tests/`), `mise run gate` (daemon integration tests with the fake CLI, client
state tests, in-crate geometry gates), quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-daemon`,
`micold-client`)

**Performance Goals**: no frame over 100 ms with 2,000 files or a 50,000-line diff (SC-003);
refresh within 2 s of a change (SC-004)

**Constraints**: offline; diff bodies never cross the wire; prompt bytes identical on all platforms
(SC-006); limits 5,000 changed lines / 2 MB per version / 50 quoted lines (R8)

**Scale/Scope**: one new core module (8 files), one daemon module, protocol v30, one client
feature + shell + glue, four new shared components, one user-guide page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision is in tested code: parsers, pairing, classification, comment state machine, prompt, target choice, watch filter in core unit tests; git reads against real temp repos; send/persist/isolation in daemon integration tests; view reducer in client state tests; windowing and component layout in geometry gates. Only `ui/changes.rs`, `shell/*` I/O wrappers and sidebar menu wiring are glue (quickstart §B). Red first per the TDD loop. |
| II. Multi-Session | PASS | Comments are per entry, never per process; a prompt goes only to a session of its entry (W2, W7); several sessions per entry handled by `pick_target`; sessions stay independently addressable; the send never resumes or alters another session. |
| III. Worktree Integration | PASS | All git reads run in the entry's directory; the Default entry is uncommitted-only by type (`ReviewScope::RootUncommitted`), creates or changes no worktree, and is not styled as one; comments are removed with their worktree through the app's own delete. |
| IV. Local-First | PASS | Local git and files; comments in a local JSON file; the prompt goes only to a local AI CLI session on the user's explicit Send; prompt text never logged (W12). |
| V. Rust + iced | PASS | iced's own highlighter, no other GUI crate. Types make invalid states unrepresentable: `ReviewScope`, `LineRange::new`, `CommentState` with no way back from `Sent`, `FileDiff` variants without lines for binary/large. |
| VI. Cross-Platform | PASS | `RelPath` with `/`, CR stripped, `\n`-only prompt; `notify`'s recommended watcher per OS behind one subscription; git invoked through the existing `no_window` wrapper (Windows); CI on all three; quickstart §C. |
| VII. Documentation | PASS | New `docs/user-guide/reviewing-changes.md`, linked from `docs/SUMMARY.md`, `docs/README.md` and `worktrees-and-sessions.md` § Managing a worktree; `settings.md` if the layout appears there. Same PR as the UI. |
| VIII. Components | PASS | New shared builders in `ui/material`: `VirtualRows`, `DiffView`, `ReviewCommentCard`, `TextArea`, each with showcase poses and geometry tests; everything else reuses existing primitives (`Tag`, `ToggleChip`, `LabelledToggle`, `Button`, `Snackbar`, `Modal` + `dialog::body`, `StageProgress`, `Scrollable`). |

Post-design re-check: unchanged, all PASS.

## Design

### D1 — Core `review` module (render-free) — R1, R7, R8, R13, R14

`crates/micold-core/src/review/` (new; `pub mod review;` in `lib.rs`):

- `mod.rs` — `RelPath`, `Side`, `LineRange`, `limits` constants.
- `base.rs` — `ReviewScope`, `Base`, `BaseUnavailable`, `Toggles`, `DiffRange::for_view`, and
  `default_branch_from(origin_head, has_main, has_master)`.
- `changes.rs` — `parse_numstat`, `parse_name_status` (`-z`), `merge_untracked`, `classify`
  (binary / not UTF-8 / large) → `ChangeList`.
- `diff.rs` — `parse_unified(&str) -> FileDiff` (CR stripping, no-newline markers, binary notice),
  `added_file(bytes) -> FileDiff`, `unified_rows`, `side_by_side_rows`, `SideLines`.
- `comment.rs` — `ReviewComment`, `CommentState`, `EntryReview` operations and `SendSnapshot`,
  `is_outdated` (data-model state machine).
- `prompt.rs` — `build` per contracts/review-prompt.md.
- `target.rs` — `pick_target`.
- `watch.rs` — `relevant_paths(entry_root, git_dirs, &[PathBuf]) -> Vec<PathBuf>`.
- `git.rs` — the I/O: `impl GitCli { fn review_base, fn change_list, fn file_diff,
  fn ignored }` running the commands of R1 through the helpers in
  `crates/micold-core/src/git.rs` (`no_window`, `local_only`, `run_git`; the last two are private
  today and become `pub(crate)`), returning the pure types.
- `store.rs` — `ReviewFile` (de)serialisation, path `reviews/<project_id>.json` via a new
  `JsonFileStore::reviews_dir()` beside the private `project_state_dir()` in
  `crates/micold-core/src/store.rs`; `reviews_dir` stays private and the daemon reaches review
  files only through `JsonFileStore` load/save methods.

### D2 — Protocol v30 — contracts/review-wire.md

`crates/micold-core/src/protocol/messages.rs`: `ClientMsg::ReviewEdit`, `ClientMsg::ReviewSend`,
`ReviewEditOp`, `SettingsSet.diff_layout`, `DaemonMsg::ReviewChanged`,
`OperationResult::ReviewSent`; `version.rs` 29 → 30. `crates/micold-core/src/settings.rs`:
`DiffLayout` and `Settings::diff_layout` (serde default).

### D3 — Daemon — R3–R6, W1–W12

- `crates/micold-daemon/src/review.rs` (new): `Reviews` state (per project, per entry
  `EntryState`), load on catalog adoption, `apply_edit`, `send` (snapshot → target → deliver →
  finish/abort), `forget_worktree`, `forget_project`; writes through the catalog.
- `crates/micold-daemon/src/ops.rs`: move `deliver_first_prompt` and the trust/bracketed checks
  out of `mcp/tools.rs` into shared `ops::type_submission(state, session, text) -> Result<(),
  Undelivered>` and `ops::create_session_with_prompt(...)`; `mcp/tools.rs` calls them unchanged in
  behaviour (its tests stay green). `ops::delete_worktree` calls `review::forget_worktree` on
  success.
- `crates/micold-daemon/src/state.rs`: per-session `last_active: Uptime`, set on start,
  `SessionInput`, activity change; `running_sessions_in(project, location)`; push
  `ReviewChanged` after `Attached`.
- `crates/micold-daemon/src/server.rs`: route the two messages; `catalog.rs`: `diff_layout` setting
  and the reviews file write (temp + rename).

### D4 — Client render-free state — data-model "Client"

`crates/micold-client/src/features/changes.rs` (new): `State`, `Msg`, `Effect` (`ReadList { seq }`,
`ReadDiff { seq, path, force_large }`, `Send(ClientMsg)`), reducers for open/close, toggles,
select, list/diff answers with stale-seq dropping and coalescing, large-file gate, pick and
composer, review pushes, send enable rule (S1), refresh request from the watcher. Wired in
`app.rs` as `Message::Changes`; selecting a session or the entry leaving the catalog closes it.
`features/sidebar.rs`: the **Review changes** menu message for worktree and Default rows.

### D5 — Client shell — R2, R9, R10

`crates/micold-client/src/shell/changes.rs` (new): runs `Effect::ReadList`/`ReadDiff` in
`spawn_blocking` via core `review::git`, highlights both sides with `iced::highlighter` into
`Spans`, returns `Msg::ListRead`/`Msg::DiffRead`. `shell/changes_watch.rs` (new): a `Subscription`
keyed by entry root, `notify::RecommendedWatcher`, 300 ms debounce, `review::watch` filter,
`git check-ignore`, emits `Msg::Changed`. Registered in `shell/subscriptions.rs` only while the view
is open. `crates/micold-client/Cargo.toml`: `notify = { workspace = true }`; workspace `iced`
features add `"highlighter"`.

### D6 — Shared components — R11, R15, contracts/changes-view.md

`crates/micold-client/src/ui/material/` (new files, builder API, `impl From<_> for Element`):
`virtual_rows.rs` (`VirtualRows::new(len, row_height, build_row).on_scroll(..)`, pure
`visible_range`), `diff_view.rs` (`DiffView::new(rows, layout, roles).spans(..).pick(..)
.on_gutter(..).slot(row, element)`), `review_comment.rs` (`ReviewCommentCard::new(text, state,
roles).outdated(b).on_edit(..).on_delete(..)`), `text_area.rs` (`TextArea::new(content, roles)
.on_action(..).placeholder(..)`). Diff row tints: new roles in core `tokens` (`diff_added`,
`diff_removed` container tints per scheme) added to `Roles` in `tokens/mod.rs` and to the role list
in `tokens/css.rs` (the CSS export and its token tests cover them), checked by
`composition_contrast.rs`.

### D7 — Glue and docs

`crates/micold-client/src/ui/changes.rs` (new) composes the view; `ui/mod.rs` shows it in place of
the terminal pane when open; `ui/sidebar.rs` adds the menu items. Showcase poses in
`showcase/sections/` + `catalogue.rs`. User guide per Constitution row VII.

## Requirement map

| FR | Where | Research / contract |
|---|---|---|
| FR-001 | D4, D7 menu items | changes-view V1 |
| FR-002 | D1 `changes.rs`, D6 file rows | R1; V L2 |
| FR-003 | D1 `base.rs`, D7 header | R7; V2, L1 |
| FR-004 | D1 `DiffRange::for_view`, D4 toggles | R1; L1 |
| FR-005 | D1 `git.rs` (`HEAD`→worktree + `ls-files --others --exclude-standard`) | R1 |
| FR-006 | D1 rows, D6 `DiffView`, D2 setting | R12; D1 |
| FR-007 | D5 highlighting, D6 tints | R10; D2 |
| FR-008 | D1 `classify`, `FileDiff` | R8; D3 |
| FR-009 | D4 large gate, D5 off-thread, D6 `VirtualRows` | R2, R8, R11; D4, D5 |
| FR-010 | D5 watcher, D4 refresh | R9; R1–R2 |
| FR-011 | D4 pick/composer, D2 `ReviewEditOp`, D1 `EntryReview` | W1, W3; C1–C3 |
| FR-012 | D1 `ReviewComment` | data-model; W1 |
| FR-013 | D1 `is_outdated`, prompt suffix | R14; review-prompt |
| FR-014 | D1 `begin_send`, D3 send | R6; W6 |
| FR-015 | D1 `prompt.rs` | R13; review-prompt |
| FR-016 | D3 send target + `ops` reuse | R4, R5; W7 |
| FR-017 | D1 `finish_send`/`abort_send`, D3 | R4, R6; W8, W9 |
| FR-018 | D3 `sending`, D4 enable rule | R6; W6, S1 |
| FR-019 | D1 `clear_sent`/`discard_pending`, D7 confirm | W4; S3 |
| FR-020 | D1 `store.rs`, D3 persistence, delete hook | R3; W5, W11 |
| FR-021 | D3 location check and target scope | W2, W7 |
| FR-022 | D1 entirely in core, tests below | R1 |
| FR-023 | D7 user guide | — |

## Test strategy by layer

| Layer | Where | What it proves | FR / SC |
|---|---|---|---|
| Core unit | `review/changes.rs` | numstat/name-status parsing incl. renames, mode-only, binary `-`, paths with spaces/UTF-8 via `-z`; untracked merge; one entry per path; large classification at 5,000/5,001 lines and 2 MB | FR-002, 005, 008, 009 |
| Core unit | `review/diff.rs` | hunk parsing, CRLF stripped, LF→CRLF-only change shown, no-newline marker, binary notice, untracked synth, NUL → binary, invalid UTF-8 → NotUtf8; unified and side-by-side give the same changed lines with both numbers | FR-006, 008, Edge |
| Core unit | `review/base.rs` | default branch order; `DiffRange` for every toggle × scope; Default has no committed range | FR-003, 004 |
| Core unit | `review/comment.rs` | add/edit/delete/clear/discard; `LineRange` rejects reversed; begin/finish/abort send; Busy, InSend, NothingPending; comments added mid-send stay pending; outdated detection | FR-011–014, 017–019 |
| Core unit | `review/prompt.rs` | P1–P10 of review-prompt.md | FR-015, SC-002, SC-006 |
| Core unit | `review/target.rs`, `review/watch.rs`, `review/store.rs` | pick latest, ties, empty; event filter drops `.git/objects`, nested worktrees for Default; JSON round-trip, missing and corrupt files | FR-016, 010, 020 |
| Core integration (real git) | `crates/micold-core/tests/review_git.rs` (new) | temp repo + worktree: committed/uncommitted/both lists, untracked not ignored, rename, binary, merge-base with `origin/HEAD`/`main`/`master`, no common history, Default root uncommitted only | FR-002–005, Edge No base |
| Daemon integration | `crates/micold-daemon/tests/review_send.rs` (new, fake CLI as `mcp_create_session.rs`) | edit validation W1–W5; send to running most-recent session; start-and-send when none; failure keeps pending; concurrent sends from two clients deliver once; no bracketed paste refused; Default entry and isolation (a prompt never reaches another entry's session); worktree delete removes comments; restart reloads comments | FR-014, 016–018, 020, 021, Isolation gate |
| Daemon regression | existing `mcp_create_session.rs`, `mcp_cross_session.rs` | the moved delivery functions behave as before | — |
| Client state | `crates/micold-client/tests/features_changes.rs` (new) | open/close, toggles, stale answers dropped, coalesced refresh, selection kept, large gate, pick across sides refused, composer survives refresh, send enabled iff pending and not sending, review push applied per entry | FR-001, 004, 009–011, 018, Edge |
| Client state | `crates/micold-client/tests/features_settings.rs` | `diff_layout` round-trips through `SettingsSet`/`SettingsChanged` | FR-006 |
| Geometry gates | `ui/material/virtual_rows.rs`, `diff_view.rs`, `review_comment.rs`, `text_area.rs` tests | `visible_range` arithmetic; only visible rows built; both layouts align numbers and columns; slot rows keep heights; card and text area anatomy | FR-006, 009, SC-003 |
| Contrast gate | `ui/material/composition_contrast.rs` | added/removed tints vs text and syntax colours, 4.5:1, both schemes | FR-007, US1 s8 |
| Visual pass | quickstart §B (visual-pass skill) | B1–B18 | all US, SC-001, 003, 004 |
| Platforms | CI on three OSes; quickstart §C | watcher, paths, CRLF, prompt bytes | Principle VI, SC-006 |
| Docs | `mise run gate` docs checks; user-guide gate | user guide page | FR-023 |

## Project Structure

### Documentation (this feature)

```text
specs/482-worktree-changes-review/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── review-wire.md
│   ├── review-prompt.md
│   └── changes-view.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── review/{mod,base,changes,diff,comment,prompt,target,watch,git,store}.rs   # new
├── protocol/{messages,version}.rs     # v30 messages
├── settings.rs                        # DiffLayout
├── store.rs                           # reviews_dir
└── tokens/{mod.rs,css.rs}             # diff_added / diff_removed roles
crates/micold-core/tests/review_git.rs # new
crates/micold-daemon/src/
├── review.rs                          # new
├── ops.rs                             # shared prompt delivery; delete hook
├── mcp/tools.rs                       # calls ops
├── state.rs, server.rs, catalog.rs
crates/micold-daemon/tests/review_send.rs  # new
crates/micold-client/src/
├── features/{changes.rs (new), sidebar.rs, mod.rs}
├── shell/{changes.rs, changes_watch.rs (new), subscriptions.rs}
├── ui/changes.rs                      # new glue
├── ui/{mod.rs, sidebar.rs}
├── ui/material/{virtual_rows,diff_view,review_comment,text_area}.rs  # new
├── ui/material/composition_contrast.rs
├── showcase/sections/, showcase/catalogue.rs
└── app.rs
crates/micold-client/tests/features_changes.rs  # new
docs/user-guide/reviewing-changes.md   # new; SUMMARY.md, README.md, worktrees-and-sessions.md
```

**Structure Decision**: existing workspace layout. Logic in `micold-core::review`, authority over
comments in the daemon, view state in the client's render-free `features/`, I/O in `shell/`,
components in `ui/material`, wiring in `ui/`.

## Complexity Tracking

None: no principle is violated. New dependencies (iced `highlighter` feature, `notify` in the
client) are justified in R9 and R10.
