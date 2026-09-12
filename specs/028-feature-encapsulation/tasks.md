---

description: "Task list for 028-feature-encapsulation"
---

# Tasks: Feature Encapsulation — Own Your Messages, Own Your State

**Input**: Design documents from `/specs/028-feature-encapsulation/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I, the Red step is mandatory — but it takes two different
shapes here, and neither is skipped:

- **For the conversions (US1, US2)** the pre-existing suite *is* the behaviour specification.
  FR-021 freezes it, `scripts/check-assertions-frozen.sh` enforces the freeze from T004 onward, and
  no production code in either story carries a rule that suite does not already exercise. Writing a
  new test per conversion would assert what is already asserted; the plan's Constitution Check
  records this and does not invoke the GUI-glue exception.
- **For the guards (US3)** the Red step is the non-vacuity probe (FR-017, SC-005): the forbidden
  violation is injected, the named failure is observed and recorded, and only then is the guard
  relied upon. Each guard has its own probe task, and an injection that fails to *compile* has
  demonstrated nothing.

**Documentation**: Per Constitution Principle VII, nothing user-facing changes, so the user guide
needs no edit. The maintainer-facing documentation is the deliverable and ships inside each story:
T018 (what each feature owns), T041 (what each feature remembers), T027 and T050 (what each guard enforces).

**Cross-platform**: Per Constitution Principle VI, the seven guards join `ci.yml`'s all-platforms
step in T052 — the omission feature 021's T058 and T077 both recorded and left open.

**Organization**: Tasks are grouped by user story. US3 appears in two phases because a guard lands
*after* the conversion it describes (plan.md, "Implementation phases"): a guard that has to be
relaxed to let its own migration through is not holding anything.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1, US2, US3 — maps to the user stories in [spec.md](./spec.md)
- Paths are repository-relative. `<client>` abbreviates `crates/micold-client`.

---

## Phase 1: Setup

**Purpose**: Establish the baseline the success criteria are measured against, and the file that
records every assertion whose spelling this feature changes.

- [X] T001 Confirm the branch is green before the first conversion — run the whole-workspace suite via `mise run test` (task defined in `mise.toml`) and record the pass, establishing FR-006's starting point
- [X] T002 [P] Reproduce the four baseline measurements from [quickstart.md](./quickstart.md) §A.1–A.4 against `crates/micold-client/src/app.rs` and `crates/micold-client/src/features/` and confirm 119 root variants, 44 flat root fields, 1 of 10 features nested
- [X] T003 [P] Create `specs/028-feature-encapsulation/assertion-adjudications.md` with the per-task heading structure contract B1 requires (task id, the path rename that caused the change, the assertions affected)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Turn FR-021 from a sentence into something that fails. This must precede every
conversion, not follow it — US1's first commit already renames assertion text
(`Message::AboutOpened` → `Message::Help(help::Msg::AboutOpened)`), so a freeze switched on at the
end would be adjudicating a thousand renames retroactively instead of one commit at a time.

**Departure from plan.md, recorded rather than silent**: plan.md places this in P9. It is moved
here because a report nobody must act on is the failure mode this feature exists to correct, and
FR-021 says the check MUST fail this feature's branch — which is only meaningful while there is
still work left to fail.

**⚠️ CRITICAL**: No conversion work begins until this phase is complete.

- [X] T004 Extend `scope_reason()` in `scripts/check-assertions-frozen.sh` to recognise feature 028 the same way it recognises 021 — both the `specs/028-feature-encapsulation/` path test and the `*028*` branch-name case — per [research.md](./research.md) §R7
- [X] T005 Verify the freeze now blocks rather than reports: change one assertion's text in any file under `crates/micold-client/tests/`, run `scripts/check-assertions-frozen.sh`, confirm a non-zero exit naming that assertion, revert the change, and record the observed message in `specs/028-feature-encapsulation/assertion-adjudications.md`

**Checkpoint**: The freeze is live. Every conversion below must leave `mise run test` green and
either change no assertion text or adjudicate what it changed.

---

## Phase 3: User Story 1 - Change a feature without touching the root (Priority: P1) 🎯 MVP

**Goal**: Each of the nine unconverted features declares its own `Msg` and exposes one reducer
entry point, so the root gains one arm per feature instead of one per interaction. Root `Message`
falls from 119 variants to 15.

**Independent Test**: Add one interaction to any converted feature and count the files changed —
the feature module and its view, and neither `app.rs`'s vocabulary nor its reducer (SC-001,
Acceptance Scenario 1).

**Entry shape per feature** (contract M2, [data-model.md](./data-model.md) §1.1): an arm belongs in
shape B (`shell/<n>.rs`, returns `iced::Task`) when it must perform an effect, and in shape A
(`features/<n>.rs`, returns `Vec<Outcome>`) otherwise. A feature may expose both, as
`worktree_form` does today (18 arms pure, 4 effectful). `connection` is expected to be shape B only
— it writes no state at all and is the one feature absent from `OWNERS`, so forcing it to return
`Vec::new()` twelve times is the ceremony FR-005 forbids ([research.md](./research.md) §R3).

**Per-conversion definition of done** (contract M5, FR-006): one commit; the variants listed for
that feature in [data-model.md](./data-model.md) §2 moved out of `app::Message` into
`features::<n>::Msg` with the feature-name prefix dropped from each variant (M1); the root arms in
`app.rs::State::update` and `main.rs::update_inner` replaced by one wrapper arm; every emit site
under `<client>/src/ui/` updated; `mise run test` green; assertion spellings adjudicated. Order is
smallest-first per [research.md](./research.md) §R9, so the pattern is proven cheap before it
reaches the 37-variant feature.

### Implementation for User Story 1

- [X] T006 [US1] Nest `help` — 3 variants → `help::Msg` in `crates/micold-client/src/features/help.rs`, one root arm in `crates/micold-client/src/app.rs`
- [X] T007 [US1] Nest `window` — 2 variants → `window::Msg` in `crates/micold-client/src/features/window.rs`
- [X] T008 [US1] Nest `notifications` — 2 variants → `notifications::Msg` in `crates/micold-client/src/features/notifications.rs`
- [X] T009 [US1] Nest `settings` — 10 variants → `settings::Msg` in `crates/micold-client/src/features/settings.rs`, with the effectful arms in `crates/micold-client/src/shell/settings.rs`
- [X] T010 [US1] Nest `sidebar` — 10 variants → `sidebar::Msg` in `crates/micold-client/src/features/sidebar.rs`
- [X] T011 [US1] Nest `connection` — 12 variants → `connection::Msg` in `crates/micold-client/src/features/connection.rs` with the entry point in `crates/micold-client/src/shell/connection.rs` (shape B only); this is the task that exercises the two-shape rule on a 12-variant feature rather than discovering it on the 37-variant one
- [X] T012 [US1] Nest `worktree` — 18 variants → `worktree::Msg` in `crates/micold-client/src/features/worktree.rs`, **including `Message::TextCopyRequested`**, which is attributed to `worktree` despite its generic name: its single emit site is `crates/micold-client/src/ui/mod.rs:470` ([research.md](./research.md) §R2)
- [X] T013 [US1] Nest `project` — 19 variants → `project::Msg` in `crates/micold-client/src/features/project.rs`
- [X] T014 [US1] Nest `session` — 37 variants → `session::Msg` in `crates/micold-client/src/features/session.rs`, the largest conversion and the one the ordering exists to de-risk
- [X] T015 [US1] Confirm `worktree_form` needs no conversion — verify its single `Message::WorktreeForm` arm already wraps its 22-variant vocabulary and leave `crates/micold-client/src/features/worktree_form.rs` unchanged
- [X] T016 [US1] Verify the root vocabulary is exactly 15 variants — 10 feature wrappers plus the 5 cross-cutting ones enumerated in [data-model.md](./data-model.md) §2 — using [quickstart.md](./quickstart.md) §A.1 against `crates/micold-client/src/app.rs`
- [X] T017 [US1] Verify Acceptance Scenario 2 still holds — confirm no feature reducer names another feature's data by running `crates/micold-client/tests/feature_write_isolation.rs` unmodified, via `cargo test -p micold-client --test feature_write_isolation`
- [X] T018 [US1] Document what each feature owns — add or extend the module header in each of the nine converted files under `crates/micold-client/src/features/` to state the vocabulary that feature declares and its entry shape (Principle VII)

**Checkpoint**: US1 is independently shippable. The root enum is permanently smaller, the
application is green, and the work can stop here with value delivered (SC-009).

---

## Phase 4: User Story 3 - The pattern cannot be opted out of, part 1: the vocabulary guards (Priority: P1)

**Goal**: G1 and G3 make Story 1's shape non-optional. They land *after* the conversions they
describe, deliberately.

**Independent Test**: Inject each forbidden violation and confirm the guard reports it by name
(Acceptance Scenarios 1 and 3).

### Guards for User Story 3 (Red step — FR-017, SC-005) ⚠️

> Each guard is observed failing its own injected violation before it is relied upon. Record the
> observed failure message beside its task. An injection that fails to compile has demonstrated
> nothing — check that the test actually ran.

- [X] T019 [P] [US3] Write G1 in `crates/micold-client/tests/root_vocabulary_is_cross_cutting.rs` per [contracts/guards.md](./contracts/guards.md) — resolve each `app::Message` variant's owner set from the `features::`, `shell::` and `overlay::registry::` calls its arms make; fail when the set is exactly one feature and name it
- [X] T020 [P] [US3] Give G1 its third verdict in `crates/micold-client/tests/root_vocabulary_is_cross_cutting.rs` — an empty owner set is **reported, not failed**, and each such variant carries a written reason in a `NO_OWNER` allowlist (FR-013)
- [X] T021 [P] [US3] Give G1 its `ALLOWED: &[(&str, &str)]` allowlist plus the reverse check that the allowlist names only live violations, following `crates/micold-client/tests/feature_write_isolation.rs`'s `the_allowlist_names_only_live_violations` (FR-016)
- [X] T022 [US3] Non-vacuity probe for G1 — add a variant to `app::Message` in `crates/micold-client/src/app.rs` whose only arm calls `features::help::about_opened`, run `cargo test -p micold-client --test root_vocabulary_is_cross_cutting`, observe the failure naming `help`, revert with `git checkout -- crates/micold-client`, and record the message in `specs/028-feature-encapsulation/assertion-adjudications.md`
- [X] T023 [US3] Extend `crates/micold-client/tests/feature_registration_cost.rs` with G3 — every module under `src/features/` other than `mod.rs` that declares `pub enum Msg` must expose shape A or shape B; a module declaring no `Msg` passes with no allowlist entry (FR-005, FR-015)
- [X] T024 [US3] Non-vacuity probe for G3 — add `crates/micold-client/src/features/probe.rs` declaring `pub enum Msg { Tick }` and no `update`, run `cargo test -p micold-client --test feature_registration_cost`, observe the failure naming `probe`, revert, and record the message

### Implementation for User Story 3, part 1

- [X] T025 [US3] Decide and pin `Message::ScrolledBeneathOverlay` (FR-020, contract B2) — it is declared in `crates/micold-client/src/app.rs:186`, matched at `:1007`, exercised by four assertions in `crates/micold-client/tests/overlay_dismissal_delta.rs`, and emitted by nothing; record it as G1's `NO_OWNER` entry with the written reason that its behaviour is specified by tests but unreachable in the running application, and assert in G1 that the reported no-owner set is exactly that variant
- [X] T026 [US3] Verify SC-002 and SC-004 — the root vocabulary holds no variant produced and consumed by exactly one feature, and 10 of 10 features have a vocabulary and an entry point, up from 1 of 10 ([quickstart.md](./quickstart.md) §A.4)
- [X] T027 [US3] Document each guard in its own file header — `crates/micold-client/tests/root_vocabulary_is_cross_cutting.rs` and `crates/micold-client/tests/feature_registration_cost.rs` — stating the rule it enforces, how an exception is granted, and the probe that showed it non-vacuous (Principle VII)

**Checkpoint**: Stories 1 and 3's vocabulary half are complete and independently valuable. SC-002,
SC-004 and two of SC-005's three probes are satisfied.

---

## Phase 5: User Story 2 - State that is nobody else's business lives nowhere else (Priority: P2)

**Goal**: Every field the ownership map assigns to a feature becomes a field of that feature's own
`State` struct, declared in that feature's module. `app::State` goes from 44 flat public fields to
one field per feature plus the declared shared member.

**Independent Test**: Pick one feature, move its fields, and confirm the root struct's flat field
count falls by exactly the fields moved with no behaviour change observable in the existing suite.
Each feature is independently valuable and independently revertible.

**⚠️ Invariant S3 (FR-009), binding on every task in this phase**: no task may introduce
`state.<feature> = <feature>::State::default()` or `..Default::default()` over a feature struct. A
wholesale replacement is a lifetime change wearing a refactor's clothes. Fields reset together are
reset by a named operation on the feature module ([research.md](./research.md) §R5).

The per-feature field assignments are the Owner column of [data-model.md](./data-model.md) §3. All
43 owned fields land in exactly one feature struct; the QUALIFIES/COMPOSITION/SHELL/ROOT
classification decides only whether FR-007a's further move into a component applies on top — which,
today, it does for none of them.

### Implementation for User Story 2

- [X] T028 [US2] Move `notifications`' 1 field (`notify`) into `features::notifications::State` in `crates/micold-client/src/features/notifications.rs`, held as one field of `app::State`
- [X] T029 [US2] Move `help`'s 2 fields (`about_open`, `help_menu_open`) into `features::help::State` in `crates/micold-client/src/features/help.rs`
- [X] T030 [US2] Move `window`'s 2 fields (`focused_field`, `window_size`) into `features::window::State` in `crates/micold-client/src/features/window.rs`
- [X] T031 [US2] Move `worktree_form`'s 2 fields (`worktree_error`, `worktree_form`) into the existing `WorktreeForm` grouping in `crates/micold-client/src/features/worktree_form.rs`
- [X] T032 [US2] Move `settings`' 3 fields (`settings_draft`, `system_scheme`, `theme_pref`) into `features::settings::State` in `crates/micold-client/src/features/settings.rs`
- [X] T033 [US2] Move `project`'s 5 fields (`forget_target`, `project_menu_open`, `project_switcher_open`, `rename_draft`, `selector`) into `features::project::State` in `crates/micold-client/src/features/project.rs`
- [X] T034 [US2] Move `worktree`'s 6 fields (`hovered_worktree`, `worktree_delete_keep_branch`, `worktree_delete_target`, `worktree_menu_open`, `worktree_rename_draft`, `worktrees`) into `features::worktree::State` in `crates/micold-client/src/features/worktree.rs`
- [X] T035 [US2] Move `sidebar`'s 10 fields (`default_expanded`, `expanded`, `pending_reveal_scroll`, `show_agent_worktrees`, `sidebar_filter_open`, `sidebar_filters`, `sidebar_hidden`, `sidebar_scroll_offset`, `sidebar_viewport_height`, `sidebar_width`) into `features::sidebar::State` in `crates/micold-client/src/features/sidebar.rs`
- [X] T036 [US2] Move `session`'s 12 fields (`active_session`, `last_foreground_choice`, `pending_tab_reveal`, `restarted_while_inactive`, `reveal_suppressed_for`, `session_menu_open`, `session_remove_target`, `shell_instance_menu`, `tab_strip_scroll_offset`, `tab_strip_viewport_width`, `terminal_context_menu`, `terminal_released`) into `features::session::State` in `crates/micold-client/src/features/session.rs`
- [X] T037 [US2] Settle `connection`'s empty case — it owns none of the 43 attributed fields and is the one feature absent from `OWNERS` ([data-model.md](./data-model.md) §3), so it gets **no** state struct rather than an empty one, on the same no-ceremony reasoning [research.md](./research.md) §R3 applies to its vocabulary. Note it in `crates/micold-client/src/features/connection.rs`'s header, and correct the two design docs that say ten: "10 feature structs" in `specs/028-feature-encapsulation/plan.md` and §1.1/§1.2 of `specs/028-feature-encapsulation/data-model.md`, to **9 feature structs + `workspace`**
- [X] T038 [US2] Declare `workspace` as the shared member (FR-008, contract S2) — keep it a flat field of `app::State` in `crates/micold-client/src/app.rs` with a doc comment naming the three features that read its six members and why it cannot be assigned to one ([data-model.md](./data-model.md) §3.2)
- [X] T039 [US2] Preserve `State::set_worktrees`' cross-struct reconciliation in `crates/micold-client/src/app.rs` — its writes now cross three feature structs; keep them field-by-field so the menu and hover state that survives a re-discovery today still survives it (S3, [research.md](./research.md) §R5)
- [X] T040 [US2] Adjudicate the renamed assertions — record every assertion whose text changed because a path was renamed, with the rename that caused it, in `specs/028-feature-encapsulation/assertion-adjudications.md` (FR-021, contract B1)
- [X] T041 [US2] Document what each feature remembers — state it in each feature module's header under `crates/micold-client/src/features/`, so SC-007 is met by reading one file (Principle VII)

**Checkpoint**: A maintainer can name everything a feature remembers by reading that feature's
module alone. Stories 1 and 2 both work; the ownership map is no longer the thing to consult.

---

## Phase 6: User Story 3 - The pattern cannot be opted out of, part 2: the state guards (Priority: P1)

**Goal**: G2 makes Story 2's shape non-optional, and FR-007a's component rule ships as a guard that
moves nothing today and catches the first field that genuinely qualifies.

**Independent Test**: Add a root state field with a single writing feature and confirm the suite
fails and names that feature (Acceptance Scenario 2).

### Guards for User Story 3 (Red step — FR-017, SC-005) ⚠️

- [X] T042 [US3] Write G2 in `crates/micold-client/tests/root_state_is_shared.rs` per [contracts/guards.md](./contracts/guards.md) — every public field of `app::State` is either a feature struct (its type resolves to `crate::features::<n>::State`) or a declared shared member in `SHARED: &[(&str, &str)]`; a flat field that is neither fails, with its single writer resolved through the same transitive `&mut State` scan `feature_write_isolation.rs` performs, and named
- [X] T043 [US3] Non-vacuity probe for G2 — add `pub scratch_pad: String` to `app::State` in `crates/micold-client/src/app.rs`, written only from `crates/micold-client/src/features/help.rs`, run `cargo test -p micold-client --test root_state_is_shared`, observe the failure naming `help`, revert, and record the message
  - Observed at T043: `` `state.scratch_pad: String` — written only by `help` — move it into `features/help.rs`'s `State` ``, from `every_root_field_is_a_feature_struct_or_a_declared_shared_member`. Three of the four tests passed, so the injection compiled and ran. Recorded in `specs/028-feature-encapsulation/assertion-adjudications.md` under `## T043`.

