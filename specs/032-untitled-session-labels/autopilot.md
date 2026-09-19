# Autopilot ledger — 032-untitled-session-labels

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: see the screen show the name of past session is still not displayed even when the bug was reported and fixed — traced as [029 BUG-001](../029-persistent-session-names/bugs/BUG-001.md) (ledger: [BUG-001.autopilot.md](../029-persistent-session-names/bugs/BUG-001.autopilot.md)); its size decision sent the flow to Phase 1.
- **Kind**: feature
- **Worktree branch**: fix/the-name-of-past-session-is-still-not-shown
- **Started**: 2026-09-19
- **Phase**: 3-design
- **Next step**: Design unit, awaiting the answer to the open escalation (Copilot old-session scope); then plan review round 2, tasks, milestones, analyze, tasks review, checklists, PR 2.

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
| D8 | 2-clarify r1 | Must FR-014's bound cover the Copilot sessions? | Yes, the 44 sessions' first turns (SC-009 since D9) (first turn at record 2–10). | agent-resolved | spec.md *Copilot evidence* |
| D9 | 3-design | Is Copilot's `summary:` read as the session title when `name:` is absent (Copilot 1.0.10–1.0.36 sessions)? | Yes: "read the old copilot summary as the name too". `name:` wins; `summary:` ranks above a derived label. Applied as FR-016, US1 #6, SC-008 (rewritten), SC-009. | decided by user | spec.md *Copilot evidence*; 44 sessions on the dev machine |

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

### Plan review log

| Round | Model | Verdict | Findings | Resolution |
|---|---|---|---|---|
| 1 | session (opus) | CHANGES | F1 BLOCKER: the 44 old Copilot sessions are in no Copilot index and not in micold's catalog, so they are never rows; reading `summary:` changes no row, SC-008/US1 #6 unmeetable. F2 MAJOR: claude CLI-command follower can be a `system/local_command` record (`/reload-plugins`). F3 MAJOR: labels not counted toward the live broadcast. F4 MAJOR: spinner-first order never re-arms `name_stale`. F5–F8 MINOR: Pending re-read cost, per-session writes, schema hash does not cover session.rs, FR-013 row / R6+R10 alternatives / workspace dep. | F2–F8 fixed in contract C3.5a–b′, C6.3a–b, research R3/R5/R6/R7/R8/R9/R10/R11, plan. F1 verified (44 no-name sessions, 0 in the 4 index ids) and escalated: it changes requirements (category 6). |

## Open escalation

**2026-09-19, Phase 3-design, category 6 (the plan proved false, and the fix changes requirements).**
Decision needed: the 44 old Copilot sessions (Copilot 1.0.10–1.0.36, `summary:` but no `name:`) are
not rows in micold at all: Copilot's per-cwd index (`~/.copilot/sidebar-sessions-state/*.json`, 4
ids in total) lists none of them, discovery reads only that index (026 research R3), and micold's
catalog knows none of them. The spec's "today all 44 read New session" was wrong, so D9 as applied
(US1 #6, SC-008) changes no row. Options:
- (A, recommended) Keep FR-016 (`summary:` is the title of any Copilot session micold lists) and
  rescope US1 #6 / SC-008 to listed sessions, measured by fixtures; no new discovery. Why: 026 R3
  chose the index as discovery's source and the scan only as a fallback; surfacing old sessions is a
  separate feature.
- (B) Also discover Copilot sessions by scanning `~/.copilot/session-state/*/workspace.yaml` for a
  matching `cwd:` when they are not in the index, which adopts old sessions into sidebars (30 of the
  44 would appear under the `max-speed` project), costing a 281-directory scan per project open.

## Follow-ups not done

- Carried from 029 BUG-001 ledger: `custom-title` / `agent-name` records ignored by `ClaudeProvider::parse_title` (candidate 029 BUG, out of scope here).
- Carried from 029 BUG-001 ledger: unverified `micold_time_track` untitled rows whose transcripts hold `ai-title`s.
