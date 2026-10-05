---

description: "Task list for #582: attach a provider's existing worktrees and sessions"
---

# Tasks: Attach a Provider's Existing Worktrees and Sessions

**Input**: Design documents from `/specs/582-attach-provider-worktrees-sessions/`

**Prerequisites**: plan.md, spec.md, research.md (R1-R10), data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY and first (Constitution I). Within each story, the test tasks come before the
implementation tasks they cover, and each test must be seen failing for the right reason first.

**Documentation**: each user-facing story carries its user-guide task (Constitution VII).

**Cross-platform**: no OS branch in core; paths are `PathBuf` (Constitution VI).

**Organization**: grouped by user story. Paths: `crates/micold-core/src`, `crates/micold-daemon/src`,
`crates/micold-client/src`, tests beside each crate in `tests/`.

## Phase 1: Setup

- [X] T001 Create `crates/micold-core/src/attach.rs` and export it from `crates/micold-core/src/lib.rs`, holding only the data-model.md types: `AttachableWorktree`, `Availability` (`Attachable`, `Unavailable(Missing | Invalid)`), `ResumableSession`, `AttachTarget` (`Default | Worktree { dir_name }`), `ResumableStatus` (`Resumable | NeedsWorktreeAttach | Unresumable(UnresumableReason: WorktreeMissing | WorktreeInvalid | NoLocation)`), `DiscoveryReport { worktrees, sessions, notes }`, `DiscoveryNote`, `SkipReason` (`StoreMissing | StoreUnreadable | EntryCorrupt | SandboxStoreNotReadable`), `AttachItem` (`Worktree { dir_name } | Session { id }`), `AttachOutcome` (`Attached | AlreadyAttached | Refused(reason)`, reason includes `NotAWorktreeOfProject`); all `serde` Serialize/Deserialize.
- [X] T002 [P] Add a shared test fixture `crates/micold-core/tests/support/attach_fixture.rs`: builds a scratch git repo with N worktrees under `.claude/worktrees/`, a missing (prunable) one, and a fake provider store (`~/.claude/projects/<encoded cwd>/<uuid>.jsonl`) seeded for this repo and for a sibling repo (`/a/proj` vs `/a/proj-x`).

## Phase 2: Foundational (blocks all stories)

- [X] T003 Write failing round-trip tests for `ClientMsg::{AttachDiscover { req, project }, AttachApply { req, project, targets: Vec<AttachItem> }}` and `DaemonMsg::AttachReport { req, report }` in `crates/micold-core/tests/protocol_roundtrip.rs`.
- [X] T004 Add those three messages in `crates/micold-core/src/protocol/messages.rs` and update the pinned hash in `crates/micold-core/tests/schema_hash.rs` (R6, contracts/attach-protocol.md).
- [X] T005 Write failing tests in `crates/micold-daemon/tests/attach_apply.rs` for `Catalog::attach_worktrees`: a batch writes one provenance record per `dir_name` with one persist; a recorded `dir_name` returns `AlreadyAttached`; a target absent from the live worktree cache returns `Refused(NotAWorktreeOfProject)`; two concurrent attaches of one worktree leave exactly one record (FR-004, FR-016).
- [X] T006 Implement `Catalog::attach_worktrees` over `claim_worktree` in `crates/micold-daemon/src/catalog.rs`: validate each target against the worktree cache, one persist per batch, no git and no filesystem write (FR-003).

## Phase 3: User Story 1 - Attach a provider worktree from the app (P1) 🎯 MVP

**Goal**: list attachable provider worktrees and attach them from the app.

**Independent Test**: `mise run test-core` plus `crates/micold-daemon/tests/attach_apply.rs` and
`crates/micold-client/tests/attach_dialog.rs`; quickstart B2-B3 for worktrees.

