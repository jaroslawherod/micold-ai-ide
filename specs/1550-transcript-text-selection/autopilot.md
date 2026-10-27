# Autopilot ledger — #1550 transcript-text-selection

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1550 (issue #1550: Structured view: text and values cannot be selected or copied); labels: bug, flow:bug (now bug, flow:feature, in-progress; switched from the bug flow, which returned NEXT: feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1550
- **Worktree branch**: claude/project-thread-zj503u
- **Started**: 2026-10-27
- **Phase**: clarify
- **Next step**: clarify unit, continued with the user's answer to the open escalation (FR-012): apply it to spec.md, record it in Clarifications and Decisions, run another clarify round if it changed a requirement, then plan

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 14b551961ac69d5a737bde175d79033eaadcc4f0:828bc8e24442d9c9685869c35fe69134934bd233 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | 80048701cd16dc386ae196e8da6d4dd7a429becb:828bc8e24442d9c9685869c35fe69134934bd233 | CLEAN (2 MINOR fixed: rewrap, US2 scenario 6 for FR-019a) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle):

- **Q (FR-012)**: When the user releases the pointer at the end of a drag in the structured
  transcript, should the selection also be copied to the clipboard automatically?
  - **(Recommended) No: copy only on a copy key, the context menu's Copy, or a copy control.** The
    issue's "Expected" names Ctrl+C / Cmd+C as the copy action, not release; FR-004 already guards
    the clipboard against plain clicks; the structured view is a document view (like a browser or
    editor), where selecting never overwrites the clipboard, and auto-copy would silently replace
    clipboard contents the user meant to paste into the composer.
  - **Yes: copy on release, as the terminal view does** (docs/user-guide/worktrees-and-sessions.md
    "the selection is copied to the clipboard automatically on release";
    specs/006-real-terminal-emulator/spec.md FR-013). Consistent between the two session views;
    empty selections never write (006 FR-013c).

## Follow-ups not done

None.
