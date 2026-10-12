# Implementation Plan: Agent-created worktrees carry the issue reference

**Branch**: `feat/532_agent-created-worktrees-should-carry-the-issue` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/532-worktree-issue-reference/spec.md`

## Summary

`create_worktree` already derives `${type}/${ticket}_${name}` from `type`, `ticket`, `name` and
`github_issue` through `micold_core::naming::derive`, the function the create-worktree form calls
(feature 550). Sessions still pass a literal `branch` because nothing tells them not to. This feature
changes the text callers read: the tool description, the `ticket` schema description and the
`initialize` instructions, plus the user guide, and pins both with tests. No input, algorithm or
result field changes. Full reasoning: [research.md](research.md).

## Technical Context

**Language/Version**: Rust (workspace toolchain, `rust-toolchain.toml`).

**Primary Dependencies**: existing only (`micold-core` mcp::{tools, jsonrpc}, naming; `micold-daemon`
mcp::tools). No new crate.

**Storage**: N/A. Nothing is stored; the strings are compiled in.

**Testing**: `cargo test` via `mise run test-core` (catalog, instructions) and `mise run gate`
(daemon integration, docs scripts).

**Target Platform**: Linux, macOS, Windows; the strings and the shared naming code are platform
independent.

**Project Type**: desktop app plus daemon (existing workspace).

**Performance Goals**: N/A.

**Constraints**: a call that passes only `branch` returns the same result as before (FR-004, FR-009).
The `initialize` instructions stay one short paragraph.

**Scale/Scope**: three string literals, two doc pages, four tests.

## Constitution Check

*GATE: passes before Phase 0; re-checked after Phase 1 (nothing changed).*

- [x] **I. Test-First**: the string changes are driven by failing catalog and instructions tests
  written first (see Test strategy); no production line without a red test.
- [x] **II. Multi-Session Support**: PASS. No session state is added or touched.
- [x] **III. Worktree Integration**: PASS. The change makes agent-created worktrees carry the same
  tags as dialog-created ones; creation still goes through the existing `ops::create_worktree`.
- [x] **IV. Local-First Storage**: PASS. Compiled-in text; no remote service (`github_issue`
  behaviour is unchanged).
- [x] **V. Rust + iced Stack**: PASS. Rust only; no UI code.
- [x] **VI. Cross-Platform Parity**: PASS. Same strings and same naming code on every platform; no
  platform branch.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/agent-tools.md` and
  `worktrees-and-sessions.md` change in the same PR (FR-008).
- [x] **VIII. Reusable UI Component Foundation**: PASS. No UI.

## Requirement mapping

| Requirement | Where |
|---|---|
| FR-001, FR-002 | `description` of the `create_worktree` entry, `crates/micold-core/src/mcp/tools.rs` (contract C1) |
| FR-006 | `ticket` property `description`, same entry (C2) |
| FR-007 | `INSTRUCTIONS` in `crates/micold-core/src/mcp/jsonrpc.rs` (C3) |
| FR-003 | unchanged: `micold_core::naming::derive`; pinned by the parity test (T3) |
| FR-004, FR-005, FR-009 | no code change; pinned by existing `literal_calls_are_unchanged` and `without_a_ticket_there_is_no_ticket_segment_or_tag`; result shape untouched |
| FR-008 | `docs/user-guide/agent-tools.md` ("Creating a worktree from a type, ticket and name" and the tool table row), `docs/user-guide/worktrees-and-sessions.md` (naming section around the issue-tag paragraph) |

## Test strategy

| Layer | Test | Covers |
|---|---|---|
| core unit, `crates/micold-core/tests/mcp_tools_catalog.rs` | T1: `create_worktree` description contains `${type}/${ticket}_${name}`, `${type}-${ticket}_${name}`, the instruction to include the issue reference, and names `ticket` and `github_issue` as the way rather than a literal `branch` | FR-001, FR-002, SC-004 |
| core unit, same file | T2: `ticket` schema description says it drives the issue tag and the issue filter | FR-006 |
| core unit, `crates/micold-core/tests/mcp_jsonrpc.rs` (extends the existing `instructions` phrase test) | T4: `initialize` instructions contain the convention string and the include-the-issue-reference rule | FR-007, SC-004 |
| daemon integration, `crates/micold-daemon/tests/mcp_create_worktree.rs` | T3: extends the existing `type_ticket_and_name_create_the_forms_worktree` into a table over ticket form (`123`, `#123`, `ABC-123`, blank) x type x name, reusing `Fixture` and `parse_tags`; branch and directory equal `micold_core::naming::derive` for the same `WorktreeNaming`, and `parse_tags(ref)` holds the issue tag `TagFilter::HasIssue` keys on | FR-003, SC-001, SC-002, US1 s2 to s4 |
| daemon integration (existing) | `literal_calls_are_unchanged`, `without_a_ticket_there_is_no_ticket_segment_or_tag`, `the_forms_naming_refusals_apply` and the `github_issue` module's test (`{"github_issue": 123}`) stay green unmodified | FR-004, FR-005, SC-003, US1 s4, US2 |
| docs | no automated wording assertion (the `scripts/tests/*.test.sh` checks cover structure and links only); verified by the diff review and quickstart B step 3 | FR-008 |
| geometry gates, quickstart B | not applicable: no UI change | |

## Project Structure

### Documentation (this feature)

```text
specs/532-worktree-issue-reference/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/agent-guidance.md
└── tasks.md             # /speckit-tasks, not created here
```

### Source Code (repository root)

```text
crates/micold-core/src/mcp/tools.rs       # create_worktree description, ticket property text
crates/micold-core/src/mcp/jsonrpc.rs     # INSTRUCTIONS (and its doc comment, which names only scope and policy)
crates/micold-core/tests/mcp_tools_catalog.rs
crates/micold-core/tests/mcp_jsonrpc.rs
crates/micold-daemon/tests/mcp_create_worktree.rs
docs/user-guide/agent-tools.md
docs/user-guide/worktrees-and-sessions.md
```

**Structure Decision**: edit in place; every path above exists. No new module.

## Complexity Tracking

No violations.
