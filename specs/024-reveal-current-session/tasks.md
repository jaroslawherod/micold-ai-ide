# Tasks: Reveal the current session in the sidebar

**Input**: Design documents from `/specs/024-reveal-current-session/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Bugfix**: 2026-08-19 — [BUG-001](./bugs/BUG-001.md) Updated from bugfix patch: T021 reopened,
Phase 8 added (T061–T065).

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), test tasks are MANDATORY and
are written to fail first. The three things no test in this repository can see — a single frame, a
perceptual weight, a row's position against a real viewport — are quickstart §B's job, not an excuse
to skip §A.

**Documentation**: Per Principle VII, each user-facing story carries its own user-guide task in the
same change. Both target sections of `docs/user-guide/worktrees-and-sessions.md` already exist:
`## Starting, switching, and closing sessions` (:313) and `### Filtering worktrees by tag` (:47).

**Cross-platform**: Per Principle VI, nothing here branches on platform — the work is arithmetic,
a predicate, and iced operations. CI covers Linux, macOS and Windows.

**Build commands**: `mise run test` (whole workspace, matches CI), `mise run test-core` (render-free
core, faster while iterating). Never raw `cargo` — see CLAUDE.md.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)

---

## Phase 1: Setup

**Purpose**: A known-good starting point and the scaffolding the story tests share.

- [X] T001 Confirm a green baseline with `mise run test` before editing anything under `crates/`, so a later red test is this feature's and not inherited
- [X] T002 [P] Extend the shared scaffolding in `crates/micold-client/tests/support/mod.rs` with (a) a project holding sessions in both a worktree and the Default location, and (b) an N-worktree project for SC-003's 30-location sizing case — both render-free, no filesystem access, matching the module's existing `FakeScanner` style

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The derived open-state model. Every story reads it, so nothing else can start until it
is in place.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T003 Add the three view-state fields to `State` in `crates/micold-client/src/app.rs` — `reveal_suppressed_for: Option<SessionId>`, `sidebar_viewport_height: f32`, `pending_reveal_scroll: Option<f32>` — each with a doc comment naming its invariant from [data-model.md](./data-model.md) I1–I5, and none of them persisted
- [X] T004 Write failing tests for the effective-open predicate in `crates/micold-client/tests/features_sidebar.rs`: forced by the current session, suppressed by a user collapse, unaffected by a replaced worktree list, and reduced to `user_open` when the session record or its location is gone (contract §1.1–§1.4)
- [X] T005 Implement `location_of_current_session()` and `effective_open()` in `crates/micold-client/src/features/sidebar.rs` per contract §1.1 — pure, render-free, no caching of the result
- [X] T006 Replace the direct `expanded` / `default_expanded` reads in `worktree_tree`, `filtered_worktree_tree` and `sidebar_entries` with `effective_open` in `crates/micold-client/src/features/sidebar.rs`, so §1.3's "evaluated on every view" is structural rather than a convention
- [X] T007 Write the failing test for the collapse path in `crates/micold-client/tests/app_state.rs` **before** T008: collapsing a forced-open row sets `reveal_suppressed_for = active_session`, and from then on the location reads closed while that session stays current (contract §2.1, FR-005). Red must be observed here — `app.rs` is a render-free reducer with decision logic of its own, so Principle I's GUI-wiring exception does **not** reach it
- [X] T008 Re-express `Message::WorktreeExpansionToggled` (`crates/micold-client/src/app.rs:909`) and `Message::DefaultExpansionToggled` (`:914`) to toggle against `effective_open` rather than against `expanded` alone — collapsing a forced-open row must set `reveal_suppressed_for` (contract §2.1), not merely remove a key that was never there
- [X] T009 Confirm `crates/micold-client/tests/features_are_render_free.rs` still passes with the new predicate, and that nothing added in T005–T008 pulled a rendering type into `features/`

**Checkpoint**: Open-ness is derived and a user's collapse is honoured. Rows do not yet open on
their own — that is US1.

