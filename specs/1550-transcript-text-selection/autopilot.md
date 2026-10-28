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
- **Next step**: plan unit (continuing handover): fix plan review round 1 findings, then round 2 (scoped, sonnet)

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
| Plan | 1 | 94244e6c1589e20bfeeeb532e3c9cd56beb33cee:783f9af1dc50a9469289c5e39f0e25d6a0ad62d2 | CHANGES: 2 MAJOR, 3 MINOR (open, see Handover) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan unit handed over at the 150k context cap (2026-10-28). Done: plan.md, research.md (R1–R13),
data-model.md, quickstart.md, contracts/{selection-model,selectable-text,keys-menu-copy}.md,
committed in 6a12d919b. branch-start.sh rebased the branch onto origin/main, so origin's copy of the
branch has diverged: pushing needs a force-with-lease, which this unit's permissions refused (not
pushed). Plan review round 1 (snapshot in *Review rounds*) returned CHANGES. Open findings, all
verified as holding:

- F1 MAJOR: rows without an `EntryId` (omitted notice `Row::Omitted`, permission prompts at the end
  `RowKey::Prompt(PromptId)` and under an entry via `with_trailing`, `ui/transcript/mod.rs:55-61`)
  have no `TextPos`. Fix: a row-level key (e.g. `RowPos { Omitted | Entry(EntryId) | Prompt(PromptId) }`)
  ordered with entries, say what `units()` and the lazy key do for them, add S-1/S-2/S-6 cases.
- F2 MAJOR: M4 too big for one session. Split: M4a leaf renderers + lazy key + parity test + budget;
  M4b SelectionArea wiring, edge scroll, context menu, user-guide selection text.
- F3 MINOR: name the call site that clears a selection when a pane shows another session (pane
  assign path in features/window.rs or app.rs), add to M2 with a two-pane test.
- F4 MINOR: `transcript_key` needs a `focused: bool` input (SelectAll/CopySelection require it), or
  state that update gates on `focused_field`; K-8/K-11 tests.
- F5 MINOR: `FileDiff::path` → `EditDiff::path` (`structured/edit_diff.rs:18`) in data-model.md and research.md R10.

Next: fix F1–F5, commit, run round 2 scoped on sonnet with
`scripts/autopilot/review-snapshot.sh diff 94244e6c1589e20bfeeeb532e3c9cd56beb33cee:783f9af1dc50a9469289c5e39f0e25d6a0ad62d2`,
then set Next step to `tasks unit`, push, return DONE.

## Open escalation

None.

## Follow-ups not done

None.
