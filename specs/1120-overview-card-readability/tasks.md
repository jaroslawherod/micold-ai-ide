---

description: "Tasks: Overview cards, readable layout and content"
---

# Tasks: Overview cards, readable layout and content

**Input**: Design documents from `/specs/1120-overview-card-readability/`

**Prerequisites**: plan.md, spec.md, research.md (R1–R15), data-model.md, contracts/card-text.md
(C1–C10), contracts/overview-card.md (K1–K4), quickstart.md

**Tests**: Constitution Principle I: every test task is written first and run, and must fail for the
stated reason (missing item, or the old rule's output) before the implementation task it covers.
Every row of a contract table is one assertion. Core tests run with `mise run test-core`; client
tests with `scripts/build-lock.sh cargo test -p micold-client --lib -- <filter>`.

**Documentation**: Principle VII: each story's user-guide task ships in that story's milestone
(`docs/user-guide/overview.md`), so CI's user-guide gate passes on every `feat` PR.

**Cross-platform**: Principle VI: no platform code. The card draws in the bundled Roboto in logical
pixels (R14), so the Linux-measured geometry tests hold on macOS and Windows CI.

**Organization**: grouped by user story, in **implementation order**, which differs from the spec's
order on purpose: US1's header band and 14 px body lines make a card with every row taller than the
fixed 172 px, so US1 needs US4's grow-to-fit first (see *Dependencies*). US2 is the MVP.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, dependency or fixture. Two new core modules and one cdk widget are created by
the tasks that first need them.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the *Card text* entity every later phase fills in.

- [X] T001 Create `crates/micold-core/src/overview_text.rs` (module doc: pure, no I/O, no logging, FR-020) and add `pub mod overview_text;` to `crates/micold-core/src/lib.rs`, with the data-model types only: `pub struct Line { pub text: String, pub tooltip: String }`, `pub struct ProgressRow { pub text: String, pub bar: Option<Ratio> }` and `pub struct CardText { title: String, title_tooltip: String, branch: Option<Line>, sessions: String, progress: Option<ProgressRow>, prs: Option<(String, Option<String>)>, activity: Option<Line> }` (all `pub`, `Debug, Clone, PartialEq, Eq`). No functions yet; `mise run test-core` stays green

**Checkpoint**: the types compile; nothing uses them.

---

## Phase 3: User Story 2 - The card says what the worktree is for (Priority: P1) 🎯 MVP

**Goal**: the title is the issue title, else the branch, else the display name; the branch reads
`type/number`; one sessions line with plurals; no empty progress or pull request row. Rules in core
(FR-006–FR-010, FR-018), the card renders them in its current (whole-card fill) layout.

**Independent Test**: `mise run test-core` (C4–C9) and
`scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card`.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T002 [US2] Unit tests in `crates/micold-core/src/overview_text.rs` for `card_title(issue_title: Option<&str>, branch: Option<&str>, display_name: &str) -> String` (every C4 row, a blank issue title counts as unknown), `short_branch(branch: &str) -> Option<String>` (every C5 row: `fix/1115_container-daemon-fails-to-start`, `fix/1115-foo`, `feat-813_worktree-overview`, `chore/42` shorten; `main`, `claude/project-thread-7041o6`, `feat/make-branch-selector`, `scratch/023-docs-gate`, `fix/1115_`, `fix/12a` are `None`) and `branch_line(title: &str, branch: Option<&str>) -> Option<Line>` (every C6 rule: `None` without a branch, `None` when the title equals the branch, else short-or-unchanged text with the full branch as tooltip). Red: the functions do not exist (FR-006, FR-007, US2 scenarios 1–4 and 8)
- [X] T003 [US2] Unit tests in `crates/micold-core/src/overview_text.rs` for `sessions_line((total, running): (u32, u32)) -> String` (every C7 row, including `(120, 101)` and the clamped `(2, 5)`), `progress_row(&Progress) -> Option<ProgressRow>` (every C8 row: neither phase nor ratio is `None`; a bar only with a ratio), `activity_line(Option<&CurrentActivity>) -> Option<Line>` without sanitising yet (`line · done/total` with a ratio, the line alone without; tooltip the same text; `None` for no activity or a line blank after trim) and `CardText::of(&CardModel)` (each field from its rule; `title` read from `model.summary` until T009 renames that field to `title`; `title_tooltip` = display name, `prs` = `model.prs.footer()`). Red: the functions do not exist (FR-008, FR-009, US2 scenarios 5–6)
- [X] T004 [P] [US2] In `crates/micold-core/src/overview.rs` tests, replace the `PrDisplay::footer` expectations with every C9 row: `Counts{0,0}` → `None`; `Counts{1,0}` → `"1 open"`; `Counts{0,2}` → `"2 merged"`; `Counts{1,2}` → `"1 open · 2 merged"`; capped 100 open, 0 merged → `"100+ open"`; `Hidden` → `None`; `Unknown(r)` → the dash with `r.label()`. Red: today's rule prints `0 open · 0 merged` (FR-010, US2 scenario 7)
- [X] T005 [US2] In the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/overview_card.rs` (reuse `model`, `with_progress`, `card` and `test_support::leaves`): a card with neither phase nor ratio and `PrDisplay::Counts{0,0}` has no text leaf reading `No task progress` or containing `0 open`; a card with title `Overview cards: readable layout`, display name `feat-1120_x` and branch `feat/1120_x` shows the title, the branch line `feat/1120`, and the display name as the title's tooltip and the full branch as the branch line's tooltip; a card whose title is its branch has one leaf with the branch text; the sessions part reads `1 session · 1 running` for `sessions = (1, 1)`. Update `the_progress_row_has_a_bar_only_with_a_ratio` so `(None, None)` expects no row. Red: the card still prints the display name, the full branch, `1 sessions, 1 running` and `No task progress` (FR-006–FR-010, SC-003)

### Implementation for User Story 2

- [X] T006 [US2] Implement `card_title`, `short_branch` (R3: hand-written byte scan, `<type><sep><digits>` with `<sep>` `/` or `-`, `<type>` accepted by `naming::ConventionalType::from_token`, then the end or `_`/`-` and a non-empty slug; no regex) and `branch_line` in `crates/micold-core/src/overview_text.rs`; T002 passes
- [X] T007 [US2] Implement `sessions_line` (R4, `running` clamped to `total` for display, no cap), `progress_row` (R5, moved from the client's `progress_words`: `Implement · 6/10`, `6/10`, the phase alone, `None` for neither), `activity_line` (today's `activity_words` text, moved) and `CardText::of` (title from `model.summary`, which T009 renames) in `crates/micold-core/src/overview_text.rs`; T003 passes
- [X] T008 [P] [US2] Change `PrDisplay::footer` in `crates/micold-core/src/overview.rs` to R6's rule, signature unchanged; T004 passes
- [X] T009 [US2] Rename `CardModel.summary` to `title` in `crates/micold-core/src/overview.rs` (doc: "issue title, else branch, else display name", FR-006) and its test helper (around line 1065); in `crates/micold-client/src/ui/overview.rs::card` set it with `overview_text::card_title(issue_title, branch, display_name)`, passing `Some(text)` only for `TitleLookup::Known`; remove `features::overview::summary` and its tests from `crates/micold-client/src/features/overview.rs` (the rule is now C4, core-tested); set `title` in `crates/micold-client/src/showcase/sections/surfaces.rs`'s `card_model` (an issue-like sentence, branch `feat/813_worktree-overview`). update `CardText::of` and the T003 tests to read `model.title`. Find every use with LSP `findReferences` on `summary` first
- [X] T010 [US2] Render `CardText::of(&model)` in `crates/micold-client/src/ui/material/overview_card.rs`, layout otherwise unchanged: the title line shows `title` (tooltip `title_tooltip`); the branch line shows `branch` when `Some` (tooltip = full branch); the separate summary line is removed; the footer reads `<type word> · <sessions line>` (the type word moves to the header in US1); the activity line shows `activity` when `Some` (tooltip = its text); the progress row and its bar only when `progress` is `Some`; the pull request row from `prs`. Remove `progress_words`, `ProgressWords`, `activity_words` and `Words.name_tooltip`/`Words.footer`; T005 passes and every existing `overview_card` test stays green
- [X] T011 [P] [US2] In `docs/user-guide/overview.md`: rewrite *One card per location* for the title rule (issue title, else branch, else folder name, which shows on hover), the short branch (`fix/1115`, full name on hover) and the sessions line (`No sessions`, `1 session · 1 running`); fold *The summary line* into a *Title* paragraph and delete that section; drop *No task progress* from *Feature progress* (the row is hidden when there is nothing to show); in *Pull request counts* say the row is hidden at 0 · 0 and a single count shows alone (FR-019)

**Checkpoint**: the card says the right things; core rules covered by `mise run test-core`.

---

## Phase 4: User Story 3 - The activity line reads as a sentence, never as markup (Priority: P1)

**Goal**: markup stripped, slash commands as sentences, no line rather than markup, the hook keeps
the whole prompt sanitised (FR-011–FR-013, FR-020).

**Independent Test**: `mise run test-core` (C1, C2, C3, C10).

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T012 [P] [US3] Create `crates/micold-core/src/markup.rs` (`pub mod markup;` in `crates/micold-core/src/lib.rs`, module doc: pure, never logs, FR-020) with a `readable(text: &str) -> Option<String>` stub returning `None` and unit tests for every C1 row (single, nested/unbalanced, multi-line wrapper, multi-line text joined, `a < b` / `x<3` / `if a<b` unchanged, empty / whitespace / tags-only → `None`, `<<b>a>` → `None`, `a<b and c>d` → `Some("ad")`). Red: the stub fails the `Some` rows (FR-011, FR-013, spec edge cases on nested markup and `a < b`)
- [X] T013 [US3] Unit tests in `crates/micold-core/src/overview_text.rs` for `activity_text(text: &str) -> Option<String>` (every C2 row: `/speckit-autopilot #1117` → `Running autopilot on #1117`, `/clear` → `Running clear`, `/compact keep the plan`, markup inside arguments removed first, `/home/user/x` and `/` alone unchanged, `/speckit-` → `Running speckit`, `/etc` → `Running etc`, plain text unchanged, markup only → `None`) and for C3 with sanitising: an `<agent-message …>` line reads `Continue M2`, a markup-only line with a ratio is `None` (the ratio never shows alone), a todo item is sanitised the same way, a multi-line line becomes one line. Also, in the `#[cfg(test)]` module of `crates/micold-client/src/ui/overview.rs`, a guard test (green today, kept green by T017): a location whose main session's `hook_activity` has prompt `None` and no todos, beside a helper session in the same location with readable prompt text, builds a `CardModel` whose `activity` is `None` (spec edge case "Several sessions": no fall back to another session's prompt). Red: `activity_text` does not exist and `activity_line` shows markup (FR-011–FR-013, US3 scenarios 1–6)
- [X] T014 [P] [US3] Unit tests in `crates/micold-core/src/hook_activity.rs` for every C10 row: a multi-line `<agent-message from="x">` wrapper around `Continue M2` stores `Some("Continue M2")` and returns `true`; the same again returns `false`; a markup-only prompt after a readable one stores `None` and returns `true`; 300 readable characters store the first 200 (`MAX_TEXT_CHARS`). Red: today `apply` stores the first raw line (FR-011, FR-013, spec edge case "Several sessions")

### Implementation for User Story 3

- [X] T015 [US3] Implement `markup::readable` in `crates/micold-core/src/markup.rs` by R7 steps 1–3 (a tag is `<`, optional `/`, an ASCII letter, then everything up to and including the next `>` with no `<` before it; collapse whitespace runs to one space and trim; `None` when empty or a tag remains); T012 passes
- [X] T016 [US3] Implement `activity_text` in `crates/micold-core/src/overview_text.rs` (R7: `readable`, then a leading `/` + name `[A-Za-z0-9_:.-]+` ended by whitespace or the end → `Running <name>` or `Running <name> on <args>`, `speckit-` removed from the name's front, `-` read as a space; the same for every command, Clarifications) and make `activity_line` use it; T013 passes
- [X] T017 [US3] Change `HookActivity::apply` in `crates/micold-core/src/hook_activity.rs` (R8): `readable` on the whole prompt, then clip to `MAX_TEXT_CHARS`; an unreadable prompt sets `prompt` to `None`; return `true` only when the value changed; first list every reader of `HookActivity.prompt` with LSP `findReferences` (daemon `state.rs`, `crates/micold-daemon/tests/hooks_receiver.rs` and others) and update any that assume a first raw line; T014 passes and the existing `hook_activity` tests stay green
- [X] T018 [P] [US3] In `crates/micold-client/src/showcase/sections/surfaces.rs`, give one sample card the activity `<agent-message from="orchestrator">Continue M2</agent-message>` (it shows `Continue M2`, R15)
- [X] T019 [P] [US3] In `docs/user-guide/overview.md` rewrite *What the agent is doing now*: the line shows the latest prompt's words on one line with agent markup removed, a slash command reads as a sentence (`/speckit-autopilot #1117` → *Running autopilot on #1117*, `/clear` → *Running clear*), and the line is hidden when nothing readable is left (FR-019)
- [X] T020 [US3] Run quickstart §A's FR-020 grep (`! grep -nE 'tracing::|log::|println!|eprintln!|dbg!' crates/micold-core/src/{markup,overview_text,hook_activity}.rs`) and confirm it finds nothing

**Checkpoint**: no markup on any card; `mise run test-core` green.

---

## Phase 5: User Story 4 - Cards grow to fit, never overlap (Priority: P2)

**Goal**: the new type scale (title 17, body 14, nothing under 12), a 172 px minimum instead of a
fixed height, and every card in a grid row as tall as the tallest (FR-004, FR-014, FR-015). Done
before US1 because US1's header needs the room.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- even_row card_grid overview_card type_role_mapping`
and `mise run test-core`.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T021 [P] [US4] In `crates/micold-core/tests/tokens_scales.rs` assert `typography::OVERVIEW_CARD_TITLE` is 17 px size, 24 px line, weight 500, listed in `typography::SCOPED` and absent from `typography::ALL` (which keeps its fifteen roles). Red: the token does not exist (FR-004, R10)
- [ ] T022 [P] [US4] In `crates/micold-client/src/ui/material/type_role_mapping.rs` assert `TypeRole::CardTitle` resolves to `typography::OVERVIEW_CARD_TITLE`, and change `no_role_invents_a_size_outside_the_scale` to admit a role only when its name is in `typography::SCOPED` (one declared list, R10). Red: `CardTitle` does not exist (FR-004)
- [ ] T023 [P] [US4] Create `crates/micold-client/src/ui/cdk/even_row.rs` (`pub mod even_row;` in `crates/micold-client/src/ui/cdk/mod.rs`) with an `EvenRow` stub that lays out nothing, and in-file tests per K2: three children of natural heights 40/90/60 at 600 px wide, 3 columns, spacing 16, `min_height(50)` are each `(600 − 32)/3` wide and 90 tall at x = 0, 200, 400 (within 0.01); with `min_height(172)` each is 172 tall; two children in 3 columns leave the third slot empty and keep the slot width; a press inside the second child reaches it (events forwarded). Red: the stub lays out zero sizes (FR-014, US4 scenarios 2–3)
- [ ] T024 [US4] In `crates/micold-client/src/ui/material/card_grid.rs` tests (reuse `cards`, `grid`, `card_boxes`): a row where one card has every optional row and the others have none gives every card of that row the tallest card's height, which is greater than `overview_card::MIN_HEIGHT`; a lone sparse card is `MIN_HEIGHT` tall; the column counts of `the_laid_out_grid_has_the_columns_its_width_gives` are unchanged. At a 200 px grid width (under one minimum card) there is one card per row and no card overlaps another (spec edge case "Pane narrower than one minimum-width card"). Replace the `overview_card::HEIGHT` uses in this file with `MIN_HEIGHT` semantics. Red: cards are a fixed 172 (FR-014, US4 scenarios 2–3)
- [ ] T025 [US4] In `crates/micold-client/src/ui/material/overview_card.rs` tests: replace `the_card_with_every_row_fits_its_height` and `the_card_has_one_height_in_all_five_states` with (a) a card with every optional row (branch, activity, progress with bar, pull requests) laid out at 280, 380 and 480 px wide: no two text leaves overlap, every leaf sits inside the card, the card is at least `MIN_HEIGHT` tall, and a long title, branch and activity end in an ellipsis with the full text as tooltip; (b) every text leaf's size is ≥ 12 px and the title leaf is 17 px; (c) a sparse card given a 172 px minimum by its limits is 172 tall. Extend `no_text_overlaps_at_280_and_a_long_name_ellipsizes` to 380 and 480. Red: the title is 22 px, body lines 12 px, height fixed (FR-004, FR-015, US4 scenario 1, SC-002, SC-004)

### Implementation for User Story 4

- [ ] T026 [US4] Add `OVERVIEW_CARD_TITLE = role("overview_card_title", 17.0, 24.0, 500, 0.0)` and `pub const SCOPED: [TypeRole; 1]` to `crates/micold-core/src/tokens/typography.rs` (R10); T021 passes
- [ ] T027 [US4] Add `TypeRole::CardTitle` to `crates/micold-client/src/ui/material/text.rs` resolving to `OVERVIEW_CARD_TITLE`, and the `SCOPED` exception in `type_role_mapping.rs`; T022 passes
- [ ] T028 [US4] Implement `EvenRow` in `crates/micold-client/src/ui/cdk/even_row.rs` as an `advanced::Widget` per K2 and R12 (measure each child at slot width with unbounded height, `H = max(min_height, tallest)`, lay each out at exactly `slot × H`; forward draw, `update`, `operate`, `mouse_interaction` and `overlay` to the children; draw nothing itself; builder `new(children, columns).spacing(f32).min_height(f32)` and `From<EvenRow> for Element`); `crates/micold-client/tests/cdk_no_appearance.rs` scans every `ui/cdk/` file, so `EvenRow` names no colour, border or style; T023 passes and `cdk_no_appearance` stays green
- [ ] T029 [US4] In `crates/micold-client/src/ui/material/overview_card.rs` rename `HEIGHT` to `MIN_HEIGHT` (doc: the minimum, today's 172 px card, FR-014), drop `Length::Fixed(HEIGHT)` for a shrink height that accepts a larger minimum from its limits, draw the title at `TypeRole::CardTitle` and every body line at `TypeRole::Body`, nothing below `TypeRole::Label` (12 px); in `crates/micold-client/src/ui/material/card_grid.rs` build each row as `EvenRow::new(chunk, columns).spacing(spacing::MD).min_height(overview_card::MIN_HEIGHT)` (K3, column rule unchanged); fix every other `HEIGHT` user found with LSP `findReferences`; T024, T025 pass and every other `overview_card` and `card_grid` test stays green
- [ ] T030 [P] [US4] In `docs/user-guide/overview.md` *Layout*: cards are at least as tall as before, grow to fit what they show, and every card in a row is as tall as the row's tallest (FR-019)

**Checkpoint**: larger, legible type; cards grow; rows even; nothing overlaps at 280/380/480 px.

---

## Phase 6: User Story 1 - Read a card's state and type at a glance (Priority: P1)

**Goal**: a tinted header row (state left, type right) over a neutral body, every text at 4.5:1,
Idle still quieter, outline / focus ring / ripple over the whole card, shown in the showcase and
the layout snapshot (FR-001–FR-003, FR-005, FR-016, FR-017, FR-019).

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card composition_contrast`,
`--test layout_snapshot`, `--test showcase_completeness`, and quickstart §B.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T031 [P] [US1] In `crates/micold-client/src/ui/material/composition_contrast.rs` replace `an_overview_cards_text_and_icon_are_legible_on_its_state_fill` with a check over the new `Palette { header, on_header, body, on_body, on_body_variant }` for both schemes and the five states: `on_header` on `header`, `on_body` and `on_body_variant` on `body` each ≥ 4.5:1; `body` is the same for every state; Idle's `header` (`surface_variant`) differs from `body` and from the four coloured headers; run `the_card_target_outline_is_visible_on_every_state_fill` against both fills. Red: `Palette` has `fill`/`text`/`icon` (FR-002, FR-003, FR-005, SC-001, US1 scenarios 2–3)
- [ ] T032 [US1] In `crates/micold-client/src/ui/material/overview_card.rs` tests (K4): at 280, 380 and 480 px the header spans the card's full width at its top, the state glyph and word end left of where the type glyph and word start, and the type word shows its word as tooltip while the state word keeps the reason tooltip; the type word is no longer in the sessions line; a pixel probe in the body reads `Palette.body` in all five states and in the header reads that state's `Palette.header`; update `a_card_shows_its_type_and_state_with_their_words`, `the_targeted_card_draws_a_primary_outline_and_the_selected_layer` (the outline is visible over the header's top edge), `the_focused_card_draws_the_focus_ring` and `the_ripple_is_drawn_in_the_states_content_colour` (ripple in `on_body`) to the new anatomy. Red: no header row exists (FR-001, FR-002, FR-016, US1 scenarios 1, 4, 5, SC-005)

### Implementation for User Story 1

- [ ] T033 [US1] Replace `Palette` and `palette` in `crates/micold-client/src/ui/material/overview_card.rs` per R9 / data-model *Palette* (header: the state's `*_container` / `on_*_container`, Idle `surface_variant` / `on_surface_variant`; body for every state `surface_container_low` / `on_surface` / `on_surface_variant`); T031 passes
- [ ] T034 [US1] Build the card face per K1 / R13 in `crates/micold-client/src/ui/material/overview_card.rs`: a header `container` (fill `header`, top corners radius 12, padding `spacing::SM` × `spacing::MD`) holding `row![state cluster, Space::Fill, type cluster]` at `TypeRole::Label` in `on_header` with their tooltips; the body (padding `spacing::MD`, gap `spacing::XS`) with the title in `on_body` and every other line in `on_body_variant`; an outer container with the `body` fill and radius 12; the target outline and *selected* layer (in `on_body`) as the top layer of a `stack`; `Ripple` (colour `on_body`), `TakesTheKeyboard` (ring `secondary`, layer `on_body`) and `ContextArea` around the whole stack; `words()` keeps only the icons and header words; T032 and every T025 geometry test pass
- [ ] T035 [US1] In `crates/micold-client/src/showcase/sections/surfaces.rs` show the five state cards in a `CardGrid` (R15), cards with different rows so the even row height shows; `scripts/build-lock.sh cargo test -p micold-client --test showcase_completeness` passes (FR-017, SC-005)
- [ ] T036 [US1] Add the `overview-open` covered state to `crates/micold-client/tests/support/covered_states.rs` (the fixture project with the Overview open, anchors on the first card's header and title, R14), regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with `UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`, confirm in the fixture diff that only the Overview state is new, and that the text-overflow and containment gates pass on it (FR-015, FR-019)
- [ ] T037 [P] [US1] In `docs/development/layout-snapshot.md` *What is covered*, count and name the new Overview state (twelve states)
- [ ] T038 [P] [US1] In `docs/user-guide/overview.md` describe the header row in *One card per location* (state icon and word at the start, tinted in the state's colour; location type icon and word at the end; hover each for the reason or the type) and in *The five states, worst first* say the header carries the colour and Idle's header is grey (FR-019)
- [ ] T039 [US1] Visual pass per quickstart §B (B1–B8) with the `visual-pass` skill on the showcase in light and dark and on the client Overview; screenshots under `specs/1120-overview-card-readability/visual-pass/`, results in `specs/1120-overview-card-readability/visual-pass.md` (US1 scenarios 1–5, SC-002, SC-003, SC-005)

**Checkpoint**: all four stories hold; the full gate is green.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T040 Run every quickstart §A command and record the results in `specs/1120-overview-card-readability/quickstart.md` (no code change: the close unit does it in the close PR)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (T001)** blocks every story.
- **US2 (Phase 3)**: T002–T005 (red) → T006–T008 → T009 → T010; T011 any time.
- **US3 (Phase 4)**: needs US2's `overview_text` and `activity_line` (T007). T012–T014 (red) →
  T015 → T016, T017; T018, T019 any time; T020 last.
- **US4 (Phase 5)**: needs US2's card rendering (T010), so the full-row card it measures is the new
  one. T021–T025 (red) → T026 → T027, T028 → T029; T030 any time.
- **US1 (Phase 6)**: needs US4 (T029): the header band and 14 px body lines make a card with every
  row taller than 172 px, which only the minimum height and `EvenRow` absorb. T031, T032 (red) →
  T033 → T034 → T035, T036; T037, T038 any time; T039 last.
- **Polish (T040)**: after T039.

### User Story Dependencies

- **US2**: Foundational only.
- **US3**: US2 (activity line in `CardText`).
- **US4**: US2 (card renders `CardText`).
- **US1**: US4 (room for the header); priority order yields to this dependency (milestones rule 5).

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Constitution Principle I).
- Each story's user-guide task ships in its milestone (Principle VII).

### Parallel Opportunities

- US2: T004/T008 (`overview.rs`) beside T002/T003/T006/T007 (`overview_text.rs`); T011 (guide).
- US3: T012/T015 (`markup.rs`), T014/T017 (`hook_activity.rs`), T018 (showcase), T019 (guide).
- US4: T021 (core token test), T022 (`type_role_mapping.rs`), T023 (`even_row.rs`), T030 (guide).
- US1: T031 (`composition_contrast.rs`) beside T032; T037, T038 (docs).

## Parallel Example: User Story 3

```bash
Task: "T012 markup.rs stub and C1 tests"
Task: "T014 hook_activity.rs C10 tests"
Task: "T019 user guide: What the agent is doing now"
```

## Implementation Strategy

### MVP First

US2 (T001–T011): the card says the right things, with the rules in core. It ships alone and is
useful on its own.

### Incremental Delivery

US2 → US3 (no markup) → US4 (type scale, grow-to-fit) → US1 (header row, palette, snapshot,
showcase, visual pass). Each step leaves a working Overview on `main`.

## Notes

- Before renaming `summary` or `HEIGHT`, or replacing `Palette`'s fields, list every use with LSP
  `findReferences`.
- Never point `CARGO_TARGET_DIR` elsewhere; run tests through `scripts/build-lock.sh` or `mise run`.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Card content from core text rules 🎯 MVP

- **Tasks**: T001–T011
- **Deliverable**: an Overview card's title is the issue title (else the branch, else the folder
  name, on hover), the branch reads `type/number`, the sessions line reads `1 session · 1 running`,
  and no *No task progress* or `0 open · 0 merged` row appears; the rules live in `micold-core`
  and the user guide says so
- **Satisfies**: US2 acceptance scenarios 1–8; FR-006–FR-010, FR-018 (title, branch, sessions,
  progress, pull request rules); SC-003 (Story 2 part); edge cases long title, branch with a type but
  no number, unconventional type, project root, orphan, counts over 99
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card`;
  `scripts/gate.sh`
- **Depends on**: —
- **Tier**: full

### M2 — Activity line without markup

- **Tasks**: T012–T020
- **Deliverable**: a card whose agent was prompted with `<agent-message …>Continue M2</agent-message>`
  reads *Continue M2*, `/speckit-autopilot #1117` reads *Running autopilot on #1117*, a markup-only
  prompt shows no activity line, and the user guide describes it
- **Satisfies**: US3 acceptance scenarios 1–6; FR-011–FR-013, FR-018 (sanitiser), FR-020; SC-003
  (Story 3 part); edge cases nested markup, markup in arguments, `a < b`, several sessions
- **Verify**: `mise run test-core`; quickstart §A FR-020 grep; `scripts/gate.sh`
- **Depends on**: M1
- **Tier**: full

### M3 — Larger type, cards that grow, even rows

- **Tasks**: T021–T030
- **Deliverable**: card titles are 17 px and body lines 14 px; a card is at least 172 px tall and
  grows to fit its rows; every card in a grid row is as tall as the row's tallest; nothing overlaps
  at card widths 280, 380 and 480 px
- **Satisfies**: US4 acceptance scenarios 1–3; FR-004, FR-014, FR-015; SC-002, SC-004; edge cases
  issue title arriving while open, pane under 280 px
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- even_row card_grid overview_card type_role_mapping`;
  `mise run test-core`; `scripts/gate.sh`
- **Depends on**: M1
- **Tier**: full

### M4 — Tinted header row over a neutral body

- **Tasks**: T031–T039
- **Deliverable**: every card has a header row in its state's colour with the state at the start and
  the type at the end over one neutral body, all text at 4.5:1 in both themes, shown in the
  showcase's five-state grid and in a new `overview-open` layout-snapshot state
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001–FR-003, FR-005, FR-016, FR-017, FR-019;
  SC-001, SC-005; edge cases both themes and every state, orphan header type
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card composition_contrast`;
  `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`; `--test showcase_completeness`;
  quickstart §B recorded in `specs/1120-overview-card-readability/visual-pass.md`; `scripts/gate.sh`
- **Depends on**: M3
- **Tier**: full

T040 (quickstart §A results, no code) is left to the close unit (milestones rule 4).