---

## Phase 3: User Story 1 - See which session I landed on after switching projects (Priority: P1) 🎯 MVP

**Goal**: After a project switch the row holding the incoming project's current session is already
open, and that session's row is marked in a way that survives greyscale.

**Independent Test**: Two projects, each with a running session in a worktree. Switch from one to
the other; without further clicks the incoming project's row holding the current session is open and
that session's row is marked as current.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T010 [P] [US1] Failing test in `crates/micold-client/tests/switch_active.rs`: the switch path makes the incoming project's current session's location read as open, for a worktree *and* for Default (US1 scenarios 1–2), leaves every other location as it was (scenario 3, FR-004), opens nothing when the incoming project has no session (scenario 4, FR-013), and does not close an already-open row (scenario 5)
- [X] T011 [P] [US1] Failing test in `crates/micold-client/tests/switch_active.rs`: view state still does not carry between projects (FR-007) — the only location opened on the user's behalf is the incoming project's
- [X] T012 [P] [US1] Failing test in `crates/micold-client/tests/app_state.rs`: commit-on-clear — when `active_session` changes, **including to `None`**, the outgoing forced location is committed to `expanded` / `default_expanded` before re-derivation, so it stays open (contract §2.3, FR-001c, invariant I3)
- [X] T013 [P] [US1] Failing test in `crates/micold-client/tests/app_state.rs`: a collapse suppressed under T007 stops suppressing as soon as `active_session` changes, so the next reveal is not swallowed by an old collapse (contract §2.1, SC-006, invariant I2) — the half of FR-005 that only exists once something arms a reveal
- [X] T014 [P] [US1] Failing test in `crates/micold-client/tests/sidebar_state.rs`: `WorktreesLoaded` and the binary's re-discovery neither close a forced row nor clear `reveal_suppressed_for` (contract §2.2, SC-008) — the file that already covers `set_worktrees`'s pruning
- [X] T015 [P] [US1] Failing test in `crates/micold-client/src/ui/material/type_role_mapping.rs`: the new current-session role resolves to `typography::LABEL_MEDIUM` (12/16 at weight 500 — the same size as `SIDEBAR_SESSION`, differing only in weight), and `TypeRole::ALL` and the mapping table still agree on the count
- [X] T016 [P] [US1] Failing test in `crates/micold-client/tests/type_role_call_sites.rs`: the current row's heavier name comes from a role in the scale, never an ad-hoc font weight at a call site
- [X] T017 [P] [US1] Failing test in `crates/micold-core/tests/tokens_contrast.rs`: the current row's pairing (`secondary_container` / `on_secondary_container`) still clears AA in both schemes — a check that the mark's colour half is unchanged, not a new token

### Implementation for User Story 1

- [X] T018 [US1] Add `State::set_current_session(Option<SessionId>)` in `crates/micold-client/src/features/session.rs` performing, in order: commit the outgoing forced location (I3, on **every** transition including one to `None`) → clear `reveal_suppressed_for` (I2) → arm `pending_reveal_scroll` **only when the new value is `Some`** (I5, contract §3.0a). Arming on a clear would leave the field armed with no target — armed forever by I4, then applied to whatever row appeared next — and FR-001a forbids scrolling when the user closes the session they were on
- [X] T019 [US1] Route `restore_after_activation`'s write to `active_session` (`crates/micold-client/src/features/session.rs:92`) through `set_current_session`, so the project switch arms a reveal without a new message
- [X] T020 [US1] Add `TypeRole::SidebarSessionCurrent` in `crates/micold-client/src/ui/material/text.rs` — variant, `resolved()` arm mapping to `typography::LABEL_MEDIUM`, `ALL` 11 → 12 (including the `[TypeRole; 11]` length and the type-level prose that says "the eleven the application actually distinguishes"), and `name()`. Deliberately no new core token: `LABEL_MEDIUM` already carries exactly the right figures, and adding a `SIDEBAR_SESSION_CURRENT` alias would widen `typography::SIDEBAR` and the core tests that enumerate it for no gain (research R9 stays true — `micold-core` is untouched)
- [X] T021 [US1] (reopened and closed by BUG-001, T063) Render the current session row's name at that role in `crates/micold-client/src/ui/material/tree_view.rs`, reached by a chainable builder on the existing `TreeItem` (Principle VIII), leaving the `secondary_container` pill exactly as it is and touching neither the lifecycle tint nor the activity dot (contract §4.2, §4.3)
  - **Reopened (BUG-001)**: the role is selected here and then discarded one layer down —
    `Ellipsized::at_role` keeps `role.size()` and drops `role.font()`, so the row renders at 400
    like its siblings. Closes only when the weight is on screen, not when the role is named.
  - **Closed again (2026-08-19)**: `Ellipsized` now carries the role's font, so the role this task
    selects reaches the pixels. §A-level proof is T061/T062; the on-screen half is T065.
