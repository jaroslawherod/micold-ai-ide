---

description: "Task list for feature 946: review comments the agent answers — stable batch, replies, review panel, re-anchoring"
---

# Tasks: Review Comments the Agent Answers — Review Panel, Stable Batch, Replies and Re-anchoring

**Input**: Design documents from `specs/946-review-replies/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (batch-format,
agent-tool, review-wire-delta), quickstart.md. Extends feature 482 (`specs/482-worktree-changes-review/`).

**Tests**: Mandatory (Constitution I). Every test task comes before the implementation it covers and
must fail first for the right reason (TDD loop, cycle log in `specs/946-review-replies/tdd/cycle-log.md`).

**Documentation**: `docs/user-guide/reviewing-changes.md` and `docs/user-guide/agent-tools.md` grow
with each story, in the same milestone as the behaviour they describe (Constitution VII, CI
user-guide gate).

**Cross-platform**: batch, references, replies and re-anchoring are core logic with `\n` only and
`\r` normalised (FR-012, FR-025); the P1 golden test runs on all three CI platforms (Constitution VI).

**Wire**: every wire-visible change of this feature is made in **one** edit (T007), so
`PROTOCOL_VERSION` moves 48 → 49 once; a second bump later in the feature fails
`crates/micold-core/tests/schema_hash.rs` (see the doc comment in
`crates/micold-core/src/protocol/version.rs`). `ReviewEditOp::Mark` and `ReviewEditOp::Reanchor`
exist on the wire from M1 but the daemon refuses them (T009) until T040 (Mark, M3) and T063
(Reanchor, M5) serve them; no client sends them before those milestones.

**Renames**: use the `LSP` tool (`findReferences`) for the one symbol rename of this feature,
`CommentState::Pending` → `Draft` (T003), not `grep` alone. `ReviewError::NothingPending` and
`EntryReview::discard_pending` keep their names; only user-facing wording changes.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US4 from spec.md (US5 is deferred to epic #945 and has no tasks)

---

## Phase 1: Setup (Shared Infrastructure)

- [x] T001 Declare the two new core modules: `pub mod anchor;` and `pub mod reply;` in `crates/micold-core/src/review/mod.rs`, each with an empty file `crates/micold-core/src/review/anchor.rs` and `crates/micold-core/src/review/reply.rs` holding only a module doc comment (plan § Project Structure). No new crate (research header)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the four statuses, the comment reference, the version 2 review file and the feature's
whole wire delta in one edit. No user-visible behaviour beyond 482's yet: Reported comments cannot
be made until M2.

- [x] T002 [P] Write failing unit tests in `crates/micold-core/src/review/comment.rs`: `CommentState::{Draft, Sent { at }, Reported { sent_at, report }}` serialises exactly as data-model.md § store (`{ "draft": null }`, `{ "sent": { "at": … } }`, `{ "reported": { "sent_at", "outcome": "answered", "reply", "first_at", "last_at", "by": { "agent": "<uuid>" } } }`, `"by": "user"` for a user mark); `status()` maps to `Status::{Draft, Sent, Applied, Answered}` (FR-001); `ReviewComment::reference()` is `C-` + the first 8 lowercase hex digits of the id's UUID and at most 12 characters (FR-014); `CommentRef` parsing accepts the prefix `C-` or `c-`, upper or lower case hex and surrounding whitespace, and refuses anything else (`C-123`, `D-1a2b3c4d`, 9 hex digits, `C-1a2b3c4g`); `fresh_id(&EntryReview, draw: impl FnMut() -> CommentId)` returns the first drawn id whose reference no comment of the entry has, given a scripted `draw` whose first id collides (research R4); `Move` refuses `to.len() != from.len()`; `clear_sent` removes Sent and Reported comments and keeps Drafts (FR-019); `set_text` and `delete` on a Sent or a Reported comment are `Refused` (FR-002)
- [x] T003 Implement `CommentState` (replacing 482's `Pending | Sent`; rename `Pending` → `Draft`, `NothingPending` message "there are no draft comments to send"), `Status`, `Report { outcome, reply, first_at, last_at, by }`, `Outcome::{Applied, Answered}`, `Reporter::{Agent { session: Uuid }, User}`, `CommentRef`, `ReviewComment::reference()`, `Move { id, from, to }` `fresh_id` and the widened `clear_sent` in `crates/micold-core/src/review/comment.rs` until T002 passes. Carry the rename through `crates/micold-daemon/src/review.rs`, `crates/micold-daemon/src/state.rs`, their tests, `crates/micold-client/src/features/changes.rs` and `crates/micold-client/src/ui/changes.rs` so the workspace compiles; the client treats a Reported comment as a Sent one (card `CardState::Sent`, no edit or delete, counted as sent) until T030 derives its status. `Report` has no public constructor yet (it is built only by `EntryReview::report`/`mark`, T023/T039; research R1)
- [x] T004 [P] Write failing unit tests in `crates/micold-core/src/review/store.rs` (contracts/review-wire-delta.md S2): a version 1 file with `{ "pending": null }` and `{ "sent": { "at": … } }` loads as Draft and Sent; a version 2 file with all three states round-trips byte-equal; saving after loading a version 1 file writes `"version": 2`; a file of version 3 is quarantined as 482's `read_or_quarantine` does; two comments of one entry whose references collide load with the earlier one unchanged and the later one under a fresh id with a different reference (research R4); a v1 Sent comment has a reference after load
- [x] T005 Implement `REVIEW_FILE_VERSION = 2`, reading versions 1 and 2 (`pending` → Draft) and the collision re-id on load (through `fresh_id`, T003) in `crates/micold-core/src/review/store.rs` until T004 passes
- [x] T006 [P] Write failing wire tests in `crates/micold-core/tests/protocol_roundtrip.rs`: `DaemonMsg::ReviewChanged` carrying a comment in each of `Draft`, `Sent { at }` and `Reported { sent_at, report }` (both `Reporter` arms) round-trips; `ClientMsg::ReviewEdit` with `ReviewEditOp::Mark { id, outcome, note }` and `ReviewEditOp::Reanchor { moves }` round-trips (W13–W15)
- [x] T007 Make the feature's whole wire delta in one edit (contracts/review-wire-delta.md W13–W15): `ReviewEditOp::Mark { id: CommentId, outcome: Outcome, note: String }` and `ReviewEditOp::Reanchor { moves: Vec<Move> }` in `crates/micold-core/src/protocol/messages.rs` (`ReviewComment` already carries `CommentState`); `PROTOCOL_VERSION` 48 → 49 with its "Bumped 48 → 49" doc line in `crates/micold-core/src/protocol/version.rs`; the new pin in `crates/micold-core/tests/schema_hash.rs`; in `Reviews::apply_edit` (`crates/micold-daemon/src/review.rs`) the two new arms change nothing and answer as an edit that applied nothing, only so the workspace compiles (T009 replaces them). T006 passes
- [x] T008 [P] Write failing assertions in `crates/micold-daemon/tests/review_edit.rs`: `ReviewEdit` with `Mark` and with `Reanchor` is answered `OperationError { kind: Refused }` and stores nothing (placeholder; T040 retires the `Mark` assertion, T063 the `Reanchor` one)
- [x] T009 Answer `Mark` and `Reanchor` in `Reviews::apply_edit` in `crates/micold-daemon/src/review.rs` with `Refused` "this review action is not available in this build", storing nothing, until T008 passes. `mise run gate` green

**Checkpoint**: statuses, references, review file v2 and protocol v49 in place; behaviour is 482's.

---

## Phase 3: User Story 2 — A stable batch with one block per comment (Priority: P1) 🎯 MVP

**Goal**: **Send review** delivers every Draft comment as one documented, byte-stable message: a
fixed preamble and one self-contained block per comment, each with its reference.

**Independent Test**: spec US2 Independent Test (core, no window, no session); quickstart §B step 4.

**Why before US1**: both are P1; an agent can report on a comment only by the reference this batch
gives it (US1 depends on US2, never the reverse).

- [x] T010 [P] [US2] Replace 482's prompt tests with failing unit tests P1–P9 of contracts/batch-format.md in `crates/micold-core/src/review/prompt.rs`: P1 the contract's example, exact bytes (comments with UUIDs `00000001-…`, `00000002-…`, `00000003-…`, C-00000003 outdated, worktree entry), with a doc comment saying the bytes must change only together with "The review message" in `docs/user-guide/reviewing-changes.md` (FR-013; the test does not read `docs/`, research R6); P2 Default entry wording (`the project root` twice); P3 a 51-line quote elided to first line, `... 49 lines not shown ...`, last line, heading keeping `lines 1-51`, and a 50-line quote not elided; P4 a quote containing ```` ``` ```` gets a four-backtick fence; P5 CRLF and lone `\r` in quote and text: no `\r` in the output; P6 ordering ties (same path and start: current before removed; same side: earlier creation first, then id); P7 20 comments across 5 files: 20 `### Comment C-` headings, 20 distinct references, each text exactly once (SC-002); P8 built twice → equal bytes; P9 the preamble contains `REPORT_TOOL` and `REPORT_TOOL == "report_review_comment"`. Also: no trailing line break; only the comments passed in (the caller passes Drafts)
- [x] T011 [P] [US2] Write failing unit tests in `crates/micold-core/src/review/comment.rs`: `EntryReview::add` refuses with `RefTaken` an id whose reference another comment of the entry already has, and stores nothing (FR-014)
- [x] T012 [P] [US2] Extend the failing daemon test `crates/micold-daemon/tests/review_send.rs`: with a running session in the worktree, two Drafts and one Sent comment, `ReviewSend` types exactly one submission equal to `prompt::build` of the two Drafts, holding one `### Comment <ref>:` heading per Draft with that comment's `reference()` and none for the Sent one (US2 s1, FR-009); the two become `Sent`; a refused delivery leaves both Draft and reports why (US2 s6, FR-015, as 482 W9)
- [x] T013 [P] [US2] Write failing client state tests in `crates/micold-client/tests/features_changes.rs`: the toolbar button reads "Send review (n)" for `n` Drafts and "Sending…" while a send is open (FR-009 rename of 482 S1), and the S2 snackbar texts name "Send review"
- [x] T014 [US2] Implement the new `prompt::build(entry, comments, outdated)` layout of contracts/batch-format.md (preamble paragraphs one line each, `### Comment <ref>: <path>, <lines> <side>`, outdated note line, fence one backtick longer than the longest run and at least three, elision over 50 lines, `\n` joins, no trailing break) and `pub const REPORT_TOOL: &str = "report_review_comment"` in `crates/micold-core/src/review/prompt.rs` until T010 passes
- [x] T015 [US2] Implement `RefTaken` in `EntryReview::add` in `crates/micold-core/src/review/comment.rs`, and make `Reviews::apply_edit`'s `Add` arm in `crates/micold-daemon/src/review.rs` draw its id with `fresh_id(entry, CommentId::new_v4)`, until T011 passes
- [x] T016 [US2] Make `review_send` in `crates/micold-daemon/src/state.rs` / `Reviews::begin_send` in `crates/micold-daemon/src/review.rs` snapshot only Draft comments and build with the new `prompt::build` until T012 passes; the batch text is never logged (482 W12)
- [x] T017 [US2] Rename the button and snackbars to **Send review** in `crates/micold-client/src/features/changes.rs` (`send_button` label) and `crates/micold-client/src/ui/changes.rs` until T013 passes
- [x] T018 [P] [US2] Rewrite "Sending comments to the session" in `docs/user-guide/reviewing-changes.md` as "Sending a review": the **Send review** button, that only drafts are sent, and a new section "The review message" carrying contracts/batch-format.md's example verbatim and a field-by-field description (reference, path, lines, side, outdated note, fence, elision, comment text, order) (FR-013, FR-028); replace every "Send to session" and "pending" comment wording with "Send review" and "draft"