- [X] T007 [P] [US1] Write failing tests in `crates/micold-core/tests/attach_discovery.rs` for `attachable_worktrees`: lists agent-owned unrecorded worktrees (FR-001, scenario 1); a recorded worktree is not listed (FR-004); a prunable or missing one is `Unavailable(Missing)`; only worktrees directly under `.claude/worktrees/` qualify; an unattached provider worktree stays hidden by `classify_owner` (FR-013).
- [X] T008 [US1] Write failing daemon tests in `crates/micold-daemon/tests/attach_apply.rs` for `AttachDiscover` and `AttachApply` with worktree items: the attached worktree survives a daemon restart and shows with "Show agent worktrees" off (FR-002); branch, files and uncommitted changes unchanged, checked with `git status` and file hashes (FR-003, SC-003); attaching again returns `AlreadyAttached` (FR-004); 16 worktrees attach in one action in under 10 s with a generous CI bound (SC-001).
- [X] T009 [P] [US1] Write failing reducer tests in `crates/micold-client/tests/attach_dialog.rs`: checkbox selection, "Attach all", "Attach selected", and the "already attached" message (FR-004, scenario 3).
- [X] T010 [US1] Implement `attachable_worktrees` in `crates/micold-core/src/attach.rs` from `reconcile()` output filtered to `classify_owner == Agent`, with `session_count` (catalog sessions at that worktree) and `provider` when known.
- [X] T011 [US1] Implement `attach_discover` and `attach_apply` (worktree items; blocking work off the lock via `spawn_blocking`, one persist, one `CatalogChanged` broadcast) in `crates/micold-daemon/src/state.rs`, and the two message arms in `crates/micold-daemon/src/server.rs`.
- [X] T012 [US1] Create the render-free reducer `crates/micold-client/src/features/attach.rs` (dialog state, selection, outcome messages) and register it in `crates/micold-client/src/features/mod.rs`.
- [X] T013 [US1] Add `PendingOp::Attach` request/response plumbing in `crates/micold-client/src/shell/daemon_sync.rs`.
- [X] T014 [US1] Add the sidebar "Attach existing…" button and the modal list dialog (checkbox per row, "Attach selected", "Attach all") in `crates/micold-client/src/ui/sidebar.rs` and new `crates/micold-client/src/ui/attach_dialog.rs`, built from the existing overlay dialog and shared button/row primitives with the builder API (R9, Constitution VIII).
- [X] T015 [US1] Update `docs/user-guide/worktrees-and-sessions.md`: how to attach provider worktrees from the app and what attaching does not touch.
- [ ] T016 [US1] Run the `visual-pass` skill for quickstart B2 and B3 (worktrees only), light and dark theme, and save the evidence in `specs/582-attach-provider-worktrees-sessions/visual/`.

## Phase 4: User Story 2 - Discover and resume provider sessions (P1)

**Goal**: list resumable provider sessions of the project and resume one in its worktree.

**Independent Test**: `crates/micold-core/tests/attach_discovery.rs`,
`crates/micold-daemon/tests/attach_apply.rs`; quickstart B1-B3 for sessions.

- [X] T017 [US2] Write failing tests in `crates/micold-core/tests/attach_discovery.rs` for `discover_resumable` over the T002 fixture: 5 of this project listed, the sibling project `/a/proj-x` never (FR-006, FR-007, SC-002); a catalog session is subtracted (scenario 3); a corrupt entry is skipped and noted while others list (FR-009, SC-005); a missing store gives no sessions and a `StoreMissing` note (scenario 4, FR-014); a deleted worktree's session is `Unresumable` and never mapped elsewhere; a root session is `AttachTarget::Default`; a store of thousands lists newest first within a page bound (R4: one directory listing per location, no per-conversation read), and a Windows-style path with a different case maps to the same project (macOS/Windows edge case); the provider store tree hash is unchanged after the pass (FR-010).
- [X] T018 [P] [US2] Write failing tests for `AiCliProvider::store_dirs` in `crates/micold-core/tests/provider_store_dirs.rs`: Claude returns the encoded root, git-listed worktrees and directories under the encoded `<root>/.claude/worktrees/`, accepting an unknown directory only when its first transcript line's `cwd` is under this project's `.claude/worktrees/` (R3); Copilot and Pi return known locations only; add Copilot (`sidebar-sessions-state/<sha256(cwd)>.json`) and Pi (`sessions/--<encoded cwd>--/`) fixture-store tests in the same `provider_store_dirs.rs` that list this project's sessions at known locations and none for a provider with no readable store (FR-006, FR-014).
- [x] T019 [US2] Write failing daemon tests in `crates/micold-daemon/tests/attach_apply.rs`: a `Session` item adds an idle catalog entry and never starts the provider (FR-008); a session whose worktree is attachable resumes by attaching the worktree first, then `SessionStart`; a second start while `Starting`, `Running` or `Restarting` is refused with text naming "already running" (FR-016); a sandboxed project reports `SandboxStoreNotReadable`; the resume launch for each provider carries the right argument and session id (`--resume <id>`, `--resume=<id>`, `--session-id <id>`, FR-008).
- [x] T020 [US2] Write failing reducer tests in `crates/micold-client/tests/attach_dialog.rs` for session rows (status, reason text, resume action) and that discovery notes appear as a footer in the dialog so the user can see why none were found (Story 2 scenario 4, FR-009).
- [X] T021 [US2] Add `store_dirs(config_dir, root) -> Vec<StoreDir>` to `AiCliProvider` in `crates/micold-core/src/provider.rs`, default returning known locations only, overridden by `ClaudeProvider`.
- [X] T022 [US2] Implement `discover_resumable` in `crates/micold-core/src/attach.rs`: one directory listing per location, known catalog ids subtracted before any `is_archived` stat, mtime-descending order, title and label read for the returned page only, only read methods called (R3, R4).
- [x] T023 [US2] Extend `attach_discover` and `attach_apply` in `crates/micold-daemon/src/state.rs` for sessions: adopt each as an idle `Session::restored`, attach an unattached worktree first, report `SandboxStoreNotReadable`; make the resume refusal text name "already running" in the existing start guard (R7).
- [x] T024 [US2] Show session rows, notes and the resume action in `crates/micold-client/src/features/attach.rs` and `crates/micold-client/src/ui/attach_dialog.rs`.
- [x] T025 [US2] Update `docs/user-guide/worktrees-and-sessions.md`: resumable sessions, why one may be unresumable, and that a session already running outside Micold is not detected (R7).
- [ ] T026 [US2] Run the `visual-pass` skill for quickstart B2 and B3 with sessions and a note for an unreadable store; save evidence in `specs/582-attach-provider-worktrees-sessions/visual/`.