- [X] T022 [US1] Set that builder from `session_tree_item` in `crates/micold-client/src/ui/sidebar.rs:455` off the `selected` value it already computes — no second source of truth for "is current"
- [X] T023 [US1] Document the reveal and the mark in `docs/user-guide/worktrees-and-sessions.md` under `## Starting, switching, and closing sessions`: switching a project opens the row holding the session you land on and marks it, closing that row sticks, and the mark is independent of keyboard focus and of run state (FR-014, FR-015)

**Checkpoint**: The reported bug is gone for lists that fit the panel. US1 is shippable alone.

---

## Phase 4: User Story 2 - The current session is actually on screen (Priority: P2)

**Goal**: The revealed row is inside the viewport, without moving the panel when it already was.

**Independent Test**: A project with more locations than the panel is tall and the current session's
location near the bottom. Switch to it; the current session's row is visible without scrolling. Then
scroll manually and confirm nothing snaps back.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T024 [P] [US2] Failing test in `crates/micold-client/tests/sidebar_tree.rs`: `row_heights` agrees with `density::height(LIST_ROW_ONE_LINE_BASE | LIST_ROW_TWO_LINE_BASE, step)` plus `spacing::XS` at both densities, sharing the figures `crates/micold-client/src/ui/material/anatomy_size.rs:471,519` already asserts. **This is the feature's highest-value test** — a computed height that drifts from the rendered one scrolls to the wrong place silently (research R6, R10)
- [X] T025 [P] [US2] Failing test in `crates/micold-client/tests/sidebar_tree.rs`: `scroll_target` returns `None` for an already-visible row (§6.1, FR-009), the minimal clamped offset otherwise — up for a row above, down for one below, never centred (§6.2, FR-008) — and `None` at `viewport_height == 0.0` (§6.3)
- [X] T026 [P] [US2] Failing test in `crates/micold-client/tests/material_builder_api.rs`: `Scrollable` still constructs with its required inputs and terminates in `.into()`, with `id` and `on_viewport_resize` chainable rather than positional, and unset by default (contract scrollable-viewport §1.2, §2.5)
- [X] T027 [P] [US2] Failing test in `crates/micold-client/tests/app_state.rs`: `pending_reveal_scroll` is drained only when a row for the current session exists in the projection, stays armed otherwise, is `None` afterwards (§6.4, §6.5, invariant I4), and is never armed at all by a transition to `None` (I5, FR-001a)

### Implementation for User Story 2