**Checkpoint (M1)**: Send review types the documented batch with references; the agent can answer
in its reply text (the preamble's fallback) until M2 adds the tool.

---

## Phase 4: User Story 1 — The agent answers each comment, and the answer shows next to it (Priority: P1)

**Goal**: the entry's session reports each comment applied or answered through
`report_review_comment`; cards show the status and reply in every window and after a restart; the
user can mark a Sent comment by hand and read a full reply.

**Independent Test**: spec US1 Independent Test; quickstart §B steps 5–7 and 10.

### Part A — agent reports, shown on the cards (scenarios 1–5, 7, 8)

- [x] T019 [P] [US1] Write failing unit tests in `crates/micold-core/src/review/comment.rs` for `EntryReview::report(reference, outcome, reply, by, at)`: from Sent → `Reported { sent_at }` keeping the send time, `first_at == last_at == at`; a second report replaces outcome, reply and `by`, keeps `first_at`, moves `last_at` (FR-005); `applied` with an empty reply is accepted; the reply is `\r\n`/`\r` → `\n` and trimmed; refusals, each changing nothing: unknown reference `NotFound`, a Draft `Draft`, a comment in the open send `InSend`, `answered` with a blank reply `Invalid`, a reply over 20,000 characters `Invalid` (exactly 20,000 accepted) (FR-003, FR-004); returns the previous status
- [x] T020 [P] [US1] Write failing unit tests in `crates/micold-core/src/review/reply.rs`: `display` keeps `\n`, replaces each other C0 control character with U+2400 + its code (ESC → `␛`, TAB → `␉`) and DEL with `␡`, leaves other text unchanged; `shortened(reply, 4_000)` returns the text unchanged and `false` at 4,000 characters, the first 4,000 characters and `true` at 4,001, never splitting a multi-byte character (Edge Cases "very long reply", "control characters")
- [x] T021 [P] [US1] Write failing core tests for the tool (contracts/agent-tool.md): in `crates/micold-core/tests/mcp_tools_catalog.rs`, `tools/list` lists `report_review_comment` right after `send_session_input` with the contract's input schema, description text, `readOnlyHint: false`, `destructiveHint: false`, and its name equals `review::prompt::REPORT_TOOL` (P9); parsing gives `Operation::ReportReviewComment { comment, outcome, reply }` and refuses with `invalid_input` and the contract's messages checks 1–4 (missing or malformed `comment`, unknown `outcome`, `answered` with a missing or blank `reply`, `reply` over 20,000 characters); `audit_target()` is the reference and never contains the reply; in `crates/micold-core/tests/mcp_policy.rs`, `decide` returns `Proceed` with no confirmation for a worktree caller and for a Default caller (FR-003, D4, D5)
- [x] T022 [P] [US1] Write the failing daemon integration test `crates/micold-daemon/tests/review_reports.rs` (new; fake CLI and MCP client as `crates/micold-daemon/tests/mcp_read_tools.rs`): after a delivered send, a session of the worktree reports the first comment `applied` with "Renamed as asked" → result `{ comment, status: "applied", file, lines, previous_status: "sent" }`, `ReviewChanged` with the Reported state reaches two attached clients, and the state is back after a daemon restart (US1 s1, s7, FR-007, FR-008); `answered` with a blank reply is refused and the comment stays Sent (s3); a session of another worktree gets `refused_by_policy` (5a); a Default session reports on the Default entry's comment (D5); a cleared comment is `not_found` (5b); a comment inside a held-open send is `conflict` "being sent" (6); a Draft is `conflict` "still a draft" (7); a re-report keeps `first_at` (s5); the audit log line names the reference and not the reply; no confirmation request is raised; the second client receives the push within 2 s of the tool result (FR-007, SC-005)
- [x] T023 [US1] Implement `EntryReview::report` in `crates/micold-core/src/review/comment.rs` until T019 passes (the only builder of an agent `Report`)
- [x] T024 [US1] Implement `reply::display` and `reply::shortened` in `crates/micold-core/src/review/reply.rs` until T020 passes
- [x] T025 [US1] Implement `Operation::ReportReviewComment`, its catalog entry (schema, description and annotations from contracts/agent-tool.md, name from `REPORT_TOOL`), parsing with checks 1–4, `tool_name()` and `audit_target()` in `crates/micold-core/src/mcp/tools.rs`, and its `Proceed` arm in `crates/micold-core/src/mcp/policy.rs`, until T021 passes
- [x] T026 [US1] Implement `Reviews::report(catalog, project, caller_location, reference, outcome, reply, session, at)` in `crates/micold-daemon/src/review.rs` (lookup in the caller's entry, else the project's other entries for 5a, else 5b; write then push `ReviewChanged`, contracts/review-wire-delta.md W17) and the `report_review_comment` handler in `crates/micold-daemon/src/mcp/tools.rs` (caller's `SessionLocation` as `whoami` resolves it; `ReviewError` → error categories in the contract's order; success JSON with `previous_status`; reply never logged) until T022 passes
- [x] T027 [P] [US1] Write failing geometry tests in `crates/micold-client/src/ui/material/review_comment.rs`: `CardState::{Applied, Answered}` draw their status tag; `.reply(text, shortened)` draws the reply under the comment text, with a "Show all" affordance only when `shortened`; neither state draws Edit or Delete
- [x] T028 [P] [US1] Write failing client state tests in `crates/micold-client/tests/features_changes.rs`: a pushed Reported comment's card model has status Applied or Answered, its reply passed through `reply::display` and `reply::shortened(.., 4_000)`, and no edit or delete (FR-002, FR-007); a reply of 4,001 characters is marked shortened; a second window's state applies the same push (US1 s1)
- [x] T029 [US1] Implement `CardState::{Applied, Answered}` and `.reply(..)` on `ReviewCommentCard` in `crates/micold-client/src/ui/material/review_comment.rs` until T027 passes
- [x] T030 [US1] Derive each card's status and reply in `crates/micold-client/src/features/changes.rs` (replacing T003's Reported-as-Sent mapping) until T028 passes
- [x] T031 [US1] Draw Applied and Answered cards with their replies in `crates/micold-client/src/ui/changes.rs` (glue, quickstart §B step 5)
- [x] T032 [P] [US1] Add Applied and Answered `ReviewCommentCard` poses (short reply, shortened reply) to `crates/micold-client/src/showcase/sections/review.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [x] T033 [P] [US1] Document `report_review_comment` in `docs/user-guide/agent-tools.md` (arguments, outcomes, own-entry rule including the Default entry, refusals, that it never waits for approval) and add "Statuses and replies" to `docs/user-guide/reviewing-changes.md` (Draft, Sent, Applied, Answered; replies on cards; kept across restarts; a session without the tool server cannot report) (FR-028)
- [ ] T034 [US1] Visual pass of quickstart §B steps 4–6 and 10, light and dark, with the `visual-pass` skill; evidence under `specs/946-review-replies/visual-pass/` — **not run in M2**: the cloud container has no xdotool/Xvfb driver; recorded in PR and ledger follow-ups

### Part B — marking by hand and reading a full reply (scenario 6, FR-006)

- [x] T035 [P] [US1] Write failing unit tests in `crates/micold-core/src/review/comment.rs` for `EntryReview::mark(id, outcome, note, at)`: on a Sent comment → `Reported { by: User }`, `first_at == last_at == at`; `applied` with an empty note accepted; refusals, each changing nothing: unknown id `NotFound`, a Draft or a Reported comment `Refused` (research R9), a comment in the open send `InSend`, `answered` with a blank note or a note over 20,000 characters `Invalid`; the note is `\r`-normalised and trimmed; a table-driven check that no operation (`finish_send`, `report`, `mark`, `clear_sent`, `set_text`, `delete`) turns a Sent or Reported comment back into a Draft (FR-001)
- [x] T036 [P] [US1] Replace T008's `Mark` placeholder assertion in `crates/micold-daemon/tests/review_edit.rs` with failing W14 tests: unknown id `NotFound`; during a held-open send `Busy`; on a Draft `Refused`; blank answered note or over limit `InvalidInput`; success writes the file before `OperationOk` and pushes `ReviewChanged` to a second client
- [x] T037 [P] [US1] Write failing client state tests in `crates/micold-client/tests/features_changes.rs`: a Sent card offers **Mark applied** and **Answer…**, Draft and Reported cards offer neither; **Mark applied** sends `ReviewEdit::Mark { outcome: Applied, note: "" }`; **Answer…** opens the composer with `ComposerTarget::Answer(id)`, Save is disabled while the note is blank and sends `Mark { outcome: Answered, note }`; **Show all** on a shortened reply sets `reply_open` and the dialog text is `reply::display` of the full reply; closing clears it; `reply_open` is cleared when that comment leaves the entry
- [x] T038 [P] [US1] Write failing geometry tests in `crates/micold-client/src/ui/material/review_comment.rs`: `.on_mark_applied(..)` and `.on_answer(..)` draw two actions on a Sent card and nothing on other states; `.on_show_reply(..)` makes "Show all" pressable
- [x] T039 [US1] Implement `EntryReview::mark` in `crates/micold-core/src/review/comment.rs` until T035 passes
- [x] T040 [US1] Serve `ReviewEditOp::Mark` in `Reviews::apply_edit` in `crates/micold-daemon/src/review.rs` (replacing T009's placeholder for `Mark`; `ReviewError` → `NotFound`/`Busy`/`Refused`/`InvalidInput` per W14) until T036 passes
- [x] T041 [US1] Add `ComposerTarget::Answer(CommentId)`, `reply_open` and the mark/answer/show-reply messages to `crates/micold-client/src/features/changes.rs` until T037 passes
- [x] T042 [US1] Implement `.on_mark_applied`, `.on_answer` and `.on_show_reply` on `ReviewCommentCard` in `crates/micold-client/src/ui/material/review_comment.rs` until T038 passes
- [x] T043 [US1] Wire the mark actions, the Answer composer under the card, and the full-reply dialog (built with `crate::ui::material::dialog` as `discard_dialog` is) in `crates/micold-client/src/ui/changes.rs`
- [x] T044 [P] [US1] Add the Sent-with-mark-actions card pose and the full-reply dialog pose to `crates/micold-client/src/showcase/sections/review.rs`
- [x] T045 [P] [US1] Add "Marking a comment by hand" and reading a full reply to `docs/user-guide/reviewing-changes.md` (FR-028)
- [ ] T046 [US1] Visual pass of quickstart §B step 7 and the full-reply dialog, light and dark; evidence under `specs/946-review-replies/visual-pass/` — **not run in M3**: the cloud container has no xdotool/Xvfb driver; recorded in PR and ledger follow-ups

**Checkpoint (M2, M3)**: the review round trip comes back answered, by the agent or by hand.

---

## Phase 5: User Story 3 — One review panel for all of an entry's comments (Priority: P2)

**Goal**: a panel right of the diff lists every comment of the entry with status, text and reply
start; rows jump, edit and delete drafts; Send review, Clear sent and Discard drafts sit with it.

**Independent Test**: spec US3 Independent Test; quickstart §B steps 1–3 and 9.

- [x] T047 [P] [US3] Write failing geometry tests in `crates/micold-client/src/ui/material/review_panel_row.rs` (new): `ReviewPanelRow::new(lines_label, text, status, roles)` has one fixed height for every state (so `VirtualRows` can window it); `.reply(..)` draws the reply's start on one clamped line (`line_clamp`); `.on_edit(..)`/`.on_delete(..)` draw two actions; `.selected(true)` draws the selected state layer; a status tag per `Status`
- [x] T048 [P] [US3] Write failing geometry tests in `crates/micold-client/src/ui/material/review_comment.rs`: `.highlighted(true)` draws the highlighted outline, `false` matches the plain card
- [x] T049 [P] [US3] Write failing client state tests in `crates/micold-client/tests/features_changes.rs`: `panel_rows` groups by path (bytewise), then start line, side (current first), created (FR-016); each row has its lines and side label (from the comment's stored range; T064 switches it to the anchor), the start of its text, its status and the start of its reply; a row whose file is not in the current diff carries the "not in the current diff" mark and keeps its status (Edge Case "file gone"); `status_counts` per status; an entry with no comments yields the empty state text (US3 s5); the panel is open by default and the toolbar toggle "Review (n)" closes and reopens it; selecting a row gives `Jump::Shown { path, line }`, selects that file, scrolls its lines into view and sets `panel.focus` so its card is highlighted (FR-017); a comment whose kind is toggled off gives `Jump::Hidden(Why::KindToggledOff)` and one whose file is not listed `Jump::Hidden(Why::NotListed)`, each with its message and no jump; Edit on a Draft row opens the composer `Edit(id)` with its text and Delete sends `ReviewEdit::Delete`; Sent, Applied and Answered rows offer neither, and a row with a reply opens `reply_open` (FR-018, US3 s4); Send review, Clear sent and Discard drafts from the panel follow the toolbar's availability (FR-019); a push from another window changes the rows (US3 s6); building rows and counts for 200 comments stays within the 100 ms budget, asserted as the client's existing timing tests do (SC-006)
- [x] T050 [US3] Implement `ReviewPanelRow` (builder, Principle VIII) in `crates/micold-client/src/ui/material/review_panel_row.rs` and register it in `crates/micold-client/src/ui/material/mod.rs` until T047 passes
- [x] T051 [US3] Implement `.highlighted(..)` on `ReviewCommentCard` in `crates/micold-client/src/ui/material/review_comment.rs` until T048 passes
- [x] T052 [US3] Add `Panel { open, focus, offset, viewport }`, `PanelRow`, `panel_rows`, `status_counts`, `Jump`, `Why` and the panel messages to `crates/micold-client/src/features/changes.rs` until T049 passes
- [x] T053 [US3] Render the panel as the Changes view's third pane, right of the diff, in `crates/micold-client/src/ui/changes.rs`: rows through `VirtualRows`, group headers by file, status counts, the empty state, the "Review (n)" toolbar toggle, Send review / Clear sent / Discard drafts in the panel header, the hidden-reason message, and the focused card drawn `.highlighted(true)` (glue, research R8)
- [x] T054 [P] [US3] Add `ReviewPanelRow` poses (each status, with and without reply, selected) and a highlighted card pose to `crates/micold-client/src/showcase/sections/review.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [x] T055 [P] [US3] Add "The review panel" to `docs/user-guide/reviewing-changes.md`: what a row shows, counts, jumping and why a jump can be refused, editing and deleting drafts from it, the panel's buttons, collapsing it (FR-028)
- [ ] T056 [US3] Visual pass of quickstart §B steps 1–3 and 9, light and dark; evidence under `specs/946-review-replies/visual-pass/` — **not run in M4**: the cloud container has no xdotool/Xvfb driver (see `visual-pass/m4.md`); follow-up for a run with a display

