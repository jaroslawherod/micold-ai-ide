# Autopilot ledger — 032-untitled-session-labels

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: see the screen show the name of past session is still not displayed even when the bug was reported and fixed — traced as [029 BUG-001](../029-persistent-session-names/bugs/BUG-001.md) (ledger: [BUG-001.autopilot.md](../029-persistent-session-names/bugs/BUG-001.autopilot.md)); its size decision sent the flow to Phase 1.
- **Kind**: feature
- **Worktree branch**: fix/the-name-of-past-session-is-still-not-shown
- **Started**: 2026-09-19
- **Phase**: 3-design
- **Next step**: Design unit (Phase 3: plan, tasks, milestones, checklists, through opening PR 2; the clarify commits ship with PR 2).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #386 | Spec (carries 029 BUG-001 record) | MERGED 2026-09-19 | — |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | 1-spec | Where does the fix go? | A new spec, 032, on this worktree's branch. The 029 BUG-001 size decision found the fix is new behaviour (contradicts 029 FR-004). | agent-resolved | 029 BUG-001.md §Size decision; BUG-001.autopilot.md D2 |
| D2 | 2-clarify | FR-002: which typed text is the label? | (b) the first turn: a slash command's arguments, else the command name; otherwise the prompt text. | decided by user | spec.md *Label-source evidence* |
| D3 | 2-clarify | FR-006: does a later title replace the label? | Yes, the title replaces the label on the row and in what is remembered. | decided by user | BUG-001 recommendation |
| D4 | 2-clarify | FR-012: is GitHub Copilot in scope? | Yes: `claude` and Copilot both get the fallback label (user chose over the recommended claude-only). | decided by user | spec.md FR-012 |
| D5 | 2-clarify r1 | Is a CLI-handled `claude` slash command (`/model`) a turn? | No; only prompt-sending commands (skills, custom commands) are turns. | agent-resolved | spec.md US1 #4, FR-001; Copilot records none (2/87 claude sessions open with bare `/model`) |
| D6 | 2-clarify r1 | Are identical first-turn labels disambiguated? | No, as identical titles are not. | agent-resolved | 029-persistent-session-names spec; 029-pi-cli-provider FR-011 |
| D7 | 2-clarify r1 | Is a label styled differently from a title? | No; same places, same presentation. | agent-resolved | spec.md Assumptions |
| D8 | 2-clarify r1 | Must FR-014's bound cover the Copilot sessions? | Yes, SC-008's 44 (first turn at record 2–10). | agent-resolved | spec.md *Copilot evidence* |

D2–D4 applied to spec.md (FR-002, FR-006, FR-012 + *Copilot evidence*, US1 #5, SC-008) on 2026-09-19.

| Clarify round | Questions | Agent-resolved | Escalated |
|---|---|---|---|
| 1 | 4 | 4 (D5–D8) | 0 |
| 2 | 0 (no critical ambiguities) | — | — |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

### Spec review log

| Round | Model | Verdict | Findings | Resolution |
|---|---|---|---|---|
| 1 | session (opus) | CHANGES | F1 MAJOR: FR-006 left open but US2/FR-008/entity presumed "yes". F2 MAJOR: bounded read decided only in an edge case, clashing with 100% SCs. F3 MAJOR: FR-002 marker lacked evidence; (c) undefined for claude. F4–F9 MINOR: 80-char cap unjustified, concurrency hazards, sensitive text, SC-005 unmeasurable, 029 assumptions superseded silently, checklist ticks. | All accepted. US2/FR-008/entity made conditional on FR-006; FR-014 bound + FR-015 user guide added; *Label-source evidence* table read from the four transcripts ((a) gives 3 identical rows); FR-003 reason + grapheme + ellipsis + tooltip; edge cases added; SC-005 measurable; superseded 029 assumptions listed; checklist unticked with note. |
| 2 | sonnet | CLEAN | one MINOR: "clear acceptance criteria" tick while FR-002/006/012 pending. | Note added to the checklist. |

## Open escalation

None (the 2026-09-19 FR-002/FR-006/FR-012 escalation was answered: D2–D4).

## Follow-ups not done

- Carried from 029 BUG-001 ledger: `custom-title` / `agent-name` records ignored by `ClaudeProvider::parse_title` (candidate 029 BUG, out of scope here).
- Found in Phase 2: `CopilotProvider::read_title` reads only `name:`; Copilot ≤1.0.36 wrote `summary:` instead (44 sessions on the dev machine). Candidate 029 bug, out of scope (spec *Out of scope*).
- Carried from 029 BUG-001 ledger: unverified `micold_time_track` untitled rows whose transcripts hold `ai-title`s.