- [X] T028 [US2] Add `Scrollable::id(impl Into<iced::widget::Id>)` in `crates/micold-client/src/ui/material/scrollable.rs`, forwarding to the rendering stack's `scrollable::id(...)` — on the scrollable itself, never on a wrapper (contract scrollable-viewport §1.1, §1.3)
- [X] T029 [US2] Add `Scrollable::on_viewport_resize(impl Fn(Size) -> M)` in the same file, backed by iced 0.14's `Sensor` (`on_show` + `on_resize`), reporting the **viewport's** size and firing on first layout as well as on change (§2.1–§2.3). Verify the `Sensor` forwards `operate`, or `scroll_to` dies at it — the trap `crates/micold-client/src/ui/material/ripple.rs:248-256` documents
- [X] T030 [US2] Keep the two scroll subscriptions independent: setting `on_viewport_resize` must not disturb `on_scroll` / `on_scroll_offset` or their existing "offset form wins" rule at `crates/micold-client/src/ui/material/scrollable.rs:100-109` (§2.4)
- [X] T031 [US2] Implement `row_heights` and `scroll_target` in `crates/micold-client/src/features/sidebar.rs` as pure functions over the ordered rows (research R6) — no renderer, no iced types. Decide and document where inter-row `spacing::XS` lives (inside each row's height is the simpler choice) and how the `f32` target reconciles with the existing `sidebar_scroll_offset: u32` at `crates/micold-client/src/app.rs:569`, including which way it rounds — a half-pixel the wrong way lands a row just off the viewport edge
- [X] T032 [US2] Add `Message::SidebarViewportResized(f32)` in `crates/micold-client/src/app.rs` writing `sidebar_viewport_height`, and wire the sidebar's `Scrollable` at `crates/micold-client/src/ui/sidebar.rs:137` with an id and `on_viewport_resize`
- [X] T033 [US2] Drain `pending_reveal_scroll` into `iced::widget::operation::scroll_to` in `crates/micold-client/src/main.rs`, only once the projection holds a row for the current session (§6.4) — the async path where the worktree list arrives after the switch (research R7)
- [X] T034 [US2] Document scroll-into-view in `docs/user-guide/worktrees-and-sessions.md` under `## Starting, switching, and closing sessions`: the panel scrolls only when the row is not already visible, and your own scrolling is never overridden until the app next moves you (FR-009, FR-010, SC-007)

**Checkpoint**: US1 + US2 — the reveal works for a 30-location project.

---

## Phase 5: User Story 3 - Reveal wherever the app moves me (Priority: P2)

**Goal**: The same reveal on every path where the app makes a session current, and pointedly *not*
on the paths where the user did it themselves or where nothing becomes current.

**Independent Test**: Start a session and confirm the reveal; click a session and confirm nothing
opens or scrolls; close the current session and confirm nothing is marked and no successor is
promoted.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T035 [P] [US3] Failing test in `crates/micold-client/tests/app_state.rs`: `Message::SessionStarted` arms a reveal and marks the new session (US3 scenario 2, FR-001)
- [X] T036 [P] [US3] Failing test in `crates/micold-client/tests/app_state.rs`: `Message::SessionSelected` marks the clicked session and arms **nothing** — no location opened, no scroll (US3 scenario 4, FR-006)
- [X] T037 [P] [US3] Failing test in `crates/micold-client/tests/app_state.rs`: `SessionCloseRequested` / `SessionRemoveConfirmed` leave no session current, promote no successor, arm no scroll, and close no row on the user's behalf — the outgoing location is committed open instead (US3 scenario 3, FR-001a, FR-001c, contract §3.0a)
- [X] T038 [P] [US3] Failing test in `crates/micold-client/tests/forget_project.rs`: forgetting the **active** project clears `active_session` (`crates/micold-client/src/app.rs:877`) through the same path — outgoing row committed, nothing armed. The existing file for this behaviour, so the new rule is asserted where the old one already is
- [X] T039 [P] [US3] Failing source gate in a new `crates/micold-client/tests/current_session_writers.rs`: every writer of `active_session` goes through `set_current_session` except `Message::SessionSelected`, and the gate names the six writers contract §3's table enumerates — the two that set (`restore_after_activation`, `SessionStarted`), the one exemption (`SessionSelected`), and the four that clear (close, remove, project forgotten, `reconcile_catalog`'s dangling-pointer drop at `crates/micold-client/src/main.rs:2401`). Written in the idiom of this repo's existing gates (`tests/logical_state_ownership.rs`, `tests/material_boundary.rs`) — it is what makes contract §3.0 a rule rather than a list a future caller quietly falls off

