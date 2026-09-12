---

description: "Task list for feature 029 — Refresh the Worktree List on Demand"
---

# Tasks: Refresh the Worktree List on Demand

**Input**: Design documents from `specs/029-refresh-worktrees-list/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/worktree-refresh.md](./contracts/worktree-refresh.md),
[quickstart.md](./quickstart.md)

**Tests**: Mandatory (Constitution Principle I). Every story writes its failing tests first. The
GUI-glue exception covers only `src/main.rs`, `src/ui/`, `src/showcase/` — note that
`src/shell/daemon_sync.rs` is **not** covered by it, which is why the single-flight guard and the
timeout both carry tests (research R7).

**Documentation**: Mandatory (Principle VII). US1 and US2 each ship their user-guide change.

**Cross-platform**: Nothing here is platform-specific — no new path handling, no new process
spawn. The final phase confirms it rather than assuming it (Principle VI).

**Organization**: By user story. US1 is deliverable alone; US2 adds the in-flight affordance on
top of it; US3 is largely proof that the reuse in US1 really did inherit the existing
reconciliation.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 / US2 / US3
- Paths are repository-relative; `crates/` is the workspace root

---

## Phase 1: Setup

**Purpose**: know the tree is green before touching it, and have the manual fixture ready.

- [X] T001 Establish the green baseline from the repository root: `cargo fmt --all -- --check`, then `mise run test`, then `cargo clippy --workspace --all-targets`. A pre-existing failure must be identified now, not mistaken for this feature's later (quickstart §A.1).
- [X] T002 [P] Create the manual-pass fixture per quickstart §B.0: `git init -b main /tmp/wt-029` with one empty commit. Nothing else in this feature needs it, so it can be built at any point before Phase 6.
- [X] T003 [P] Confirm `crates/micold-core/src/protocol/version.rs` still reads `PROTOCOL_VERSION = 9` and that no other change on this branch has moved `SCHEMA_HASH`. The one-bump-per-feature rule is only safe if this feature is the only wire change in flight (contracts §6).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the wire and the icon. Both are prerequisites of every story and neither is
user-visible on its own.

**⚠️ CRITICAL**: no user story work begins until T012 passes.

### The wire (contracts/worktree-refresh.md)

- [X] T004 [P] Add a serialize/deserialize round-trip case for `ClientMsg::WorktreeRefresh { req, project }` to `crates/micold-core/tests/protocol_roundtrip.rs`. It MUST fail to compile — the variant does not exist yet.
- [X] T005 [P] Create `crates/micold-daemon/tests/worktree_refresh.rs`: with a project open, create a worktree behind the daemon's back, send `WorktreeRefresh`, and assert the daemon emits `DaemonMsg::CatalogChanged` carrying the new worktree **and then** `OperationOk { result: OperationResult::Ack }` — the ordering assertion is the point (contracts §4), and a test that only checks the set of frames would pass on the wrong order. MUST fail.
- [X] T006 Add `ClientMsg::WorktreeRefresh { req: u64, project: PathBuf }` to `crates/micold-core/src/protocol/messages.rs` beside the other worktree RPCs, **and** bump `PROTOCOL_VERSION` 9 → 10 with its history line in `crates/micold-core/src/protocol/version.rs` — in one edit. A second bump later in this feature fails `crates/micold-core/tests/schema_hash.rs` (contracts §6).
- [X] T007 Add the `ClientMsg::WorktreeRefresh` arm to `route()` in `crates/micold-daemon/src/server.rs`: call the existing `refresh_worktrees_and_broadcast` (`server.rs:1489`), then `send_ack` — in that order, and add nothing else. Reuse verbatim; a second discovery path is what FR-004 exists to prevent (contracts §2).
- [X] T008 Verify the wire: `mise run test-core` and `cargo test -p micold-daemon --test worktree_refresh`. T004 and T005 must now pass and `schema_hash.rs` must still pass — a red `schema_hash` here means the bump and the variant did not land in one edit.

### The icon (research R8)

- [X] T009 [P] Add `Icon::Refresh => '\u{e5d5}'` to the `expected()` match in `crates/micold-client/tests/icons.rs`. The match is exhaustive, so this pins the codepoint and fails to compile until T011 lands.
- [X] T010 [P] Add the `Refresh` / `refresh` / `E5D5` row to the icon table in `assets/fonts/PROVENANCE.md`, keeping the existing column order.
- [X] T011 Add `Refresh` to the `Icon` enum, to `Icon::ALL`, and to `glyph()` in `crates/micold-client/src/icons.rs` — all three in one edit, since `ALL` and `glyph()` are what the two gates read.
- [X] T012 Verify the icon: `cargo test -p micold-client --test icons --test icons_font`. `icons_font` parses the shipped `.ttf`, so a green run is direct proof U+E5D5 is not tofu.

**Checkpoint**: the daemon can serve a refresh nothing asks for, and the icon exists unused.

---

## Phase 3: User Story 1 - See a worktree that appeared outside the app (Priority: P1) 🎯 MVP

**Goal**: pressing a control in the sidebar header makes an externally created worktree appear,
without leaving the project.

**Independent Test**: quickstart §B.1 — open `/tmp/wt-029`, `git worktree add` from a terminal,
confirm the sidebar does **not** show it, press refresh, confirm it appears.

**Note on scope**: this story deliberately ships *without* the in-flight state. The control is
actionable whenever a project is open; US2 adds `refreshing` on top. That keeps US1 genuinely
deliverable rather than nominally so, and the spec says as much ("Without it US1 still works").

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these first and watch them fail.

- [X] T013 [P] [US1] Test `State::can_refresh_worktrees()` in `crates/micold-client/tests/app_state.rs`: true with an active project, false with none (data-model §3 cases P1, P3).
- [X] T014 [P] [US1] Test `Msg::RefreshRequested` in `crates/micold-client/tests/features_worktree.rs`: it produces the outcome that asks the shell to send a refresh for the active project, and leaves the worktree listing byte-identical (data-model rule T6).
- [X] T015 [P] [US1] Extend the inline `mod tests` in `crates/micold-client/src/shell/daemon_sync.rs`: `PendingOp::WorktreeRefresh.describe()` reads `"refresh the worktree list"`; a matching `OperationOk` clears the pending op; a send while disconnected raises the existing "Not connected to the session service" notice rather than a new one.

### Implementation for User Story 1

- [X] T016 [US1] Add `Msg::RefreshRequested` and its reducer arm to `crates/micold-client/src/features/worktree.rs`, emitting the cross-feature `features::Outcome` the shell acts on (feature 028's contract: the feature module never talks to the wire itself).
- [X] T017 [US1] Add `pub fn can_refresh_worktrees(&self) -> bool` to `crates/micold-client/src/app.rs`, returning `self.workspace.active.is_some()`. Connectedness is deliberately excluded — `send_op` already reports it (data-model §3).
- [X] T018 [US1] Add `PendingOp::WorktreeRefresh` (with its `describe()` arm), the send function, and the `OperationOk` / `OperationError` / disconnect-drain arms to `crates/micold-client/src/shell/daemon_sync.rs`, following `on_worktree_include_requested` (`:1376`) as the pattern. The error arm MUST NOT call `set_worktrees` (FR-008).
- [X] T019 [US1] Add the one routing arm for `Message::Worktree(WorktreeMsg::RefreshRequested)` to `crates/micold-client/src/main.rs` (glue).
- [X] T020 [US1] Add the control to the header row in `crates/micold-client/src/ui/sidebar.rs`, between the `Length::Fill` title and `add_worktree`: `IconButton::new(Icon::Refresh, r).compact().tint(r.on_surface_variant)` wrapped in a `Tooltip` reading "Refresh the worktree list", with `on_press` attached **only** when `state.can_refresh_worktrees()` — so an unavailable control cannot emit a message at all (research R7, FR-005).
- [X] T021 [US1] **Immediately** run the width gates: `cargo test -p micold-client --test layout_text_overflow --test gates`. If "Worktrees" is pushed past the sidebar's clip at the 180 px minimum, apply research R10 fallback 1 — ellipsize the title with `ui/material/ellipsized.rs` — **now**, before T022, because everything downstream would have to be re-recorded.
- [X] T022 [US1] Regenerate the layout fixture deliberately: `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`, then read the diff to `crates/micold-client/tests/fixtures/layout_snapshot.txt` and confirm the **only** change is the header gaining one control (FR-011). An unexplained second change is a bug, not noise.
- [X] T023 [US1] Document the control in `docs/user-guide/worktrees-and-sessions.md` — what it does, when you need it (a worktree created outside the app), and that switching projects is no longer the workaround — and add the `Refresh` row to `docs/user-guide/icons.md` (Principle VII).

**Checkpoint**: US1 is independently demonstrable via quickstart §B.1 and §B.3.

---

## Phase 4: User Story 2 - Know that the refresh happened (Priority: P2)

**Goal**: the control shows that the request was taken, that it is running, and that it finished —
including when nothing changed, and including when no answer ever comes.

**Independent Test**: quickstart §B.4 and §B.5 — press refresh on an unchanged project and see
in-progress then completion; stop the daemon and see the failure notice with the list intact.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T024 [P] [US2] Test the transition table in `crates/micold-client/tests/features_worktree.rs`: `RefreshRequested` sets `refreshing`; `RefreshFinished` clears it; a second `RefreshRequested` while refreshing is **dropped** (rule T2); none of the three touches the listing (rule T6).
- [X] T025 [P] [US2] Extend `crates/micold-client/tests/app_state.rs`: `can_refresh_worktrees()` is false while `refreshing` (data-model case P2).
- [X] T026 [P] [US2] Test the bounded wait in the inline `mod tests` of `crates/micold-client/src/shell/daemon_sync.rs`: with no reply at all the control returns to idle and a notice is raised, and a reply arriving **after** the timeout is discarded without an error (rule T4). This is the codebase's first wire timeout — the test must assert the timeout path itself, not just the happy path (research R5).

### Implementation for User Story 2

- [X] T027 [US2] Add `refreshing: bool` to `State` in `crates/micold-client/src/features/worktree.rs`, plus `Msg::RefreshFinished` and `Msg::RefreshTimedOut(u64)` and their reducer arms, implementing rules T1–T6 (data-model §2). A `bool`, not an `Option<u64>` — the rationale is recorded there.
- [X] T028 [US2] Extend `can_refresh_worktrees()` in `crates/micold-client/src/app.rs` with `&& !self.worktree.refreshing`.
- [X] T029 [US2] Add the 30 s timeout to `crates/micold-client/src/shell/daemon_sync.rs`, firing `RefreshTimedOut(req)` for the outstanding correlation id, and record in a comment beside it that this is the first timeout on this protocol and that a second one should become a shared helper rather than a copy (research R5).
- [X] T030 [US2] Raise the completion notice via the existing `notify_info` on the `OperationOk` arm, and the failure notice on `OperationError` — both through existing surfaces only; this feature defines no new notification type (spec Assumptions, research R6).
- [X] T031 [US2] Make the control show the busy state in `crates/micold-client/src/ui/sidebar.rs`: while `refreshing`, omit `on_press` and change the tooltip to "Refreshing worktrees…". No spinner — the reason a new indeterminate component was rejected is recorded in research R6.
- [X] T032 [US2] Register the busy header as a covered layout state in `crates/micold-client/tests/support/covered_states.rs` (the single registration site, feature 019) and regenerate the fixture if it moves. If the busy state is layout-identical to idle, say so in a comment there instead of adding a redundant entry.
- [X] T033 [US2] Extend `docs/user-guide/worktrees-and-sessions.md` with what the user sees while a refresh runs, what happens if it fails (the list stays), and the fact that a refresh that finds nothing still confirms it ran.

**Checkpoint**: US1 and US2 both work; the control is honest about its own state.

---

## Phase 5: User Story 3 - Refresh without losing my place (Priority: P3)

**Goal**: prove that a refreshed listing is reconciled exactly like any other, so nothing about the
user's arrangement moves.

**Independent Test**: quickstart §B.2 — expand, filter, scroll, select, start a session, press
refresh on an unchanged project, and confirm nothing moved; then compare against an
app-initiated worktree creation and confirm the two are indistinguishable.

**Note**: this story adds no production code by design. FR-004 is satisfied *structurally* — there
is one reconciliation path, not two kept in agreement (contracts §2) — so the story's work is the
evidence that the structure holds, and a guard against a future second path.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T034 [P] [US3] Test in `crates/micold-client/tests/sidebar_state.rs` that applying an identical listing leaves expansion, filters, scroll position and selection untouched, and that a listing missing a previously expanded+selected worktree is reconciled by the existing path — asserting the *shared* behaviour, not a refresh-specific copy of it (FR-004, FR-009).
- [X] T035 [P] [US3] Test in `crates/micold-client/tests/features_worktree.rs` that `RefreshRequested`, `RefreshFinished` and `RefreshTimedOut` each leave the listing untouched — the listing arrives only via `CatalogChanged` (rule T6).

### Implementation for User Story 3

- [X] T036 [US3] Create `crates/micold-client/tests/refresh_is_only_on_demand.rs`, a source scan asserting the refresh RPC has exactly one client-side call site — the sidebar header control — so no automatic, periodic or filesystem-triggered refresh can be added without the guard failing (FR-012). `crates/micold-core/tests/documentation_is_not_read.rs` is the precedent for the scan style, including its rule that a stale allowlist entry fails too.
- [X] T037 [US3] Add the sentence to `docs/user-guide/worktrees-and-sessions.md` that the list is never re-read on its own — on demand means on demand — so a user who expects live updates learns the truth from the docs rather than from a support thread.

**Checkpoint**: all three stories independently demonstrable.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T038 Run the full gate in CI's order from the repository root: `cargo fmt --all -- --check`, `mise run test`, `cargo clippy --workspace --all-targets`. `fmt` first — it gates every other CI job.
- [X] T039 Walk quickstart §A.2's obligation table row by row and confirm each named gate really does contain new coverage: revert this feature's change to the file beside a row and the row's test must fail. A row whose test passes without the change is a row that proves nothing.
- [X] T040 Confirm Principle VI by inspection and by CI: no new path handling, no new process spawn, no platform-conditional code was added. Cross-platform parity here is a property to check, not a claim to make.
- [X] T041 Run quickstart Part B §B.1–§B.7 by eye, via the repo's `visual-pass` skill (take a private display and pin directory — another agent's window can otherwise land in the run).
- [X] T042 Answer research R6's open question in the visual pass: does an inert greyed button read as *busy* or as *broken*? If it reads as broken, file it as a finding against the icon-button component rather than fixing it inside this feature — the fix would be a new shared affordance, which is a feature of its own.
- [X] T043 Re-read [plan.md](./plan.md)'s Constitution Check against what was actually built and record any deviation in Complexity Tracking. It is currently empty; if it is still empty at the end, that is a result worth stating rather than an omission.

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: T004–T008 (wire) and T009–T012 (icon) are independent of each other and can run in parallel; both block every story.
- **US1 (Phase 3)**: needs Phase 2 complete. Delivers the MVP.
- **US2 (Phase 4)**: needs US1 — it modifies the predicate, the control and the reply arms US1 created.
- **US3 (Phase 5)**: needs US1; independent of US2 (its assertions are about the listing, which neither story's messages touch).
- **Polish (Phase 6)**: needs every story that is being shipped.

### Within a story

Tests before implementation, always (Principle I). Then: feature state → shell → glue → gates →
docs. T021 is the one ordering that is not a convention but a hard requirement — the width gates run
before the snapshot is regenerated, or the regeneration bakes in a layout that then has to be
redone.

### Parallel opportunities

- T002, T003 together.
- The whole wire track (T004–T008) alongside the whole icon track (T009–T012).
- Within each story, every `[P]` test task at once — they are separate files.
- US3's tests (T034, T035) can be written during US2, since they assert behaviour US1 already
  established.

**Caution**: `mise run` builds share one `target-shared/` directory with an exclusive lock, so
"parallel" here means parallel authoring, not parallel `cargo` invocations — a second build waits
(see CLAUDE.md), and that is intended.

---

## Parallel Example: Foundational Phase

```bash
# Two independent tracks, one blocking checkpoint each:
Track A (wire):  T004 → T005 → T006 → T007 → T008
Track B (icon):  T009 ∥ T010 → T011 → T012
```

---

## Implementation Strategy

### MVP first (US1 only)

1. Phase 1 → Phase 2 → Phase 3.
2. **Stop and validate**: quickstart §B.1 and §B.3.
3. At this point the feature's whole reason to exist is delivered — an externally created worktree
   is reachable in one action.

### Incremental delivery

1. Foundation → US1 → demo (MVP).
2. US2 → the control stops being ambiguous when nothing changed.
3. US3 → the guarantee that it never costs the user their place.

### The one thing to watch

Research R10, taken early by T021: the sidebar header already had no spare width before this
feature, and a third control is exactly the kind of change that fails for a reason unrelated to its
own logic. If T021 goes red, the fallback order is fixed in advance — ellipsize the title, then a
recorded FR-045-style deviation, and **not** moving the control somewhere else.

---

## Notes

- `[P]` = different files, no dependency on an incomplete task.
- Commit after each task or logical group; the wire edit (T006) is deliberately one commit's worth
  of change in one file pair.
- Verify every test fails before implementing it. A test written after the code it covers proves
  the code compiles, not that it works.
