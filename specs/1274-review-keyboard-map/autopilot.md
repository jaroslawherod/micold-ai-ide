# Autopilot ledger — #1274 review-keyboard-map

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1274 "Review UI: keyboard map for reviewing": Blocked by: #1273, #1271, #1127. j/k file, n/p hunk, ]/[ comment, v reviewed, g next unreviewed, c comment, Ctrl+Shift+Enter send, ? shortcut sheet; the jump-to field finds files of the open review and comments. Part of the review UI redesign: `docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu (Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes) updated. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1274
- **Worktree branch**: claude/project-thread-m7gq7k
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks unit. Plan committed (plan, research, data-model, quickstart, contracts). Cut M1 core-only first (plan.md#milestones: REVIEW_KEYS + resolve, navigate, jump, header_rows; no blocker needed), then M2 after #1269/#1270/#1271/#1273, M3 after #1127. D4 still provisional.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | `g` single key or `g f` prefix (redesign table) for jump-to? | `g` = next unreviewed (issue wording); jump-to keeps Ctrl/Cmd+Shift+P | agent-resolved | issue #1274 body; spec.md#assumptions |
| D2 | spec | Do next/prev wrap? | j/k, ]/[, g wrap (803 step rule); n/p continue into adjacent file and wrap | agent-resolved | crates/micold-core/src/navigation.rs; spec.md FR-003, FR-004 |
| D3 | clarify 1 | Send chord with unsaved composer text: save+send, or saved drafts only? | Saved drafts only; composer stays open with its text and the keyboard | agent-resolved | spec.md#FR-010 ("exactly" Send review); crates/micold-client/src/features/changes.rs Msg::SendPressed; review-redesign.md proposal 8 |
| D4 | clarify 1 | FR-001: single-letter keys on by default or behind a Settings switch (redesign open question 5)? | On by default, no setting | provisional — recommended option, awaiting user | review-redesign.md open question 5; spec.md#terms, FR-002, SC-002; specs/803-keyboard-navigation/spec.md#assumptions. If the user picks a switch (on or off by default), FR-001 gains a Settings requirement and clarify re-runs |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 2f7c3dc307d7f818f4c2995e0ffd0af13213bfa9:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 3 MINOR, all fixed (FR-002 scope, FR-004 p/n with no current hunk, checklist note) |
| Review plan | 1 | 209eba49689ebf46a4457c2a5dd880637479b3d2:5ca86ffe05655dc3708ecbaa3676d58bdfc4f439 | CLEAN: 3 MINOR, all fixed (R7 why not `.key_binding`; timing bound 50 ms; pending edge keyed to its path) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (D4 is provisional; the coordinator's decision card to the user is still open.)

## Follow-ups not done

- Plan review ran via headless `claude -p --agent autopilot-reviewer`: Bash is denied there (workspace untrusted), so the reviewer used Read/Grep/Glob; a first run that only had denied Bash calls returned a vacuous CLEAN and was rerun. Symbols it could not confirm (`UnifiedIndex`, `SideIndex`, `jump_offset`, `settings::DiffLayout`, `is_primary_shift`, `Msg::SendPressed`, `window.focused_field`, `overlay::registry::topmost`) were confirmed by the plan unit with grep.
- Coordinator requirement from #1276 folded into the plan: public `micold_core::review::keys::REVIEW_KEYS` in M1; client binds only from it.

- Spec review ran via headless `claude -p --agent autopilot-reviewer` (read-only tools): no Agent tool in this unit.

- Blockers #1269/#1270/#1271/#1273/#1127 not merged at spec time; spec states the dependency (spec.md#dependencies).
