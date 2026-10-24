# Research: Review UI — reviewed marks and progress (#1271)

Each decision records what was chosen, why, and what was rejected. Code paths are as of
`origin/main` at planning time; #1269 (Review tab, `State.open` becomes a map keyed by entry) is
not merged yet (R8).

## R1 What identifies a file's change: `ChangeId`

- **Decision**: a `ChangeId` is the SHA-1 digest (40 lowercase hex characters) of a canonical byte
  record of the file's change, built by pure functions in the new `micold-core::review::marks`:
  - **Working tree**: `ChangeId::of_worktree(&WholeFile)` over the file's kind tag (`A`, `M`, `D`,
    `R<from path>`, `T` for mode only), its path, and both sides' mode and git object id (an absent
    side is spelled as git spells it, all zeros). An untracked file is digested exactly as git's
    added file: tag `A`, old mode and id all zeros, new mode from disk, new id from
    `hash-object`. So `git add` and then committing an unaltered new file keep its id, and its
    mark (US2 s5); a separate untracked tag would change the id on every `git add`. The object ids come from
    git (R2), so two changes have the same id exactly when path, kind, modes and both sides'
    contents are the same: the spec's "every line of its diff on both sides identical", and also
    right for binary, not-UTF-8 and large files, whose lines are never read (Edge Cases).
  - **Pull request**: `ChangeId::of_patch(&ChangedFile, patch)` over the kind tag, the path and
    the file's own section of the pull request's diff (`PrDiffLoad::Ready.patches`, feature 775),
    which is already in memory when the pull request's diff is shown.
  - The record starts with a source tag (`w` or `p`) so the two derivations can never collide;
    marks are kept per source anyway (FR-006).
- **Rationale**: the id is small (enough to store per file and push to every window, Key Entities:
  "not the diff itself"), stable across platforms and builds (Principle VI: the record is bytes
  with `/` paths and no platform text), and cheap: the working tree needs no file's diff read.
- **Digest crate**: `sha1` 0.10 (RustCrypto, MIT OR Apache-2.0), already in `Cargo.lock` as a
  transitive dependency, added to `micold-core` as a direct one. No new crate enters the build.
  The digest guards against accidental equality, not an adversary (git itself relies on the same
  function for the object ids the record holds).
- **Alternatives rejected**:
  - *Hash every file's full diff text* (the literal spec wording): needs one `git diff -U3` of the
    whole tree on every re-read (feature 482's file watching re-reads within 2 s); for large and
    binary files the text is never read today (limits R8). Object ids give the same equality for
    the working tree without reading contents.
  - *Store the raw object ids instead of a digest*: works for the working tree, but a pull
    request's patch carries only abbreviated ids (GitHub's `index abc1234..def5678`), so the PR
    source would still need a digest; one type for both is simpler.
  - *`std::hash::DefaultHasher`*: its output is not stable across Rust releases, so a mark written
    by one build would not match the next build's id (SC-004).
  - *`sha2`*: not in the dependency tree; SHA-1 is enough for this use.

## R2 Reading the whole change, whatever the toggles

- **Decision**: a new read `GitCli::whole_change(dir, &ReviewScope) -> io::Result<WholeChange>`
  in `micold-core/src/review/git.rs`, with a matching `Git` trait method (and the fakes in
  `micold-core/src/git.rs`). It always reads the entry's whole change, independent of the
  Committed / Uncommitted toggles (FR-005):
  1. `git diff --raw --full-index` (with the R1 options of `DIFF`) over `<base>` for a worktree
     with a base, else over `HEAD` (the Default entry, or no base: their whole change is the
     uncommitted one, Edge Cases > Toggles);
  2. the committed and uncommitted path sets (`range_paths`, as `change_list` reads them) to give
     each file its `Origin` (Committed, Uncommitted, Both), which the D4 rule needs (R7);
  3. one `git hash-object --stdin-paths` for the files whose new id git printed as all zeros (the
     working-tree copy is not hashed in the index) and for untracked files (`untracked`), so every
     file has both ids. A path holding a newline cannot go through the pipe; that file gets no id
     (it cannot be marked), as `object_sizes` treats such paths.
  `parse_name_status` keeps parsing the same records; `NameStatus` gains the two modes and ids it
  already reads past (`old_mode`, `new_mode`, `old_id`, `new_id`) instead of a second parser.