**Checkpoint (M4)**: one place shows the whole review and its state.

---

## Phase 6: User Story 4 — Comments follow their code after the agent edits (Priority: P2)

**Goal**: a current-side comment moves with its quoted code; only gone or ambiguous code marks a
Draft or Sent comment outdated; the next batch names the new lines.

**Independent Test**: spec US4 Independent Test (core, no window, no session); quickstart §B step 8.

- [x] T057 [P] [US4] Write failing unit tests in `crates/micold-core/src/review/anchor.rs` for `locate(quote, last, lines) -> Located` and `Located::shown(last, len)` (research R2): a comment on 10–12 → `Same` when unchanged; 5 lines inserted above → `Moved(15..=17)` (US4 s1); 3 deleted above → `Moved(7..=9)` (s2); one of its lines changed → `Gone`, `shown` = last place clamped into `1..=max(len, 1)` (s3), including a file now shorter than the range; the quote at two places → the nearest to `last.start()` (s4); equally near above and below → `Ambiguous`; a restored quote after `Gone` → `Moved` (s6); lines differing only by a trailing `\r` on either side are equal, and a CRLF-only change gives `Same` (FR-025); `locate` for 200 comments against a 5,000-line file stays within a 100 ms budget, asserted release-only as the core's budget tests do (BUG-001: ignored in a debug build, run by CI's release step) (`crates/micold-core/tests/typeahead_budget.rs`) (SC-006, send path); `is_outdated` true only for `Gone`/`Ambiguous` with status Draft or Sent, false for Applied and Answered (FR-021, FR-024); SC-004: a deterministic loop over insert and delete offsets and sizes around a quote never yields `Same`/`Moved` onto lines that do not equal the quote (no new crate)
- [x] T058 [P] [US4] Write failing unit tests in `crates/micold-core/src/review/comment.rs` for `EntryReview::reanchor(moves) -> usize`: a move applies only while the comment's range equals `from` and `to.len()` equals the quote's length; non-matching moves are skipped; the same moves applied twice apply once (second returns 0); quote, text and state never change (FR-023); applies to a comment in the open send
- [x] T059 [P] [US4] Replace T008's `Reanchor` placeholder assertion in `crates/micold-daemon/tests/review_edit.rs` with failing W15 tests: `Ack` always; a matching move is written and pushed; the same `Reanchor` from two clients applies once and pushes once; nothing applied → no write and no push. Extend `crates/micold-daemon/tests/review_send.rs` with W16: a Draft whose quoted lines moved down 5 on disk is sent with its new `lines …` heading and stored at the new range (US4 s1, FR-023); a Draft whose code is gone gets the outdated note even when the client's `outdated` list is empty; a removed-side Draft keeps the client's `outdated` verdict (FR-022)
- [x] T060 [P] [US4] Write failing client state tests in `crates/micold-client/tests/features_changes.rs`: after a diff read, `OpenView::anchors` (replacing `outdated`) holds each comment's `Anchor { shown, outdated }`: moved comments drawn at the new lines and not outdated; a Draft or Sent comment whose code is gone outdated at its clamped last place; an Applied or Answered one at its last place not outdated (US4 s7, FR-024); a removed-side comment keeps 482's verdict (s5); one `ReviewEdit::Reanchor` with the moves is sent only when the shown new side is the working tree (Default entry, or Uncommitted on) and never for a pull request's diff or when nothing moved (research R3); `panel_rows`' lines label and `Jump::Shown.line` use `anchors[id].shown`, so a moved comment's row and card name the same lines; `ReviewSend`'s `outdated` lists removed-side ids only; the composer still follows its lines across a refresh (s8, as 482)
- [x] T061 [US4] Implement `locate`, `Located` and `is_outdated` in `crates/micold-core/src/review/anchor.rs` (first-line prefilter, O(lines × n) worst case) until T057 passes
- [x] T062 [US4] Implement `EntryReview::reanchor` in `crates/micold-core/src/review/comment.rs` until T058 passes
- [x] T063 [US4] Serve `ReviewEditOp::Reanchor` in `Reviews::apply_edit` in `crates/micold-daemon/src/review.rs` (replacing T009's placeholder; allowed during an open send) and re-anchor at send in `review_send` in `crates/micold-daemon/src/state.rs`: read each Draft current-side comment's working-tree file on the blocking pool outside the state lock (`SideLines::from_bytes`; missing or non-text → gone), apply `locate`, store moves as W15, judge their outdated note, use the client's ids only for removed-side comments; until T059 passes
- [x] T064 [US4] Replace `outdated`/`judge_outdated` with `anchors` and the `Reanchor` effect, and take `panel_rows`' lines and `Jump::Shown.line` from `anchors`, in `crates/micold-client/src/features/changes.rs` until T060 passes
- [x] T065 [US4] Draw cards at `Anchor::shown` with the outdated mark from `Anchor::outdated` in `crates/micold-client/src/ui/changes.rs` (glue)
- [x] T066 [P] [US4] Replace "Outdated comments" in `docs/user-guide/reviewing-changes.md` with "How comments follow their code": moving with the code, nearest occurrence, when a comment is outdated, removed-side comments, Applied/Answered comments never outdated, the next batch naming the new lines (FR-028)
- [ ] T067 [US4] Visual pass of quickstart §B step 8, light and dark; evidence under `specs/946-review-replies/visual-pass/` — **not run in M5**: the container has no xdotool/Xvfb driver; recorded in PR and ledger follow-ups

**Checkpoint (M5)**: comments stay on their code across the agent's edits.

---

## Phase 7: Polish & Cross-Cutting Concerns

No code: left to the close unit (milestones.md rule 4).

- [ ] T068 Run quickstart §B steps 1–10 end to end, light and dark, and record the results and screenshots of steps 2, 5, 8 and 9 under `specs/946-review-replies/visual-pass/` — **not run**: needs a display (no xdotool/Xvfb in the cloud container); with T034, T046, T056, T067 a follow-up
- [x] T069 Record quickstart §C (macOS and Windows): the P1 golden test passing on all three CI platforms (SC-003) and, where a machine is available, steps 4, 5 and 10; note in `specs/946-review-replies/quickstart.md` what was run where — recorded in quickstart §C: the golden test ran green on all three CI platforms; steps 4, 5, 10 on macOS/Windows hardware not run
- [x] T070 Check `docs/user-guide/reviewing-changes.md` and `docs/user-guide/agent-tools.md` against FR-028's list and the batch example against contracts/batch-format.md byte for byte; fix wording only — checked: the batch example in the guide equals the contract's byte for byte; FR-028's topics are all covered; no wording change needed
- [ ] T071 [US4] (BUG-001) Split `two_hundred_comments_locate_inside_the_budget` in `crates/micold-core/src/review/anchor.rs`: the debug test keeps the functional assertion (every quote is `Moved`); the unchanged 100 ms timing assertion moves to a release-only test (`#[cfg_attr(debug_assertions, ignore = "a release-build budget (BUG-001)")]`), and CI's "Test (type-ahead frame budget, release)" step runs it.

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (T001)** → **Foundational (T002–T009)** → every story.
- **US2 (T010–T018)** after Foundational.
- **US1 Part A (T019–T034)** after US2 (the agent reports by the batch's reference).
- **US1 Part B (T035–T046)** after Part A (card actions extend Part A's card states).
- **US3 (T047–T056)** after US1 Part B (rows show statuses, replies and the full-reply dialog).
- **US4 (T057–T067)** after Foundational for its core (T057, T058, T061, T062); its client tasks
  after US3, because T060/T064 switch the panel's row lines and jump (T052) from the stored range to
  `anchors`.
- **Polish (T068–T070)** after all stories.

### Within each phase

Test tasks first and failing; then the implementation that makes them pass; then glue, showcase,
docs, visual pass.

### Parallel opportunities

- Foundational: T002, T004, T006, T008 (different files).
- US2: T010, T011, T012, T013 together; T018 any time after T014.
- US1 Part A: T019, T020, T021, T022, T027, T028 together; T032, T033 after their implementation.
- US1 Part B: T035, T036, T037, T038 together.
- US3: T047, T048, T049 together.
- US4: T057, T058, T059, T060 together.

## Parallel Example: User Story 1, Part A

```text
T019 comment.rs report tests      T020 reply.rs tests
T021 mcp catalog/policy tests     T022 daemon review_reports.rs
T027 card geometry tests          T028 client features tests
```

## Implementation Strategy

1. **MVP (M1 + M2)**: M1 (Setup, Foundational, US2) sends the stable batch with references; M2 completes the P1 round trip (US1 reports shown on the cards).
2. **M2, M3**: US1: the agent's reports come back on the cards; marking by hand and full replies.
3. **M4**: US3: the review panel.
4. **M5**: US4: re-anchoring.
5. **Close**: Polish (quickstart §B/§C records, docs check).

Each milestone leaves `main` green and the app working: M1 changes only the batch and the store, and
the batch's fallback sentence covers the tool's absence until M2.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Stable review batch with comment references 🎯 MVP

- **Tasks**: T001–T018
- **Deliverable**: **Send review** types one message into the entry's session: the documented preamble and one `### Comment C-xxxxxxxx` block per draft, byte-identical to contracts/batch-format.md's layout; comments are stored in review file version 2 and 482's files still load.
- **Satisfies**: US2 acceptance scenarios 1–6; US1 scenario 8; FR-001, FR-002, FR-008, FR-009–FR-015, FR-019 (Clear sent), FR-026, FR-027 (batch, reference)
- **Verify**: `mise run test-core` (`review::prompt` P1–P9, `review::comment`, `review::store`, `protocol_roundtrip`); `scripts/gate.sh` (`review_send`, `features_changes`)
- **Depends on**: —
- **Tier**: full

### M2 — Agent reports shown on the comment cards

- **Tasks**: T019–T034
- **Deliverable**: a session calls `report_review_comment` with a comment's reference; the comment's card shows Applied or Answered and the reply in every window and after a restart, and refused reports say why.
- **Satisfies**: US1 acceptance scenarios 1–5, 7; FR-003, FR-004, FR-005, FR-007 (card), FR-008, FR-027 (report validation), FR-028 (agent tools page, statuses)
- **Verify**: `scripts/gate.sh` (`review_reports`, `mcp_tools_catalog`, `mcp_policy`, `features_changes`); quickstart §B steps 4–6, 10
- **Depends on**: M1
- **Tier**: full

### M3 — Mark by hand and read the full reply

- **Tasks**: T035–T046
- **Deliverable**: the user marks a Sent comment Applied, or Answered with a note, from its card, and opens any shortened reply in full.
- **Satisfies**: US1 acceptance scenario 6; FR-006; W14
- **Verify**: `scripts/gate.sh` (`review_edit` Mark tests, `features_changes`); quickstart §B step 7
- **Depends on**: M2
- **Tier**: full

### M4 — Review panel

- **Tasks**: T047–T056
- **Deliverable**: the Changes view shows a review panel listing every comment of the entry grouped by file with status, text and reply start and per-status counts; rows jump to the comment or say why not, edit and delete drafts, and the panel hosts Send review, Clear sent and Discard drafts.
- **Satisfies**: US3 acceptance scenarios 1–6; FR-007 (row), FR-016–FR-019; SC-006
- **Verify**: `scripts/gate.sh` (`features_changes` panel tests, `review_panel_row` geometry); quickstart §B steps 1–3, 9
- **Depends on**: M3
- **Tier**: full

### M5 — Comments follow their code

- **Tasks**: T057–T067
- **Deliverable**: after lines above a comment are added or removed, its card moves with its code and the next batch names the new lines; only gone or ambiguous code marks a Draft or Sent comment outdated.
- **Satisfies**: US4 acceptance scenarios 1–8; FR-020–FR-025, FR-027 (re-anchoring); SC-004; W15, W16
- **Verify**: `mise run test-core` (`review::anchor`, `review::comment` reanchor); `scripts/gate.sh` (`review_edit` Reanchor, `review_send` W16, `features_changes`); quickstart §B step 8
- **Depends on**: M4
- **Tier**: full

**Bugfix**: 2026-10-18 — BUG-001 Updated from bugfix patch
