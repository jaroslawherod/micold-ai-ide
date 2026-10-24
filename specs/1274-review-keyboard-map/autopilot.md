# Autopilot ledger — #1274 review-keyboard-map

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1274 "Review UI: keyboard map for reviewing": Blocked by: #1273, #1271, #1127. j/k file, n/p hunk, ]/[ comment, v reviewed, g next unreviewed, c comment, Ctrl+Shift+Enter send, ? shortcut sheet; the jump-to field finds files of the open review and comments. Part of the review UI redesign: `docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu (Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes) updated. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1274
- **Worktree branch**: claude/project-thread-m7gq7k
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: clarify unit (continued): apply the user's answer to the Open escalation to FR-001, record it in spec.md#clarifications, then decide on a next round

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 2f7c3dc307d7f818f4c2995e0ffd0af13213bfa9:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 3 MINOR, all fixed (FR-002 scope, FR-004 p/n with no current hunk, checklist note) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle). FR-001 marker:
are the single-letter review keys (j k n p ] [ v g c ?) on by default, or behind a Settings switch?
`docs/development/review-redesign.md` lists it as open question 5 for the user; the mockup and
issue #1274 say nothing; the app has no key-binding settings today (specs/803-keyboard-navigation/spec.md#assumptions).

- **On by default, no setting (Recommended)**: keys act only while the review has the keyboard
  (spec Terms, FR-002, SC-002), so typing in a field or terminal is never taken; GitHub/Gerrit-style
  review keys are always-on; no new setting to build, store and document.
- **On by default, Settings switch to turn off**: adds a Settings toggle and its storage; a user who
  presses keys by accident can opt out.
- **Off by default, Settings switch to turn on**: matches the redesign's pattern for suggest chips
  (proposal item 9) but hides the feature; SC-001 then needs the switch on first.

## Follow-ups not done

- Spec review ran via headless `claude -p --agent autopilot-reviewer` (read-only tools): no Agent tool in this unit.

- Blockers #1269/#1270/#1271/#1273/#1127 not merged at spec time; spec states the dependency (spec.md#dependencies).
