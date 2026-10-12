# Tasks: Agent-created worktrees carry the issue reference

**Input**: `specs/532-worktree-issue-reference/` (spec.md, plan.md, research.md, contracts/agent-guidance.md, quickstart.md)

Test-first (Constitution I): T001 to T003 are written and seen red before the string change they pin; T004 is a characterization test of existing behaviour and is green at once.
Paths are repo-relative. US1 = P1, US2 = P2, US3 = P3.

## Phase 1: User Story 1 - A session names a worktree after its issue (P1) 🎯 MVP

- [ ] T001 [US1] Write failing test T1 in `crates/micold-core/tests/mcp_tools_catalog.rs`: the `create_worktree` description contains `${type}/${ticket}_${name}`, `${type}-${ticket}_${name}`, the include-the-issue-reference instruction, and names `ticket` and `github_issue` rather than a literal `branch` (FR-001, FR-002, SC-004).
- [ ] T002 [US1] Write failing test T2 in the same file: the `ticket` property description says it drives the issue tag and the issue filter (FR-006).
- [ ] T003 [US1] Write failing test T4 in `crates/micold-core/tests/mcp_jsonrpc.rs`, extending the existing `instructions` phrase test: `initialize` instructions contain the convention string and the include-the-issue-reference rule (FR-007, SC-004).
- [ ] T004 [US1] Extend `type_ticket_and_name_create_the_forms_worktree` in `crates/micold-daemon/tests/mcp_create_worktree.rs` into test T3, a table over ticket form (`123`, `#123`, `ABC-123`, blank) x type x name, reusing `Fixture` and `parse_tags`: branch and directory equal `micold_core::naming::derive` for the same `WorktreeNaming`, and `parse_tags(ref)` holds the issue tag `TagFilter::HasIssue` keys on (FR-003, SC-001, SC-002). Expected green at once: it pins existing behaviour.
- [ ] T005 [US1] Update the `create_worktree` description in `crates/micold-core/src/mcp/tools.rs` per contract C1; T1 goes green (FR-001, FR-002).
- [ ] T006 [US1] Update the `ticket` property description in the same entry per contract C2; T2 goes green (FR-006).
- [ ] T007 [US1] Add the convention sentence to `INSTRUCTIONS` in `crates/micold-core/src/mcp/jsonrpc.rs` per contract C3, keeping the scope and confirmation sentences and the paragraph short; T4 goes green (FR-007).

## Phase 2: User Story 2 - Existing calls keep working (P2)

- [ ] T008 [US2] Confirm `literal_calls_are_unchanged`, `without_a_ticket_there_is_no_ticket_segment_or_tag`, `the_forms_naming_refusals_apply` and the `github_issue` module's test in `crates/micold-daemon/tests/mcp_create_worktree.rs` pass unmodified after T005 to T007 (FR-004, FR-005, FR-009, SC-003). Tighten `literal_calls_are_unchanged` so it asserts the literal-branch result has exactly the pre-feature keys, which pins FR-009 (no hint, warning or extra field); the same-issue collision refusal is inherited from feature 550 and not re-tested. Beyond that tightening, no code change.

## Phase 3: User Story 3 - The user guide describes the convention (P3)

- [ ] T009 [P] [US3] Update `docs/user-guide/agent-tools.md`: the "Creating a worktree from a type, ticket and name" section and the tool table row state the convention, the `ticket` input and `github_issue` (FR-008). The docs wording is checked by the diff review and quickstart B step 3, not by a script.
- [ ] T010 [P] [US3] Update `docs/user-guide/worktrees-and-sessions.md`: the naming section around the issue-tag paragraph describes agent-created worktrees carrying the issue reference (FR-008).

## Dependencies and order

- T001 to T004 before T005 to T007 (test first); T001 and T002 share a file, T005 and T006 share an entry.
- T008 after T007. T009 and T010 are independent of the code tasks and of each other.

## Implementation Strategy

One milestone on purpose: US2 is confirm-only (no code of its own) and US3 is the user-guide work that must ship with the behaviour (milestones rule 4). One small, text-only change in one merge: tests, the three strings and the two doc pages ship together, so the user-guide gate (Constitution VII) passes in the same `feat` PR. Then `mise run gate`, quickstart A.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Agent guidance carries the issue-reference convention 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: the `create_worktree` description, its `ticket` input description and the `initialize` instructions tell a session to name an issue's worktree `${type}/${ticket}_${name}` via `ticket` or `github_issue`, and the user guide says so; literal-`branch` calls behave as before.
- **Satisfies**: US1 acceptance scenarios 1–6; US2 acceptance scenarios 1–2; US3 acceptance scenarios 1–2; FR-001 to FR-009; SC-001 to SC-004
- **Verify**: `mise run test-core`; `cargo test -p micold-daemon --test mcp_create_worktree`; quickstart Part A, and Part B step 3 for the docs
- **Depends on**: —
- **Tier**: full