- **When**: in the same background task as the view's list read (`shell/changes.rs::read_list`),
  so FR-004's "every time the view reads the list" holds by construction, including feature 482's
  watch re-reads and #1269's re-read when the tab is shown again. Its answer travels in the same
  `Msg::ListRead`. A failed whole-change read leaves the last one in place ("files keep the state
  last worked out") and, before any succeeded, no file has an id (marking unavailable).
- **Cost**: three `git diff --raw` reads (no contents) and one `hash-object` over the dirty files
  per re-read, beside the list read's own. Allowed later optimisation, not required: with both
  toggles on, `change_list`'s `BaseToWorktree` already reads the same raw records and origins.
- **Alternatives rejected**: deriving identity from the list read itself (wrong when a toggle is
  off: the list then covers only one kind, FR-005); reading only the selected file's identity
  (every row needs its state, FR-001, and *n* counts all files, FR-008).

## R3 Where marks are kept: beside the comments, in the same entry record

- **Decision**: `EntryReview` (`micold-core/src/review/comment.rs`) gains `marks: EntryMarks`; the
  daemon's `Reviews` (`micold-daemon/src/review.rs`) keeps holding one `EntryReview` per entry. An
  entry is empty only with no comments **and** no marks (`EntryReview::is_empty`), so an entry
  with marks only is kept, written and pushed on attach.
- **Rationale**: FR-010 asks for marks to be kept, pushed and removed exactly as comments are. The
  existing paths then cover marks with no new code path: `pushes_on_attach`, `apply_edit`'s write,
  `forget_worktree`, `prune_worktrees` (worktree removed outside the app) and `forget_project`.
  None of `clear_sent`, `discard_pending`, `begin_send`, `finish_send`, `abort_send`, `report`,
  `mark` (Mark applied) touch `marks` (FR-007, FR-016).
- **Alternatives rejected**: a separate `marks/<project id>.json` and a second map in the daemon
  (two writes per project, removal duplicated in four places, and the two files could disagree
  after a crash); marks kept by the client (FR-010, FR-011 need the service).

## R4 Wire: one new edit, one new field on the existing push

- **Decision**: `ReviewEditOp::SetReviewed { scope: Scope, path: String, change: Option<ChangeId> }`
  sets the mark of `path` in `scope` to `change`, or removes it with `None`. The answer is `Ack`,
  and the new state goes to every attached window as `DaemonMsg::ReviewChanged`, which gains
  `marks: Vec<ReviewMark>` (the entry's marks of every source). Contract: contracts/marks-wire.md.
- **Rationale**: one push carries comments and marks together, so a window never shows one newer
  than the other; `SetReviewed` is a *set*, not a toggle, so a repeat from another window is
  idempotent, and the daemon applies edits one at a time, so the last one wins and is pushed to all
  (FR-011, Edge Cases > Concurrent windows). An unchanged result writes and pushes nothing (as W19).
  Marking stays available while a send of the entry is open: a send freezes comments, not marks.
- **Alternatives rejected**: `Toggle { path }` (two windows pressing at once would cancel out and
  disagree with what each user meant); a separate `ReviewMarksChanged` push (a second attach push,
  a second ordering to keep, no gain); reusing the name `Mark` (already the 946 "Mark applied" op).

## R5 Review file version 4, written only with marks

- **Decision**: `micold-core/src/review/store.rs`: `REVIEW_FILE_VERSION` 4, written only while some
  entry has marks; otherwise 3 (some PR comment) or 2 as today. Reads 1 to 4. `StoredEntry` gains
  `marks` (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`); `ReviewFile` gains
  `marks: BTreeMap<String, Vec<ReviewMark>>` beside `entries`, and `to_json` writes an entry that
  has marks but no comments.
- **Rationale**: the spec's assumption, as 1149 did for PR comments: a file without marks stays
  readable by an older build (FR-012 reads older files unchanged); an older build that meets
  version 4 sets it aside as unknown, the existing behaviour.
- **Alternatives rejected**: always writing 4 (needlessly makes every file unreadable to an older
  build); adding `marks` without a version bump (an older build would load the file and silently
  drop the marks on its next write).

## R6 Older peers and an unreachable service

- **Decision**: one `PROTOCOL_VERSION` bump (the next free number when the wire milestone merges;
  68 to 69 at planning time) covering `SetReviewed`, `ReviewMark` and `ReviewChanged.marks`.
- **Finding**: the handshake (`protocol/handshake.rs`, `VersionMismatch`, plus `SCHEMA_HASH`)
  refuses a peer of another protocol, so an older window never attaches to a service that keeps
  marks, and a newer window never attaches to an older service. The spec's *Older window, newer
  service* edge case and FR-013's "too old to know them" are therefore met by the existing
  version-mismatch behaviour: the old window cannot change any mark, and nothing in this feature
  adds a mixed-version path. No capability flag is added.
- **Unreachable**: the client tells it from the project's link state (`Links::is_connected`, the
  same input `features::composer::unavailable(session, connected)` takes). The mark-control model
  takes `connected` as an argument; with `false`, marking is unavailable with the tooltip
  "The session service is not connected", rows show drafts only and no progress is shown (FR-013).
  Before the first `ReviewChanged` of an entry arrives the marks are treated as none, as comments.
- **Failure**: a refused or failed `SetReviewed` goes through a new `PendingOp::ReviewMark`, whose
  failure text "save the reviewed mark" reaches the user where other review edit failures do
  (`daemon_sync.rs`, `PendingOp::ReviewEdit`'s path). No optimistic update: the state changes only
  when `ReviewChanged` arrives, so a failure leaves it as it was (Edge Cases). The local round trip
  is well under SC-002's 0.5 s.

## R7 D4 (provisional): one rule, one place

- **Decision**: the provisional answer A (D4) is one pure function in `review::marks`:
  `mark_block(origin: Option<Origin>, toggles: Toggles, scope: &ReviewScope) -> Option<MarkBlock>`
  answering `Some(MarkBlock::HiddenKind)` when the file has changes of a kind a toggle hides. The
  client's `mark_control` calls it only for the **Mark reviewed** position; **Reviewed** (unmark)
  never consults it. Its tooltip text is one constant, `HIDDEN_KIND_TOOLTIP` ("Turn on Committed
  and Uncommitted to see this file's whole change"), and its tests are one group in
  `review/marks.rs` plus US1 s7 in the client reducer tests.
- **If the user picks B** (always markable): delete the `HiddenKind` arm, the constant and that
  test group; `WholeChange` keeps its origins (unused by marks) or drops them. Nothing else moves:
  identity is already toggle-independent (FR-005).
- **If the user picks C** (mark only the shown part): the mark must also record which kinds it
  covered and `whole_change` must yield one id per toggle set. That changes `ReviewMark` (data
  model), the store and the wire, so it must be decided before the core milestone (M1) is built.
- **Alternatives rejected**: spreading the check into the view (`ui/changes.rs`) or into the
  daemon: the daemon cannot know the client's toggles, and a view-only check is not unit-testable
  (Principle I).

## R8 Building before and after #1269

- **Decision**: the work is split so that everything below the client view can merge first:
  - **Before #1269**: `micold-core` (marks, ids, whole-change read, store), the wire, the daemon,
    and the client's receipt of marks: `Msg::ReviewChanged.marks` into `State.reviews` (keyed by
    project and entry, outside `State.open`, which #1269 replaces). None of this touches
    `State.open` or `OpenView`; the `Msg::ReviewChanged` arm, which #1269 also rewrites, only
    gains the `marks` binding and the field in the `ReviewView` insert after the `state.open`
    block (a trivial rebase conflict).
  - **After #1269**: the whole-change load kept with the list in `OpenView` (per view; #1269 keeps
    one `OpenView` per entry, so the field moves with it unchanged), `PendingOp::ReviewMark` with
    its only sender in `shell/changes.rs` (`shell` is private to the binary, so the variant alone
    would be dead code), the PR ids, the row state,
    the changed dot, the mark control, the progress in the view's header, the reducer tests, the
    layout snapshot state, the docs and the visual pass.
- **Rationale**: #1269 rewrites how `OpenView` is held and where the header is drawn; changes to
  those before it merges would conflict with it and be redone. Marks belong to the entry (spec
  Context), so the data path does not depend on the view being a tab.
- **If #1269 slips**: the spec's assumption holds: the same controls go into today's Changes view
  unchanged; the after-#1269 milestone then targets `State.open` and is rebased once #1269 lands.

## R9 UI built from existing components (Principle VIII)

- **Decision**:
  - **Row state**: a `Glyph` at the row's leading edge with three new `Icon` variants
    (`ReviewUnseen` ○, `ReviewDrafts` ●, `ReviewDone` ✓) in `crates/micold-client/src/icons.rs`,
    tinted through `icon_role` like the other state icons; the existing draft count `Tag` stays.
  - **Changed dot**: `UnreadMark::new(r)` (dot only) at the row's trailing edge inside a
    `Tooltip` ("Changed since you marked it reviewed"): it is the app's existing "new since you
    looked" mark, and #1275 later sums the same dots in the header.
  - **Mark control**: `ToggleChip::new(label, Msg::MarkToggled, r).active(reviewed).disabled(..)`
    in the diff header beside **Whole file**, wrapped in `Tooltip`; keyboard focus as the other
    chips (FR-002).
  - **Progress**: `Text` *n of m reviewed* and `ProgressBar::new(Ratio { done: n, total: m }, r)
    .width(Length::Fixed(120.0))` in the view's header, inside one fixed-width slot that is laid
    out even when empty, so the header's other controls never shift (FR-009).
- **Alternatives rejected**: a new "review state" widget (three glyphs need no component of their
  own); drawing the dot by hand (a fork of `UnreadMark`).