## Phase 5: User Story 3 - Attach through the MCP tools (P2)

**Goal**: `attach_worktree` and `list_resumable_sessions` exist as MCP tools.

**Independent Test**: `crates/micold-daemon/tests/attach_mcp.rs`.

- [ ] T027 [US3] Write failing tests in `crates/micold-core/tests/mcp_tools_catalog.rs` and `crates/micold-core/tests/mcp_policy.rs`: both tools are in the catalog with the schemas of contracts/; `attach_worktree` is mutating, audited and not destructive; a Default caller is refused with the Principle III text while `list_resumable_sessions` is allowed for every caller (FR-011, FR-015).
- [ ] T028 [P] [US3] Write failing tests in `crates/micold-daemon/tests/attach_mcp.rs`: `attach_worktree` by `dir_name`, by absolute path and by branch attaches and `list_worktrees` then returns `hidden: false` (scenario 1, SC-004); a path outside the project or another project's worktree returns `not_found` and changes nothing (scenario 2); an ambiguous branch, `default`, or an unavailable worktree returns `invalid_input`; a repeat returns `already_attached` (scenario 3); a Default caller returns `refused_by_policy` (scenario 4); `list_resumable_sessions` pages with `limit` (1..=200, default 50) and `offset` and lists no catalog session.
- [ ] T029 [US3] Add `Operation::{AttachWorktree, ListResumableSessions}`, their catalog entries and `is_mutating_tool` handling in `crates/micold-core/src/mcp/tools.rs`, and the Default-caller refusal in `crates/micold-core/src/mcp/policy.rs` (same arm as rename and delete).
- [ ] T030 [US3] Implement the `attach_worktree` and `list_resumable_sessions` handlers in `crates/micold-daemon/src/mcp/tools.rs`, with resolution order `dir_name`, canonical path, branch, per contracts/attach-worktree-tool.md and contracts/list-resumable-sessions-tool.md.
- [ ] T031 [US3] Update `docs/user-guide/agent-tools.md` with both tools, their errors and that a Default session cannot attach.

## Phase 6: User Story 4 - Start-up offer (P3)

**Goal**: a project with no provenance records is offered attaching everything found in one action.

**Independent Test**: `crates/micold-client/tests/attach_offer.rs`; quickstart B1.

