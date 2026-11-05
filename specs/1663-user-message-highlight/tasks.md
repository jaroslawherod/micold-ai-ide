# Tasks: Highlighted user messages in the structured view

**Input**: `specs/1663-user-message-highlight/` (spec.md, plan.md, research.md, data-model.md, contracts/user-turn.md, quickstart.md)

**Tests**: required (Constitution I): each test task precedes the code it covers and is run red first.

Format: `[ID] [P?] [Story] Description` — `[P]` = different files, no dependency.

## Phase 1: Setup

- [x] T001 Read `crates/micold-client/src/ui/transcript/entries.rs` (the `EntryKind::UserMessage` arm near line 271, both the command and plain branch) and `crates/micold-material/src/style.rs` (`queue_row`, `prompt_card`); note the shared-primitive pattern (`Divider`/`DisclosureCard`) to copy in `micold-material/src/lib.rs`. Decide the highlight probe for the client test (renderer quad probe, else a pure `is_user_turn(&EntryKind) -> bool` used at the wrap site) and add it to `research.md` as R6. Also confirm that queued-delivered and sub-agent user turns reach the one `EntryKind::UserMessage` arm (else add wrap sites in T007). If the fallback predicate is the probe, T006 also asserts the wrap site calls `UserTurn` (a source-level or builder-level check), since the predicate alone proves nothing.

## Phase 2: Foundational (tokens)

- [x] T002 [P] [US3] Test first, in `crates/micold-core/tests/tokens.rs`: for light and dark, assert `contrast(surface_container_high, background)` and `contrast(surface_container_high, surface_container_low)` are at least 1.05 (read the real `contrast()` values and record them in `research.md` R1); also the same pair against the command-output fill (tool and command cards use `surface_container_low` plus a shadow; confirm in `style.rs`) and that the selection colour (`primary` at alpha 0.3 over the fill) keeps `on_surface` text at least `AA_TEXT`, `contrast(on_surface, surface_container_high) >= AA_TEXT`, the same for `on_surface_variant`, and `contrast(primary, surface_container_high) >= AA_NON_TEXT` (FR-001a, FR-002, FR-007, SC-002). Run `mise run test-core`; if a pair fails, escalate to the plan (choose the nearest role) before continuing.

## Phase 3: User Story 1 + 2 — highlight on every user turn (P1) 🎯 MVP

- [x] T003 [P] [US1] Test first (a compile failure on the missing `UserTurn` counts as red), `crates/micold-material/tests/user_turn.rs`: `style::user_turn_surface(roles)` returns background `surface_container_high` with no shadow and the small corner; `style::user_turn_edge(roles)` returns background `primary`; `UserTurn::new(content, roles)` converts into an `Element` under light and dark roles; empty content builds (FR-001, FR-002, FR-006).
- [x] T004 [US1] Add `user_turn_surface(r)` and `user_turn_edge(r)` to `crates/micold-material/src/style.rs` beside `queue_row`/`prompt_card`; tokens only, no `Color::from_rgb` (FR-006).
- [x] T005 [US1] Create `crates/micold-material/src/user_turn.rs` (`UserTurn::new(content, roles)`, `From<UserTurn> for Element`): a `Row` of a 4dp fixed-width edge container and the content padded `spacing::SM`, width `Fill`, in a container using the surface style; adds no text or message (contracts/user-turn.md). Export `pub mod user_turn; pub use UserTurn` from `crates/micold-material/src/lib.rs`. Run `scripts/build-lock.sh cargo test -p micold-material user_turn` (T003 green).
- [x] T006 [US2] Test first, `crates/micold-client/tests/transcript_entry_view.rs`: using the T001 probe, a typed `UserMessage`, a queued-delivered `UserMessage` (same kind), a command `UserMessage` (folded and unfolded) and one with attachments show the highlight; assistant text, thinking, tool and command-output entries do not; empty text renders; a sub-agent-scope user turn shows it (FR-003, FR-004, FR-005, FR-009, SC-004). Red first (a compile failure on the missing `UserTurn` counts as red; stub it if the tests must run). Include a case where a new entry is appended while a command card is unfolded: the fold state survives, and two consecutive user turns both show the highlight.
- [x] T007 [US2] In `crates/micold-client/src/ui/transcript/entries.rs` wrap both `EntryKind::UserMessage` branches in `UserTurn::new(column, roles).into()`; leave the `vec![..]` selection paths, entry iteration and scroll code untouched (FR-003, FR-008, FR-010). T006 green.
- [x] T008 [US3] Run unchanged and confirm green: `scripts/build-lock.sh cargo test -p micold-client --test features_transcript_selection` (FR-008, SC-003) and the geometry gates `transcript_follow_layout`, `transcript_pane`, `transcript_open_budget`; update a gate's expected value only if it pins a user-turn height, by the wrap's padding (SC-005).
- [x] T009 [US1] User guide: add a short "Your messages" note to `docs/user-guide/structured-transcript.md` (Constitution VII; CI's user-guide gate fires on `feat`).
- [x] T010 [US1] Visual pass per `quickstart.md` §B with the `visual-pass` skill, light then dark, all listed transcript cases; add `spacing::XS` bottom padding in `UserTurn` only if back-to-back turns touch; save screenshots and the verdict as `specs/1663-user-message-highlight/visual-pass.md` (FR-011, SC-001).

## Dependencies & Order

T001 → T002 ∥ T003 → T004 → T005 → T006 → T007 → T008 → T009, T010. Tests T002, T003, T006 are written and seen red before their code.

## Implementation Strategy

One milestone: the tint without the accent edge, or without every user-turn form, would mislead (spec US2 rationale), and the three stories share one primitive and one call site (about 150 lines of change). No split along an acceptance scenario gives a separate deliverable.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Highlighted user turns 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: Every user turn (typed, delivered queued message, slash command card, main and sub-agent scope) in the structured view shows a tinted `surface_container_high` surface with a 4dp `primary` left edge, in light and dark, with selection and copy unchanged and the user guide updated.
- **Satisfies**: US1 acceptance scenarios 1–3; US2 acceptance scenarios 1–4; US3 acceptance scenarios 1–3; FR-001–FR-011; SC-001–SC-005
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-material --test user_turn`; `scripts/build-lock.sh cargo test -p micold-client --test main transcript_entry_view`; quickstart §B recorded in `visual-pass.md`
- **Depends on**: —
- **Tier**: full
