# Autopilot ledger — #613 notification-kinds

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #613 "Notifications: only notify when a session needs attention; add per-kind settings and icons" (notify by default only when a session needs real attention — waiting for input or permission, errored, finished a long task; per-kind on/off settings; a distinct icon per kind; builds on specs/039 and 575). labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #613
- **Worktree branch**: claude/project-thread-8kdqkn
- **Started**: 2026-10-06
- **Phase**: clarify
- **Next step**: clarify unit — awaiting user answer on long-task threshold (Open escalation), then apply to spec.md and run the next clarify round

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Do disabled kinds change unread marks? | No: unread stays as 039 (FR-018) | agent-resolved | issue #613 asks about notifications only; 039 FR-017 |
| D2 | spec | Defaults per kind | Needs permission, Session error, Long task finished on; Turn finished off | agent-resolved | issue #613 "Expected" bullet 1 |
| D3 | spec | Keep 039's Desktop notifications switch? | Yes, as master switch above per-kind switches | agent-resolved | 039 FR-026; stored value carries over |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c48e430ee5881189ede316fafd185dd594ab7342:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CHANGES: 1 MAJOR, 2 MINOR (all fixed) |
| Spec | 2 | 4e5c412acfa49fdd6f0f43625e21fd7c3ebc035a:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CLEAN (1 MINOR, fixed: FR-017 3:1 vs white and black) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

- **Clarify round 1, Q1 (category 1, product/scope)**: Is the long-task threshold fixed at 60 s, or user-configurable in Settings?
  - A (Recommended) fixed at 60 s: issue #613 asks only for per-kind on/off; keeps FR-009 to FR-014 unchanged; a setting can be a later request.
  - B adjustable in Settings (seconds field, default 60, beside **Long task finished**): precedent of a numeric seconds field with clamped range in `crates/micold-core/src/settings.rs` (`env_include_timeout_secs`, 1 to 60) and `docs/user-guide/settings.md` "Timeout (seconds)"; adds requirements, validation and showcase states.
  - Repo checked: issue #613 body, spec.md Assumptions, specs/039 and 575 specs, constitution: none settles it.

## Follow-ups not done

None.
