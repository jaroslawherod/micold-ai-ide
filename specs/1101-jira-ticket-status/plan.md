# Implementation Plan: Jira Ticket Status

**Branch**: `feat/1101_show-a-jira-ticket-s-current-status` | **Date**: 2026-10-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1101-jira-ticket-status/spec.md`

## Summary

Read each Jira ticket's status (name and category) and show it as a tinted chip on the picker rows,
the sidebar rows of worktrees linked to a ticket, and their Overview cards. The picker gets its
status for free by adding `status` to the fields the list and search requests already ask for. Linked
tickets are read by key in one batched `key in (...)` search (statuses of done tickets included), on
two events only: project open and window focus when the last read is over 5 minutes old. One pure
`StatusBoard` in `micold-core` holds the answers, so every surface and window draws the same status
(FR-010); the client shell runs the request through feature 533's single Jira gate and feeds the
result back as a message. A failed read dims the last status; a not-found ticket, or 3 failures,
retire it. Design: [research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/), [quickstart.md](quickstart.md).

## Technical Context

**Language/Version**: Rust 2021 workspace (`rust-toolchain` as the repo pins it).

**Primary Dependencies**: existing only: `iced` (UI), `ureq` 3 (`JiraCloud`), `serde_json` (parse).
No new crate.

**Storage**: none new. Statuses live in memory (`StatusBoard`), are refetched on open, and are never
persisted (Principle IV: nothing new leaves the device; the only traffic is the existing opt-in Jira
integration to the user's own site).

**Testing**: `cargo test` via `mise run test-core` (parse, JQL, board reducer, chip model) and
`mise run gate`; client integration tests in `crates/micold-client/tests/`; geometry gates; quickstart
Part B visual pass against `examples/fake_jira.rs`.

**Target Platform**: Linux, macOS, Windows (iced desktop client; the daemon is untouched).

**Project Type**: desktop-app, Rust workspace (`micold-core` render-free logic, `micold-client` GUI).

**Performance Goals**: 1 Jira request for up to 100 linked tickets (SC-003); the refresh runs on a
blocking thread and never blocks `view`/`update`.

**Constraints**: 10 s request timeout (existing `REQUEST_TIMEOUT`); no request when switch off or
project unconnected (SC-004); token and site never in text or logs (FR-012).

**Scale/Scope**: tens of linked tickets per project; chunks of 100 keys beyond that.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design: unchanged.*

- [x] **I. Test-First**: every unit starts from a failing test; the test list
  (quickstart "Test layers") names the layer per requirement. PASS.
- [x] **II. Multi-Session**: `StatusBoard` is per project and shared by every window and session of
  it (FR-010); concurrent triggers coalesce on an in-flight flag; no window writes to Jira. PASS.
- [x] **III. Worktree Integration**: reads tickets named by worktrees; no file/VCS operation. PASS.
- [x] **IV. Local-First**: no new storage or service; the feature works offline (last status stays,
  dimmed); the traffic is feature 533's opt-in, widened as FR-006b says and disclosed. PASS.
- [x] **V. Rust + iced**: `StatusCategory` is an enum; a board entry is a `TicketStatus` or absent,
  so "chip with category but no name" is unrepresentable. PASS.
- [x] **VI. Cross-Platform**: no platform code; chip text carries the meaning, not tint alone. PASS.
- [x] **VII. Documentation**: user-guide `settings.md`, `worktrees-and-sessions.md`, `overview.md`
  updated in the same change, plus the disclosure text (FR-006b). PASS.
- [x] **VIII. Reusable UI**: one `StatusChip` builder (not `naming::Tag::Status`, the worktree-health tag) in `ui/material/` built on the shared `Tag`,
  used by all three surfaces; no feature-local chips. PASS.

## Project Structure

### Documentation (this feature)

```text
specs/1101-jira-ticket-status/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── jira-status-source.md   # the JiraSource addition, JQL, parsing
│   └── status-chip-ui.md       # chip, surfaces, refresh events
└── tasks.md                    # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── jira.rs                 # + status field in parse_page; JiraSource::statuses; FakeJiraSource script
├── jira_status.rs          # NEW: TicketStatus, StatusCategory, StatusBoard, statuses_jql, parse_statuses
└── github.rs               # Issue + status: Option<TicketStatus> (Jira rows only)