### Implementation for User Story 3, part 2

- [X] T044 [US3] Implement FR-007a's component rule in `crates/micold-client/tests/root_state_is_shared.rs` — a path with exactly one writing feature and no reader outside that feature's module and its view must move into the component that renders it, unless an existing assertion pins it to the application
- [X] T045 [US3] Populate FR-007a's allowlist with the five QUALIFIES fields — `about_open`, `default_expanded`, `expanded`, `sidebar_filter_open`, `sidebar_filters` — each entry naming `crates/micold-client/tests/logical_state_ownership.rs` as the assertion that pins it, per FR-016 and FR-021
  - Measured at T045: the rule reaches **twelve** paths, not five. The planned five are all in it (`sidebar_filter_open`/`sidebar_filters` now spelled `sidebar.filter_open`/`sidebar.filters`), plus seven the plan did not name: `help.help_menu_open`, `project.menu_open`, `project.switcher_open`, `session.menu_open`, `session.shell_instance_menu`, `sidebar.hidden`, `window.window_size`. All seven are *which surface is open*, which the plan's manual pass counted as root-owned rather than component-local. Every one is pinned by an existing assertion, so nothing moves; eight of the twelve are pinned outside `logical_state_ownership.rs` (`about_open.rs`, `switcher_forget_menu.rs`, `project_switcher.rs`, `app_state.rs`, `sidebar_state.rs`, `features_window.rs`, `switch_active.rs`), which FR-016 allows — it requires the entry name the assertion, not which file holds it.