### Implementation for User Story 3

- [X] T040 [US3] Route `Message::SessionStarted`'s write to `active_session` (`crates/micold-client/src/app.rs:1279`) through `set_current_session`
- [X] T041 [US3] Leave `Message::SessionSelected` (`crates/micold-client/src/app.rs:1286`) writing `active_session` directly, with a comment naming FR-006 as the reason it is the one excluded transition, so the source gate's exemption is explained where the exemption lives
- [X] T042 [US3] Route the close and remove arms (`crates/micold-client/src/app.rs:1357`, `:1390`) through `set_current_session` so the outgoing row is committed open (FR-001c) while nothing is armed (I5) — and add nothing that promotes a successor
- [X] T043 [US3] Route the two remaining clears through `set_current_session` for the same reason: the active project being forgotten (`crates/micold-client/src/app.rs:877`) and `reconcile_catalog` dropping a dangling pointer (`crates/micold-client/src/main.rs:2401`). Both are app-initiated transitions to `None`; without this they skip the commit and the row of the session that just vanished snaps shut, taking its siblings with it
- [X] T044 [US3] Confirm FR-010a needs no motion: the reveal uses the same expansion path as a user-initiated expand, which is instant today (contract §7.1, research R8). If that changes later, the reveal must inherit it rather than be special-cased — record this in the doc comment on `set_current_session` in `crates/micold-client/src/features/session.rs`
- [X] T045 [US3] Document the remaining paths in `docs/user-guide/worktrees-and-sessions.md` under `## Starting, switching, and closing sessions`: a new session is revealed, a session you click is marked but nothing moves, closing the session you are on leaves nothing current, and after a cold start nothing is current until you pick or start a session (FR-001d, research R12)

**Checkpoint**: Every path the app moves you on ends in the same revealed state (SC-004), and the
paths it does not are provably inert.

---

## Phase 6: User Story 4 - Reveal it even when my filters would hide it (Priority: P3)

**Goal**: Exactly one location escapes the filters — the one holding the current session — in its
normal position, saying why it is there.

**Independent Test**: Turn on a tag filter that excludes the location holding the current session,
switch away and back. That location appears, opened, marked, chipped; every other excluded location
stays hidden.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T046 [P] [US4] Failing test in `crates/micold-client/tests/features_sidebar.rs`: the exemption admits the current session's location past the tag filters **and** past the hidden-agent setting, resolving against *all* worktrees rather than `visible_worktrees` — which excludes rows earlier, in `crates/micold-client/src/features/worktree.rs:77` (§5.1, FR-011, US4 scenarios 1 and 3)
- [X] T047 [P] [US4] Failing test in `crates/micold-client/tests/features_sidebar.rs`: no other excluded location is admitted, and the exemption ends when that location stops holding the current session — while its *open* state, committed by §2.3, survives (§5.2, §5.3, FR-012, SC-005, US4 scenario 4)
- [X] T048 [P] [US4] Failing test in `crates/micold-client/tests/features_sidebar.rs`: `shown_for_current_session` is `true` only for a node the filters would have excluded, `false` for one they admit on their own (§5.4, FR-012a)
- [X] T049 [P] [US4] Failing test in `crates/micold-client/tests/features_sidebar.rs`: the exempt row sits where it would sit unfiltered (§5.5), and `available_tag_filters` gains nothing from it — the same rule a hidden agent worktree obeys at `crates/micold-client/src/features/sidebar.rs:177-179` (§5.6)

### Implementation for User Story 4

