---

description: "Task list for feature 029 — worktree tooltip shows the full name and its details"
---

# Tasks: Worktree tooltip shows the full name and its details

**Input**: Design documents from `/specs/029-worktree-tooltip-details/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/worktree-tooltip.md](./contracts/worktree-tooltip.md),
[quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), every implementation task
below is preceded by a task that writes a test and **observes it fail**. The two exceptions are
named where they occur (T003, T004, T024) and both fall inside the constitution's GUI/render-glue
exception: they hold no decision logic and are covered by `quickstart.md` §B instead.

**Documentation**: Per Principle VII, `docs/user-guide/worktrees-and-sessions.md:147-149` currently
tells the user the tooltip shows the location and nothing else. Each story updates it as part of
that story, not afterwards.

**Cross-platform**: Per Principle VI, nothing here branches on the host OS. The one platform-shaped
detail is `Path::strip_prefix`/`Display`, which the existing location tests already pin.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: The user story the task serves (US1, US2, US3)
- Every task names the exact file it touches

## Path Conventions

Rust workspace, three crates. Paths below are repository-relative:
`crates/micold-core/`, `crates/micold-client/`, `docs/`.

---

## Phase 1: Setup

**Purpose**: Know the ground is green before moving it, and know exactly what moves.

- [X] T001 Run `mise run test` and confirm the workspace is green before any edit, so a later red is this feature's and not something inherited
- [X] T002 Record the current call sites that will move, in a scratch note or the commit body: `worktree_location_label` (`crates/micold-client/src/features/sidebar.rs:397`), its attachment (`crates/micold-client/src/ui/sidebar.rs:562-565`), and its existing assertions (`crates/micold-client/tests/features_sidebar.rs:81,96` and `crates/micold-client/tests/sidebar_tree.rs:213-225`)

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: Build the seam every story hangs off — a tooltip panel that can hold more than one
line, and a builder that owns the whole block. No story can be delivered before this phase, and no
story's fact is added in it.

**⚠️ CRITICAL**: Complete this phase before Phase 3.

- [X] T003 [P] Add a chainable `wrapping(...)` builder method to `Text` in `crates/micold-client/src/ui/material/text.rs`, applied in its `From<Text> for Element` impl — no test task: it is a pass-through to the rendering stack with no branch of its own (Principle I GUI exception), covered by `quickstart.md` §B3
- [X] T004 Bound the shared tooltip panel in `crates/micold-client/src/ui/material/mod.rs`: a documented component-owned `MAX_WIDTH` const (320dp, Material 3's rich-tooltip ceiling — see research R2 for why this is *not* an `anatomy::` token) applied to the panel container, plus `.wrapping(Wrapping::WordOrGlyph)` on its `Text` so a path with no spaces still breaks. No test task, same exception as T003; `quickstart.md` §B3 is the gate
- [X] T005 Write failing tests in `crates/micold-client/tests/features_sidebar.rs` for `worktree_tooltip`'s location half: the value matches contract §3.1 (relative under the project root, absolute outside it), it renders as a `Location: ` labelled line (§2), and the whole line is **absent** — with every other line intact — when `project_root` is `None` (§3.3). Observe them fail
- [X] T006 Replace `worktree_location_label` with `worktree_tooltip(project_root: Option<&Path>, worktree: &Worktree, display_name: &str) -> String` in `crates/micold-client/src/features/sidebar.rs`, emitting the `Location:` line only, to make T005 pass. Keep `DEFAULT_LOCATION_LABEL` and its doc comment exactly as they are (§1.3)
- [X] T007 Re-point the existing location assertions in `crates/micold-client/tests/sidebar_tree.rs:213-225` and `crates/micold-client/tests/features_sidebar.rs:81,96` at the new function without rewriting what they claim — feature 010's FR-010 behaviour is preserved, not redesigned (§3.1)
- [X] T008 Attach the tooltip unconditionally in `crates/micold-client/src/ui/sidebar.rs`: drop the `if let Some(root) = project_root` wrapper around `row_tooltip` and pass `project_root` into the builder instead (research R5, §5.1). Glue only — one call, no branch

**Checkpoint**: `mise run test` green. Hovering a worktree row shows `Location: …` — one labelled
line where there used to be a bare path. No story is delivered yet.

---

## Phase 3: User Story 1 — Read a name the row had to shorten (Priority: P1) 🎯 MVP

**Goal**: The tooltip leads with the worktree's complete name, so a row that ends in an ellipsis
stops being ambiguous.

**Independent test**: Give a project a worktree whose name is far longer than the sidebar is wide,
hover its row, and read the whole name in the tooltip while the row itself still ellipsizes
(`quickstart.md` §B1).

- [X] T009 [US1] Write failing tests in `crates/micold-client/tests/features_sidebar.rs`: `Name: <display_name>` is the **first** line (§2.3), carries the name verbatim including one long enough that a row would shorten it (FR-001), and is present whether or not a `Location:` line follows. Observe them fail
- [X] T010 [US1] Write a failing test in `crates/micold-client/tests/features_sidebar.rs` that the name in the tooltip is the caller's `display_name` — a user's rename, not a re-derivation from `dir_name` (FR-002, §2 row 1). Observe it fail
- [X] T011 [US1] Add the `Name:` line as the first line of `worktree_tooltip` in `crates/micold-client/src/features/sidebar.rs` to make T009 and T010 pass
- [X] T012 [US1] Pass `State::worktree_display_name(&wt.dir_name)`'s value — the same string the row renders — into the builder at `crates/micold-client/src/ui/sidebar.rs`, so the row and its tooltip cannot disagree (FR-002)
- [X] T013 [US1] Update `docs/user-guide/worktrees-and-sessions.md:147-149`: the hover tooltip is no longer location-only, and leads with the worktree's full name — the sentence that currently says otherwise is the one to replace

**Checkpoint**: US1 is independently shippable. `mise run test` green; `quickstart.md` §B1 passes.

---

## Phase 4: User Story 2 — See what the row's shortened label leaves out (Priority: P2)

**Goal**: The tooltip names the git branch the row's prettified label strips out, and the folder on
disk when it differs from the displayed name.

**Independent test**: Hover a worktree whose folder name carries a type token and a ticket, and
read both the bound branch and the on-disk folder that the row never shows (`quickstart.md` §B2).

- [X] T014 [P] [US2] Write failing tests in `crates/micold-client/tests/features_sidebar.rs` for the `Branch:` line: present and verbatim when `worktree.branch` is `Some`, and **absent** — not blank, not a placeholder — when it is `None` (FR-004, §2.2). Observe them fail
- [X] T015 [P] [US2] Write failing tests in `crates/micold-client/tests/features_sidebar.rs` for the `Folder:` line: present when `dir_name != display_name`, absent when they are equal, so the tooltip never prints the same string twice (FR-005). Observe them fail
- [X] T016 [US2] Write a failing test in `crates/micold-client/tests/features_sidebar.rs` pinning the full line order `Name` → `Branch` → `Folder` → `Location` on a worktree that has all four (§2). Observe it fail
- [X] T017 [US2] Add the `Branch:` and `Folder:` lines, in contract order, to `worktree_tooltip` in `crates/micold-client/src/features/sidebar.rs` to make T014–T016 pass
- [X] T018 [US2] Extend the tooltip paragraph in `docs/user-guide/worktrees-and-sessions.md` with the branch and folder the tooltip now names, and why they are not on the row

**Checkpoint**: US1 and US2 both work. `mise run test` green; `quickstart.md` §B2 passes.

---

## Phase 5: User Story 3 — Understand a row that is flagged (Priority: P3)

**Goal**: What a row says with colour and a chip — missing, invalid, outside this app — the tooltip
says in words, next to the location that explains it.

**Independent test**: Hover a missing worktree and an included one; each tooltip states its
condition, matching the row's chip word for word (`quickstart.md` §B4).

- [X] T019 [US3] Write failing tests in `crates/micold-core/tests/worktree_model.rs` for `WorktreeStatus::label()`: `Some("missing")`, `Some("invalid")`, and `None` for `Valid` — the `None` being the assertion that matters (§4.1, research R3). Observe them fail
- [X] T020 [US3] Add `WorktreeStatus::label(&self) -> Option<&'static str>` to `crates/micold-core/src/worktree.rs` to make T019 pass
- [X] T021 [P] [US3] Write failing tests in `crates/micold-client/tests/features_sidebar.rs`: a `Status:` line carrying `label()`'s word for `Missing` and `Invalid`, and **no** `Status:` line at all for `Valid` (FR-006, §4.3). Observe them fail
- [X] T022 [P] [US3] Write a failing test in `crates/micold-client/tests/features_sidebar.rs`: an `included` worktree's `Location:` value ends `" (outside this app)"` after its absolute path, and a non-included one does not (FR-007, §3.2). Observe it fail
- [X] T023 [US3] Add the `Status:` line and the `included` suffix to `worktree_tooltip` in `crates/micold-client/src/features/sidebar.rs` to make T021 and T022 pass
- [X] T024 [US3] Point `tag_chip`'s status arm at `WorktreeStatus::label()` in `crates/micold-client/src/ui/sidebar.rs:332-339`, deleting the local `match` and its `""` arm so chip and tooltip read one source (§4.2). No test task: the chip's rendering is glue, and T019 now holds the word it shows
- [X] T025 [US3] Document in `docs/user-guide/worktrees-and-sessions.md` that a flagged row explains itself on hover — the status word and the outside-this-app note, alongside the chips that already carry them