- [X] T046 [US3] Confirm `crates/micold-client/tests/logical_state_ownership.rs` is unmodified — it is the feature-017 guard that bounds FR-007a, and FR-021 forbids relaxing it to let a path move
  - Confirmed at T046: `git diff main -- crates/micold-client/tests/logical_state_ownership.rs` is 41 insertions / 27 deletions and **every one is a rename** — `Message::X` -> `Message::Feature(Msg::X)` and `state.x` -> `state.feature.x`, plus the five `use` lines those need. No assertion was deleted, weakened, or given a looser expected value; all eleven `#[test]` functions and every `assert!`/`assert_eq!`/`assert_ne!` in them survive with the same meaning. The spelling changes are adjudicated under contract B1 at T040.
- [X] T047 [US3] Shrink `OWNERS` in `crates/micold-client/tests/feature_write_isolation.rs` to the shared members — ownership is now a property of the field's type rather than of a 51-entry hand-maintained `const` (SC-007)
  - Done at T047: 51 rows -> 15 (feature 028's field moves) -> **6**, all of them `workspace.*`. The nine root fields whose type is `crate::features::<n>::State` are resolved by the new `declared_owners()`, which reads the declaration; `OWNERS` now holds only the split of the one field whose type cannot name an owner. `every_state_field_has_an_owner` fails on a non-`workspace` entry rather than a merely stale one, so the table cannot grow back.
- [X] T048 [US3] Verify SC-003 and SC-007 — the root state contains no loose path with exactly one writing feature ([quickstart.md](./quickstart.md) §A.2), and every feature's state is nameable from its own module
  - Verified at T048: §A.2 reports **10** root fields — nine `crate::features::<n>::State` plus `workspace` — down from 44. G2's `every_root_field_is_a_feature_struct_or_a_declared_shared_member` is the criterion and passes; `component_local_paths_are_pinned_or_moved` accounts for all twelve paths FR-007a reaches. SC-007: nine modules declare their own `pub struct State`, `features/connection.rs` documents why it declares none, and `OWNERS` no longer restates any of it (T047).
- [X] T049 [US3] Verify SC-005 in full — all three probes observed, each recorded with the failure message it produced, in `specs/028-feature-encapsulation/assertion-adjudications.md`
  - Verified at T049: three probe records, one per guard — `## T022` (G1, naming `help`), `## T024` (G3, naming `probe`), `## T043` (G2, naming `help`). Each records the exact panic, how many of the file's other tests passed alongside it (the check that the injection compiled), and the revert. A fourth record, `## The check was verified to block before anything was renamed (T005)`, covers the freeze itself.
- [X] T050 [US3] Document G2 and the FR-007a rule in `crates/micold-client/tests/root_state_is_shared.rs`'s header — the rule, the allowlist's meaning, and the probe (Principle VII)
  - Done at T050: the header carries five sections — the rule (feature struct or declared shared member), why it is stated over *types* rather than over a hand-maintained list of claims, the component rule and why all twelve of its hits are pinned, why neither allowlist may outlive its reason, and T043's probe.

**Checkpoint**: All three stories are independently functional. Every guard has been seen failing.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Make the guards hold everywhere, and prove the one thing source-text scanning cannot.

- [X] T051 Verify SC-001 by measurement (not by a fourth guard, per the clarification of 2026-08-25) — add one interaction to a converted feature, count the files changed, and confirm it is that feature's module and its view; record the count in `specs/028-feature-encapsulation/quickstart.md` §A
  - Measured at T051: **2 files** — `src/features/sidebar.rs` and `src/ui/sidebar.rs` — for a "Collapse all" interaction (new `Msg::AllCollapsed`, `pub fn all_collapsed`, its `update` arm, and the button that emits it). All four guards stayed green across the injection. Recorded in [quickstart.md](./quickstart.md) §A.5; reverted.
- [X] T052 Add all seven guards to the "component library + showcase gates, all platforms" step in `.github/workflows/ci.yml` — six `--test` entries carry them, because `feature_registration_cost` holds both a pre-existing rule and this feature's G3: `root_vocabulary_is_cross_cutting`, `root_state_is_shared`, `feature_registration_cost`, `feature_write_isolation`, `root_is_routing_only`, `logical_state_ownership`. Closes the omission 021's T058 and T077 both recorded (FR-018)
  - Done at T052: the six `--test` entries are appended to the step's list in `.github/workflows/ci.yml`, with a comment on the step saying why they are there — each guard reports its findings *by path*, which is exactly the difference a Linux-only run cannot see. The whole 17-binary list runs green locally.
- [X] T053 Confirm all three CI matrix jobs green before the last conversion merges — the seven guards report findings by path, which is exactly where a `\` vs `/` difference goes unnoticed on a Linux-only run
  - Confirmed at T053 on PR #228 at `8480c089`: **ubuntu-latest, macos-latest and windows-latest all green**
    (3m30s / 2m20s / 2m58s), and with them `fmt + clippy`, `assertion freeze`, `docs check`, the real-runtime
    sandbox job and `ci complete`. The six guard binaries T052 added to the all-platforms step therefore ran on
    Windows too, which is the point of the task: a guard that reports by path is exactly what a Linux-only run
    cannot vet.
- [X] T054 [P] Confirm SC-008 — run `crates/micold-client/tests/idle_requests_no_frames.rs` via `cargo test -p micold-client --test idle_requests_no_frames` and confirm no additional frames while idle (FR-011)
  - Confirmed at T054: 7 passed, 0 failed. SC-008 holds — this feature moved fields and variants and touched no render path, and `only_the_motion_primitive_asks_for_frames` is the assertion that would have caught it if it had.
- [X] T055 [P] Confirm SC-006 — `mise run test` green, and `scripts/check-assertions-frozen.sh` passing with every spelling change adjudicated
  - Observed: `mise run test` — 212 `test result: ok` blocks, EXIT=0. `scripts/check-assertions-frozen.sh` — `OK — 4311 assertion(s) intact, 354 added, 328 removal(s) adjudicated`, RC=0. Every removal is a rename or a reworded message recorded in [assertion-adjudications.md](./assertion-adjudications.md); the last one, T047's `OWNERS` staleness message, is adjudicated there as a strengthening — same predicate, strictly larger set.
- [X] T056 Run the six behaviour-preservation scenarios in [quickstart.md](./quickstart.md) §C.4 via the repository's `visual-pass` skill — the draft that survives a refusal, the expansion that survives a re-discovery, the settings draft that survives a cancel, the terminal selection that survives a tab switch, the project switch that resets what it resets today, and the filter that survives a dismissal. Any difference is a bug in this feature, not a decision to make now (FR-019, FR-020)
  - Run at T056 on 2026-08-27, recorded in [visual-pass.md](./visual-pass.md) with six comparison images.
    Headless — Xvfb `:81` + lavapipe, both binaries from one build of this branch and pinned to a private
    directory. **All six matched `main`**, so FR-020 has nothing to pin. Two of the six are worth reading
    rather than counting: the terminal selection is *cleared* by a switch on both branches — deliberately,
    in `view_and_start`, since 021 — so §C.4's "survives" was the inaccurate half and is now corrected
    there; and the expansion that a project switch drops is `worktrees_replaced` pruning against the
    incoming project's names, not `project_entered`, which touches exactly two fields and no others.
    What the pass cannot answer stands as always: mid-flight animation and perceived smoothness.
- [X] T057 Confirm no allowlist entry outlives its reason — run the reverse "the allowlist names only live violations" check in `crates/micold-client/tests/root_vocabulary_is_cross_cutting.rs` and `crates/micold-client/tests/root_state_is_shared.rs` and confirm every entry across G1, G2 and FR-007a still describes a live exception (spec Edge Cases)
  - Confirmed at T057: `the_allowlist_names_only_live_violations` passes in all three files — `root_vocabulary_is_cross_cutting.rs` (G1's cross-cutting exceptions), `root_state_is_shared.rs` (G2's `SHARED` **and** FR-007a's `COMPONENT_LOCAL`, the reverse check added at T045), and `feature_write_isolation.rs` (`ALLOWED`, still empty). Four allowlists, no entry outliving its reason.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — **blocks every conversion**, because the freeze must be live before the first assertion spelling changes
- **US1 (Phase 3)**: depends on Phase 2
- **US3 part 1 (Phase 4)**: depends on Phase 3 — G1 cannot pass until the vocabularies are nested, and a guard relaxed to let its own migration through is not holding anything
- **US2 (Phase 5)**: depends on Phase 3 (spec.md: a feature cannot own its state until it owns the messages that write it; [research.md](./research.md) §R4 confirms it — a field cannot leave the root while the root arm that writes it still names it). Independent of Phase 4.
- **US3 part 2 (Phase 6)**: depends on Phase 5, for the same reason Phase 4 depends on Phase 3
- **Polish (Phase 7)**: depends on Phases 4 and 6 — T052 can only list guards that exist

### Within Each Story

- Conversions are strictly sequential: every task in Phases 3 and 5 edits `crates/micold-client/src/app.rs`, so none of them is `[P]`
- Order within Phase 3 is smallest-first ([research.md](./research.md) §R9) so the pattern is proven cheap before the 37-variant feature; Phase 5 follows the same order for the same reason
- Each conversion is one commit, buildable, runnable and green (FR-006, contract M5)
- A guard's probe (Red) precedes relying on it; probes are never `[P]` with each other, because each mutates the tree and reverts it

### Parallel Opportunities

- T002 and T003 (Setup)
- T019, T020, T021 — G1's rule, its third verdict and its allowlist are separable pieces of one new file and can be drafted in parallel, then landed together
- T054 and T055 (Polish)
- **Across stories**: once Phase 3 completes, Phase 4 (the vocabulary guards) and Phase 5 (the state structs) are independent and can proceed in parallel by different people. Nothing else in this feature parallelises — the two files everything touches are `app.rs` and `main.rs`.

---

## Implementation Strategy

### MVP (User Story 1 + the guards that hold it)

1. Phase 1 → Phase 2 (the freeze goes live)
2. Phase 3 — nine conversions, root `Message` 119 → 15
3. Phase 4 — G1 and G3, each observed failing
4. **STOP and VALIDATE**: SC-002, SC-004, SC-005 (2 of 3), SC-009. The root enum is permanently
   smaller and the pattern is no longer optional. Shippable.

### Incremental Delivery

| Stop after | Delivered | Criteria met |
|---|---|---|
| Phase 3 | Root vocabulary 119 → 15 | SC-001, SC-004, SC-009 |
| Phase 4 | Story 1's shape is enforced | + SC-002, SC-005 (2 of 3) |
| Phase 5 | Every feature names its own state | + SC-003, SC-007 |
| Phase 6 | Story 2's shape is enforced | + SC-005 (3 of 3) |
| Phase 7 | The guards hold on every platform | + SC-006, SC-008, FR-018 |

Each stop leaves the application green and shippable. That is FR-006, and it is what makes the
plan's nine-phase ordering a sequence of deliveries rather than a single long-lived branch.

---

## Notes

- The two files every conversion touches are `crates/micold-client/src/app.rs` and `crates/micold-client/src/main.rs`; roughly 100 test files read the state paths that move in Phase 5, so that phase is wide and shallow
- Use `mise run test-core` while iterating (render-free, fast) and `mise run test` before each commit (whole workspace, matches CI)
- Commit after each task in Phases 3 and 5; the conversion tasks are deliberately one-commit-sized
- Record every observed probe failure beside its task — a probe that did not compile demonstrated nothing, and 021's record has two of exactly that
- `showcase/`, `micold-core`, `micold-daemon` and the wire protocol are out of scope