- [X] T050 [US4] Add `shown_for_current_session: bool` to `WorktreeNode` in `crates/micold-client/src/features/sidebar.rs`, leaving `DefaultNode` alone — it is already exempt from tag filtering at `:147-149`, so the flag would be permanently false there
- [X] T051 [US4] Change `filtered_worktree_tree` in `crates/micold-client/src/features/sidebar.rs` from "filter `worktree_tree`" to "filter, then re-admit the one location holding the current session", preserving unfiltered order and setting the flag only on a re-admitted node
- [X] T052 [US4] Render the FR-012a chip in the row's existing `TreeItem::tags(...)` slot from `crates/micold-client/src/ui/sidebar.rs`, reusing the label-only chip precedent `Tag::Agent` sets — and **not** adding a `Tag::Current` to `micold-core::naming` (research R5)
- [X] T053 [US4] Document the exemption in `docs/user-guide/worktrees-and-sessions.md` under `### Filtering worktrees by tag`: the location holding your current session is always listed, in its usual place, carrying a chip that says why — and it is the only exception your filters get

**Checkpoint**: All four stories independently functional.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T054 Run the whole automated gate: `mise run test`, and record which of quickstart §A's rows each new test satisfies
- [X] T055 Add the two assertions §A cannot currently claim: exactly one session row carries the mark when several sessions share a location (FR-002, contract §4.1) in `crates/micold-client/tests/features_sidebar.rs`, and the mark is independent of `terminal_focused` and of `lifecycle` (FR-014, FR-015, §4.4) in `crates/micold-client/tests/terminal_focus.rs`
- [X] T056 Run quickstart §B B1–B6 with the repo's `visual-pass` skill and fill in the recording table in [quickstart.md](./quickstart.md) — a step that fails is a defect, not a note. B1, B2 and B4 are the three headline claims and none can be automated; if §B was not run, say so rather than leaving the table blank

**§B was run on 2026-08-18** (T056/T057), against the client rather than the showcase, with the
`visual-pass` skill. B1, B3, B5 and B6 pass. **B2 and B4 fail**, and per T056's own rule a step that
fails is a defect: [BUG-001](./bugs/BUG-001.md) — the current session's name is not drawn heavier,
because `Ellipsized::at_role` keeps only the role's size — and [BUG-002](./bugs/BUG-002.md) — a
project switch never scrolls the current session's row into view in a list long enough to need it.
Both are FR-level failures with no fix in this change; the recording table in
[quickstart.md](./quickstart.md) carries the measurements and the evidence. **T058's branch is now
answerable**: R4's fallback was for a 500 weight that proved too subtle, and the honest finding is
that the weight is absent rather than subtle — so BUG-001 comes first, and R4's question can only be
re-asked once there is a weight on screen to judge.

- [X] T057 Capture the §B screenshots with `mise run screenshot` — B1's first frame after a switch and B2's pair of schemes — and check B2's pair in greyscale, which is the only real test of FR-003a
- [X] T058 If §B judges the 500-weight name too subtle, apply R4's pre-argued fallback (an outline on the pill) rather than inventing a third cue — and record the decision in [research.md](./research.md) R4
- [X] T059 [P] Cross-cutting docs review in `docs/`: confirm the two edited sections still read as one narrative and that nothing else in the user guide now describes the old collapsed-after-switch behaviour
- [X] T060 Confirm CI is green on Linux, macOS and Windows for `.github/workflows/ci.yml` (Principle VI) — the feature has no platform branch, so a platform-specific failure means a geometry assumption leaked

---

## Phase 8: Bugfix BUG-001 — the current session's name is not drawn heavier

**Goal**: The non-colour half of the mark reaches the pixels. `Ellipsized` carries the font its
role asks for, so `SidebarSessionCurrent`'s 500 weight is visible on the current session's row
while its siblings stay at 400 (FR-003a, FR-003a-i, SC-005a).

**Independent Test**: Crop the current session's row and a sibling session's row at identical
geometry in both schemes; the current name's normalised ink area exceeds the sibling's by the
margin a 400 → 500 step produces, not by the 0.3% measured on 2026-08-18.

### Tests for BUG-001 (MANDATORY — Constitution Principle I) ⚠️

