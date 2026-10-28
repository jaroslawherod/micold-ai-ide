# Autopilot ledger — #1550 transcript-text-selection

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1550 (issue #1550: Structured view: text and values cannot be selected or copied); labels: bug, flow:bug (now bug, flow:feature, in-progress; switched from the bug flow, which returned NEXT: feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1550
- **Worktree branch**: claude/project-thread-zj503u
- **Started**: 2026-10-27
- **Phase**: plan
- **Next step**: plan unit: plan review round 2 (scoped, sonnet) on the F1–F5 fixes

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Per-block or cross-block selection? | Cross-block: one range across entries in reading order | agent-resolved | issue #1550 "Expected" (across lines and blocks); spec.md#assumptions |
| D2 | spec | Ctrl+C: copy or interrupt (#1403)? | Transcript focused + non-empty selection: copy, never interrupt; no selection: #941 FR-012a unchanged; Cmd+C / Ctrl+Shift+C never interrupt | agent-resolved | spec.md FR-008, FR-009; specs/941-structured-prompt-input/spec.md FR-012a |
| D3 | spec | Which values get copy controls? | Tool card identifying value (header), full input, full result, diff file path; code blocks keep #940's control; no whole-message control | agent-resolved | spec.md FR-015–FR-018 |
| D4 | spec | Platform behaviour? | Identical on Linux/macOS/Windows; only Cmd vs Ctrl differs; LF line breaks everywhere | agent-resolved | spec.md Terms (Copy keys), Edge Cases (Cross-platform) |
| D5 | clarify | FR-011: copy rendered markdown as visible text or source? | Visible rendered text as drawn; links copy shown text; no emphasis/heading/fence markup | agent-resolved | spec.md FR-010 ("as shown"), SC-001; user guide code-block copy "no fences, no tag" (#940 FR-011) |
| D6 | clarify | FR-012: copy to clipboard on drag release, like the terminal view? | No: copy only on request (copy key, context menu Copy, copy control); release never writes the clipboard | decided by user | user answer 2026-10-28 ("No, copy only on request"); spec.md FR-012 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 14b551961ac69d5a737bde175d79033eaadcc4f0:828bc8e24442d9c9685869c35fe69134934bd233 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | 80048701cd16dc386ae196e8da6d4dd7a429becb:828bc8e24442d9c9685869c35fe69134934bd233 | CLEAN (2 MINOR fixed: rewrap, US2 scenario 6 for FR-019a) |
| Plan | 1 | 94244e6c1589e20bfeeeb532e3c9cd56beb33cee:783f9af1dc50a9469289c5e39f0e25d6a0ad62d2 | CHANGES: 2 MAJOR, 3 MINOR (all fixed: F1 RowPos row key, F2 M4 split into M4a/M4b, F3 retain_shown in shell/panes.rs::sync, F4 `focused` input to transcript_key, F5 EditDiff::path) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
