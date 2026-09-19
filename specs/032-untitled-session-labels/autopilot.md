# Autopilot ledger — 032-untitled-session-labels

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: see the screen show the name of past session is still not displayed even when the bug was reported and fixed — traced as [029 BUG-001](../029-persistent-session-names/bugs/BUG-001.md) (ledger: [BUG-001.autopilot.md](../029-persistent-session-names/bugs/BUG-001.autopilot.md)); its size decision sent the flow to Phase 1.
- **Kind**: feature
- **Worktree branch**: fix/the-name-of-past-session-is-still-not-shown
- **Started**: 2026-09-19
- **Phase**: 1-spec
- **Next step**: Wait for PR 1's `ci complete`, merge on green, then Phase 2 clarify (FR-002, FR-006, FR-012).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #386 | Spec (carries 029 BUG-001 record) | open | — |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | 1-spec | Where does the fix go? | A new spec, 032, on this worktree's branch. The 029 BUG-001 size decision found the fix is new behaviour (contradicts 029 FR-004). | agent-resolved | 029 BUG-001.md §Size decision; BUG-001.autopilot.md D2 |

### Open `[NEEDS CLARIFICATION]` for Phase 2

- FR-002: which typed text is the label (first plain typed prompt; first turn incl. slash-command args or command name; `pi` precedent first user message).
- FR-006: whether a title that arrives after a label has been shown replaces it.
- FR-012: whether GitHub Copilot is in scope (`pi` already covered by 029-pi-cli-provider FR-011).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

### Spec review log

| Round | Model | Verdict | Findings | Resolution |
|---|---|---|---|---|
| 1 | session (opus) | CHANGES | F1 MAJOR: FR-006 left open but US2/FR-008/entity presumed "yes". F2 MAJOR: bounded read decided only in an edge case, clashing with 100% SCs. F3 MAJOR: FR-002 marker lacked evidence; (c) undefined for claude. F4–F9 MINOR: 80-char cap unjustified, concurrency hazards, sensitive text, SC-005 unmeasurable, 029 assumptions superseded silently, checklist ticks. | All accepted. US2/FR-008/entity made conditional on FR-006; FR-014 bound + FR-015 user guide added; *Label-source evidence* table read from the four transcripts ((a) gives 3 identical rows); FR-003 reason + grapheme + ellipsis + tooltip; edge cases added; SC-005 measurable; superseded 029 assumptions listed; checklist unticked with note. |
| 2 | sonnet | CLEAN | one MINOR: "clear acceptance criteria" tick while FR-002/006/012 pending. | Note added to the checklist. |

## Open escalation

None.

## Follow-ups not done

- Carried from 029 BUG-001 ledger: `custom-title` / `agent-name` records ignored by `ClaudeProvider::parse_title` (candidate 029 BUG, out of scope here).
- Carried from 029 BUG-001 ledger: unverified `micold_time_track` untitled rows whose transcripts hold `ai-title`s.