crates/micold-client/
├── src/shell/jira_http.rs  # FIELDS + "status"; JiraCloud::statuses
├── src/shell/jira_refresh.rs# NEW: refresh driver (gate, spawn, apply, triggers)
├── src/shell/issues.rs     # jira_request / JiraRequest widened to pub(crate)
├── src/main.rs             # WindowFocusChanged(true) -> refresh trigger
├── src/shell/daemon_sync.rs, startup.rs  # project change / restore -> on_project_changed
├── src/main_tests.rs       # `Recording` JiraSource impl updated where a test needs statuses
├── src/ui/material/status_chip.rs  # NEW: StatusChip builder over Tag
├── src/ui/worktree_form.rs # picker row chip
├── src/features/sidebar.rs # row chip beside the issue tag
├── src/features/overview.rs, src/ui/material/overview_card.rs  # card chip
├── src/ui/settings/jira.rs # disclosure text (FR-006b)
├── examples/fake_jira.rs   # reports and changes statuses
└── tests/                  # see quickstart; jira_is_requested_only_on_named_events.rs extended
docs/user-guide/{settings,worktrees-and-sessions,overview}.md
```

**Structure Decision**: logic in `micold-core` (render-free, `mise run test-core`), IO and message
plumbing in the client `shell/`, drawing in `ui/`, matching feature 533's layout.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 | `parse_page` reads `fields.status`; `FIELDS` gains `status` ([contract §1-2](contracts/jira-status-source.md)) |
| FR-002, FR-009 | `ui/worktree_form.rs` row + `StatusChip::from_status` returns `None` for unusable status |
| FR-003, FR-004, FR-013 | `StatusChip` ([contract](contracts/status-chip-ui.md)); sidebar row |
| FR-005, FR-006 | `shell/jira_refresh.rs` triggers and `StatusBoard::due` ([data-model](data-model.md)) |
| FR-006b | `ui/settings/jira.rs` disclosure; extended named-events test; 533 amendment note in research R6 |
| FR-007 | `JiraSource::statuses` by `key in (...)`, chunks of 100 ([contract §3](contracts/jira-status-source.md)) |
| FR-008 | `StatusBoard::apply` (stale, retire on not-found/off/3 failures) |
| FR-010 | single `StatusBoard` in core state read by all surfaces |
| FR-011 | no write path; picker JQL unchanged; GitHub rows untouched |
| FR-012 | statuses path reuses `outcome_of` redaction; test extends `jira_token_never_leaks` |
| Story 3 / Overview | `overview_card.rs` chip slot fed from the same board |

## Test strategy (layer per requirement)

- **Core unit** (`mise run test-core`): `parse_page` status, `parse_statuses`, `statuses_jql`
  (quoting, chunking, bisection on 400), `StatusBoard` (apply, stale, retire, due, coalesce, epoch),
  and `TicketStatus::accessible_text` / `StatusCategory` tint class (both in `jira_status.rs`):
  FR-001, 005, 007, 008, 009, 010, 013. The client keeps only the colour lookup and the
  `StatusChip` drawing: client unit test for FR-004 (distinct tint per category).
- FR-011 (no write path, picker JQL unchanged): core unit on `list_jql`/`search_jql` unchanged plus
  the existing client picker tests. SC-002: client integration with an injected clock.
- **Client integration** (`crates/micold-client/tests/`, fake source): picker rows (FR-002), sidebar
  row follows a changed fake (US2), Overview card (US3), gate and named events (FR-006, FR-006b,
  SC-004), 50 tickets -> 1 request (SC-003), failure kinds (SC-005), token never leaks (FR-012).
- **Geometry gates**: long status name truncates without moving key/title (edge case); row unchanged
  with no chip (FR-009).
- **Quickstart Part B** (visual pass, `visual-pass` skill): tint distinctness, dimmed chip, layout.

## Complexity Tracking

No violations.