**Checkpoint**: All three stories work. `mise run test` green; `quickstart.md` §B4 passes.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T026 [P] Add a third posed instance to the `Tooltip` section of `crates/micold-client/src/showcase/sections/floating.rs` — a multi-line label long enough to reach the ceiling and wrap — so the gallery shows the shape the component grew
- [X] T027 Declare that instance's caption in the `Tooltip` entry's `posed` array in `crates/micold-client/src/showcase/catalogue.rs`; `tests/showcase_completeness.rs` and `tests/showcase_captions.rs` are what fail if T026 and T027 disagree
- [X] T028 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`; CI's first job is `cargo fmt --check` and it gates every other job, so an unformatted tree fails before a single test runs
- [X] T029 Run `mise run test` for the full workspace, matching CI, and confirm every gate named in `quickstart.md` §A is green
- [X] T030 Run the `quickstart.md` §B pass (B1–B6) with the repo's `visual-pass` skill on a private display, and append the result — date, display, and a screenshot for B1, B3, B4 and B5 — to `quickstart.md`. A step that could not be run is recorded as *not run* with its reason, never as a pass

---

## Dependencies & Execution Order

### Phase order

1. **Setup (T001–T002)** — no dependencies
2. **Foundational (T003–T008)** — blocks every story; T005 before T006 (Red before Green), T006 before T007 and T008
3. **US1 (T009–T013)** — depends on Foundational only
4. **US2 (T014–T018)** — depends on Foundational; independent of US1
5. **US3 (T019–T025)** — depends on Foundational; independent of US1 and US2
6. **Polish (T026–T030)** — T026/T027 depend on Foundational (T004); T029/T030 depend on everything

### Story independence

All three stories add lines to one function, in contract order, and each omits its own line when its
fact is absent. US2 shipped without US1 would still produce a valid tooltip — it would simply lack
the `Name:` line — so the stories are independent in the sense that matters: any one of them can be
implemented, tested, and demonstrated on its own.

The one shared file is `crates/micold-client/src/features/sidebar.rs`, which is why the
implementation tasks within a story (T011, T017, T023) are **not** `[P]` with each other. Their
tests are, because they are additive assertions.

### Parallel opportunities

- **Setup**: T001 then T002 — sequential, and both are minutes.
- **Foundational**: T003 and T004 are `[P]` with T005 (different crates' files: `ui/material/` vs `tests/`).
- **US2**: T014 and T015 in parallel — different assertions, same file, additive.
- **US3**: T021 and T022 in parallel once T020 lands.
- **Across stories**: after Foundational, all three story phases can proceed in parallel *if* their implementation tasks are serialized on `features/sidebar.rs`.

---

## Implementation Strategy

### MVP (User Story 1 only)

Phase 1 → Phase 2 → Phase 3 → `quickstart.md` §B1. That is the reported problem fixed: a name the
row shortened can be read in full. Nine tasks, and it is shippable on its own.

### Incremental delivery

Add US2 for the facts the prettified label strips out, then US3 for the flagged rows. Each phase
ends green, with its own §B step and its own paragraph in the user guide — so stopping after any
checkpoint leaves the tree consistent rather than half-migrated.

### Red-Green-Refactor, concretely

The Red in this feature is cheap and worth insisting on: every rule in the contract is a claim about
which lines exist. A test written after the fact would assert the string the implementation happens
to produce, which is exactly the failure mode Principle I names — codifying behaviour instead of
specifying it. Write the assertion about the *absent* line first (T005, T014, T015, T021): those are
the ones an implementation gets wrong by emitting a blank line, and the ones a post-hoc test would
never think to make.