> Written FIRST; confirmed to FAIL before implementation.
>
> **How that was confirmed here (2026-08-19)**: not in the usual order. Both tests name a helper the
> fix introduces (`needs_reshape`), so they could not compile against the unfixed widget — they were
> written and implemented in one pass, and the failure was demonstrated afterwards by reverting the
> two lines of fix (`at_role`'s font, and the font in the reshape key) and re-running:
> `a_role_brings_its_font_and_not_only_its_size` and `a_change_of_font_alone_re_shapes_the_label`
> both FAILED, the other ten passed. That is the same evidence Principle I asks for, obtained in the
> other order, and it is recorded rather than glossed.

- [X] T061 [BUG-001] Add a failing unit test in `crates/micold-client/src/ui/material/ellipsized.rs`:
  `at_role` carries the role's font, not only its size — assert the constructed widget's font is
  `TypeRole::SidebarSessionCurrent.font()`, that it differs from `SidebarSession`'s, and that the two
  roles' *sizes* are equal (which is why the font is the only thing left to tell them apart). Assert
  too that `Ellipsized::new` still defers to the renderer's default font, so the size-only
  constructor's appearance does not move. This is the assertion `type_role_mapping.rs` and
  `type_role_call_sites.rs` structurally cannot make: they check which role is *named*, and the
  defect is entirely in what the drawing widget does with it (BUG-001, "Why no gate saw it")
- [X] T062 [P] [BUG-001] Add a failing test that a change of font alone re-shapes the label. The
  cached paragraph is keyed on content, width and size; the two sidebar roles are deliberately the
  *same size*, so without the font in that key a row that becomes current would keep the paragraph
  shaped at 400 and the fix would not show. Extract the staleness rule into a pure function and
  assert it says "re-shape" when only the font differs

### Implementation for BUG-001

- [X] T063 [BUG-001] Give `Ellipsized` a `font: Option<Font>` field — `Some(role.font())` in
  `at_role`, `None` in `new`, resolved against `renderer.default_font()` at layout time — and use it
  in place of `renderer.default_font()` in the shaping template
  (`crates/micold-client/src/ui/material/ellipsized.rs:64,128`), which is the one place both the
  measurement and the drawn paragraph come from. `Option` rather than a bare `Font`, because the
  application's default font *is* Roboto (`shell/startup.rs:84`) while `Font::DEFAULT` is not — a
  bare default would silently restyle every size-only call site. Add the font to the reshape key.
  Closes T021
- [X] T064 [BUG-001] Audit every other `Ellipsized::at_role` call site for the same shape: a role
  chosen for a weight the widget could not carry. Record which call sites change appearance as a
  result — a widget whose whole job is text has until now had no way to say which face to draw it in,
  so the fix is not necessarily confined to the sidebar
  - **Result**: only two other `at_role` call sites exist, both showcase poses at `TypeRole::Body`
    (weight 400, which is what the application's default font already was) — nothing moves there.
    `TreeView`'s unselected `label_role` is `SidebarName` = `BODY_SMALL` = 400, also unchanged. One
    other thing does change and should: the showcase's own tree pose
    (`showcase/sections/surfaces.rs:259`, `selected_label_role(TypeRole::Label)` = `LABEL_MEDIUM`)
    was exhibiting the same defect and now draws the weight it advertises

### Verification for BUG-001

- [X] T065 [BUG-001] Re-run quickstart §B2 with the `visual-pass` skill against the fixed build, in
  both schemes, and record the ink-area measurement in [quickstart.md](./quickstart.md) beside the
  2026-08-18 numbers. Only then is T058's R4 question answerable — whether a 500 weight that is
  actually on screen is *sufficient*, or whether R4's outline fallback is still needed
  - **Done 2026-08-19**: ink-area ratio **1.222** dark and **1.221** light, against 1.003/1.002
    before the fix; measured on the luminance channel, so it is also the greyscale check SC-005 and
    FR-003a ask for. **R4's fallback is not needed** — the weight carries on its own in both
    schemes, so T058 stands as closed rather than reopened

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — **blocks every user story**
- **US1 (Phase 3)**: depends on Phase 2
- **US2 (Phase 4)**: depends on Phase 2; independently testable, but only *observable* alongside US1's reveal
- **US3 (Phase 5)**: depends on Phase 2 and on T018 (`set_current_session`, introduced in US1)
- **US4 (Phase 6)**: depends on Phase 2; the exemption is meaningless without US1's reveal
- **Polish (Phase 7)**: depends on the stories being delivered

### Within Each User Story

- Tests are written and FAIL before implementation (Principle I). This holds **across** phases too:
  T007 sits in Phase 2 ahead of T008 for exactly that reason — `app.rs` is a render-free reducer, so
  the GUI-wiring exception does not reach the toggle's decision logic
- The predicate and metrics land in `features/` before their `ui/` glue
- The user-guide task ships in the same change as its story (Principle VII)

### Parallel Opportunities

- T010–T017 (US1 tests) span six files; T012 and T013 share `tests/app_state.rs` and should be sequenced or committed together
- T024–T027 (US2 tests) — T024/T025 share `tests/sidebar_tree.rs`; T026 and T027 are independent
- T035–T039 (US3 tests) — T038 and T039 are genuinely parallel; T035–T037 share `tests/app_state.rs`
- T046–T049 (US4 tests) all touch `tests/features_sidebar.rs` — sequence them
- T028/T029 (`scrollable.rs`) and T031 (`features/sidebar.rs`) are different files and can run in parallel
- The four user-guide tasks (T023, T034, T045, T053) touch the same file and must not run in parallel

**Parallel example — User Story 1 tests:**

```bash
Task: "Failing test for the switch path in crates/micold-client/tests/switch_active.rs"
Task: "Failing test for re-discovery in crates/micold-client/tests/sidebar_state.rs"
Task: "Failing test for the role mapping in crates/micold-client/src/ui/material/type_role_mapping.rs"
Task: "Failing test for the pill's contrast in crates/micold-core/tests/tokens_contrast.rs"
```

---

## Implementation Strategy

### MVP (User Story 1 only)

1. Phase 1 → Phase 2 → Phase 3
2. **STOP and VALIDATE**: quickstart §B1, §B2 and §B3 — the reported bug is gone, the mark reads in
   greyscale, a close sticks
3. Shippable. US1 alone fixes the reported problem for any project whose location list fits the
   panel

### Incremental Delivery

1. Setup + Foundational → open-ness is derived, nothing visible changes
2. **+ US1** → the reveal and the mark (MVP)
3. **+ US2** → it is on screen even in a 30-location project (§B4)
4. **+ US3** → the remaining paths, and provably inert on the user's own actions (§B6)
5. **+ US4** → past the filters (§B5)

Each step leaves the app in a shippable state; none of them can regress the one before, because
each story's assertions stay in the suite.

### Where the risk is (research R10)

1. **T024 / T031** — the scroll arithmetic. The only place a wrong answer is silent, and where the
   `f32` / `u32` offset reconciliation has to be decided rather than assumed
2. **T029** — the `Sensor` wrapper. A wrapper that does not forward `operate` swallows `scroll_to`
   for its whole subtree
3. **T018 / T039** — the arming rule and its gate. Getting "only a transition to `Some` arms" wrong
   is what would leave a scroll armed with no target
4. **T020–T022** — the weight cue. Cheap to build, and only §B can judge it sufficient
5. Everything else — predicate and flag changes in already-tested modules

---

## Notes

- 60 tasks: 2 setup, 7 foundational, 14 US1, 11 US2, 11 US3, 8 US4, 7 polish
- `micold-core` is untouched by every task except T017, which only *asserts* an existing pairing
- Nothing here is persisted; `reveal_suppressed_for` and the two scroll fields die with the process
- Commit after each task or logical group; stop at any checkpoint to validate a story on its own