- [ ] T032 [US4] Write failing reducer tests in `crates/micold-client/tests/attach_offer.rs`: `offer_visible = no_records && !report.is_empty() && !dismissed`, where `no_records` ignores catalog sessions adopted by feature 026; a project with provenance records never gets the offer and nothing is attached without a user action (FR-012, scenarios 1-2); dismissal is per project and per run, in memory, and the "Attach existing…" dialog still lists everything (R5).
- [ ] T033 [US4] Implement `OfferState` and the offer rules in `crates/micold-client/src/features/attach.rs`, requesting `AttachDiscover` at project open from `crates/micold-client/src/shell/daemon_sync.rs`.
- [ ] T034 [US4] Add the offer banner with "Attach all" and a dismiss button in `crates/micold-client/src/ui/sidebar.rs`: first `grep` `crates/micold-client/src/ui/material/` for a notice or snackbar with action buttons and note the finding in the commit; reuse it if found, else add a `Banner` primitive to the shared library under `crates/micold-client/src/ui/material/` and consume it (R9, Constitution VIII).
- [ ] T035 [US4] Update `docs/user-guide/worktrees-and-sessions.md` with the start-up offer and how to dismiss it.
- [ ] T036 [US4] Run the `visual-pass` skill for quickstart B1 and B4 (banner, light and dark); save evidence in `specs/582-attach-provider-worktrees-sessions/visual/`.

## Phase 7: Polish

- [ ] T037 Run quickstart Part A and Part B end to end and record the result in `specs/582-attach-provider-worktrees-sessions/quickstart.md`.
- [ ] T038 Check that the user guide pages match the shipped behaviour of all four stories and that no `docs/` page still says attaching or the offer is missing.

## Dependencies & Execution Order

- Phase 1, then Phase 2 (blocks everything); then US1, US2, US3, US4 in priority order.
- US2 builds on US1's dialog, `attach_discover` and `attach_apply`. US3 needs T010 and T022 (the core passes) and T006. US4 needs the dialog from US1 and the sessions from US2.
- Within a story: tests, then core, then daemon, then client, then docs.
- Parallel: T002 beside T001; T007, T008, T009 are in different files; T017 and T018; T027 and T028.

## Implementation Strategy

MVP is M1 (Setup, Foundational, US1): attach provider worktrees from the app. Each later story is one
milestone on top, ending with the polish pass.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Attach provider worktrees from the app 🎯 MVP

- **Tasks**: T001–T016
- **Deliverable**: the sidebar's "Attach existing…" dialog lists a project's unattached provider worktrees and attaching one shows it with "Show agent worktrees" off, its branch and files untouched.
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001, FR-002, FR-003, FR-004, FR-013, FR-016 (worktrees); SC-001, SC-003
- **Verify**: `mise run test-core`; `cargo test -p micold-daemon --test attach_apply`; `cargo test -p micold-client --test attach_dialog`; quickstart B2–B3 (worktrees)
- **Depends on**: —
- **Tier**: full

### M2 — Discover and resume provider sessions

- **Tasks**: T017–T026
- **Deliverable**: the same dialog lists the project's resumable provider sessions (never another project's), flags unresumable ones with the reason, and resuming one attaches its worktree first and continues the conversation.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-006, FR-007, FR-008, FR-009, FR-010, FR-014, FR-016; SC-002, SC-005
- **Verify**: `mise run test-core`; `cargo test -p micold-daemon --test attach_apply`; quickstart B2–B3
- **Depends on**: M1
- **Tier**: full

### M3 — Attach through the MCP tools

- **Tasks**: T027–T031
- **Deliverable**: an agent calls `attach_worktree` to re-attach a provider worktree in one call and `list_resumable_sessions` to find sessions; a Default session is refused.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-005, FR-011, FR-015; SC-004
- **Verify**: `cargo test -p micold-daemon --test attach_mcp`; `cargo test -p micold-core --test mcp_policy`
- **Depends on**: M2
- **Tier**: full

### M4 — Start-up offer

- **Tasks**: T032–T036
- **Deliverable**: opening a project with no provenance records shows a banner offering to attach every found worktree and session in one action, dismissible per run.
- **Satisfies**: US4 acceptance scenarios 1–2; FR-012
- **Verify**: `cargo test -p micold-client --test attach_offer`; quickstart B1 and B4
- **Depends on**: M2
- **Tier**: full

### M5 — Polish

- **Tasks**: T037–T038
- **Deliverable**: quickstart Parts A and B pass and the user guide matches the shipped behaviour.
- **Satisfies**: SC-001–SC-005 (confirmation); Constitution VII
- **Verify**: quickstart Part A and Part B
- **Depends on**: M1, M2, M3, M4
- **Tier**: docs
