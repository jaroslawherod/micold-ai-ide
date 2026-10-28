# Autopilot ledger — #1550 transcript-text-selection

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1550 (issue #1550: Structured view: text and values cannot be selected or copied); labels: bug, flow:bug (now bug, flow:feature, in-progress; switched from the bug flow, which returned NEXT: feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1550
- **Worktree branch**: claude/project-thread-zj503u
- **Started**: 2026-10-27
- **Phase**: M1
- **Next step**: M1 reviews and gate, then M1 PR (orchestrator opens it)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1582 | design (spec, plan, tasks) | merged | 5ce410f219e38800d89f0936dbf9f48a96041cb1 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Selection model, shown text and work_value in core | — | in progress |
| M2 | T013–T023 | full | Window selection state, clearing, copy text and copy-key arbitration (render-free) | — | todo |
| M3 | T024–T032 | full | SelectableText and SelectionArea components, .selectable on four leaves | — | todo |
| M4a | T033–T038 | full | Selectable leaves in the transcript, parity and budget (not user-visible) | — | todo |
| M4b | T039–T043 | full | Selection, edge scroll, menu and copy keys in the transcript; user guide | — | todo |
| M5 | T044–T049 | full | Copy controls, docs, visual pass | — | todo |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Per-block or cross-block selection? | Cross-block: one range across entries in reading order | agent-resolved | issue #1550 "Expected" (across lines and blocks); spec.md#assumptions |
| D2 | spec | Ctrl+C: copy or interrupt (#1403)? | Transcript focused + non-empty selection: copy, never interrupt; no selection: #941 FR-012a unchanged; Cmd+C / Ctrl+Shift+C never interrupt | agent-resolved | spec.md FR-008, FR-009; specs/941-structured-prompt-input/spec.md FR-012a |
| D3 | spec | Which values get copy controls? | Tool card identifying value (header), full input, full result, diff file path; code blocks keep #940's control; no whole-message control | agent-resolved | spec.md FR-015–FR-018 |
| D4 | spec | Platform behaviour? | Identical on Linux/macOS/Windows; only Cmd vs Ctrl differs; LF line breaks everywhere | agent-resolved | spec.md Terms (Copy keys), Edge Cases (Cross-platform) |
| D5 | clarify | FR-011: copy rendered markdown as visible text or source? | Visible rendered text as drawn; links copy shown text; no emphasis/heading/fence markup | agent-resolved | spec.md FR-010 ("as shown"), SC-001; user guide code-block copy "no fences, no tag" (#940 FR-011) |
| D6 | clarify | FR-012: copy to clipboard on drag release, like the terminal view? | No: copy only on request (copy key, context menu Copy, copy control); release never writes the clipboard | decided by user | user answer 2026-10-28 ("No, copy only on request"); spec.md FR-012 |

## Tasks-unit notes

- speckit-analyze (2026-10-28): 0 CRITICAL, 0 HIGH. Fixed C1 (stability test moved before the Selection impl), C2 (T018/T019 test and wiring split), C3 (state, SelectableText and entry-view tasks split), D1, U1 (spec FR-011 states the tab/line table rule of research R3), U2 (contrast gate also checks the highlight differs from each surface), U4 (spec Terms: image caption selectable), I2/I3 (plan Delivery *Satisfies* and file tree aligned with tasks). Left: U3, U5, I1, I4 (LOW).
- M2 has 11 tasks, kept whole as the plan's Delivery cuts it: its keys and copy tasks need the state tasks and have no deliverable of their own a user can observe.
- Checklist `requirements.md`: the one unchecked item ("No [NEEDS CLARIFICATION] markers remain") confirmed satisfied by the tasks reviewer and ticked; all items closed.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 14b551961ac69d5a737bde175d79033eaadcc4f0:828bc8e24442d9c9685869c35fe69134934bd233 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | 80048701cd16dc386ae196e8da6d4dd7a429becb:828bc8e24442d9c9685869c35fe69134934bd233 | CLEAN (2 MINOR fixed: rewrap, US2 scenario 6 for FR-019a) |
| Plan | 1 | 94244e6c1589e20bfeeeb532e3c9cd56beb33cee:783f9af1dc50a9469289c5e39f0e25d6a0ad62d2 | CHANGES: 2 MAJOR, 3 MINOR (all fixed: F1 RowPos row key, F2 M4 split into M4a/M4b, F3 retain_shown in shell/panes.rs::sync, F4 `focused` input to transcript_key, F5 EditDiff::path) |
| Plan | 2 | 3974e3a581dc83999537a082f2459b51cb3471c0:6e22abe4c36f2ceabc272035bb95f51647ae92db | CHANGES: 1 MAJOR, 2 MINOR (all fixed: TranscriptView defined, retain_shown session/Primary mapping, prompt rows inside bounds clear) |
| Plan | 3 | 57e895f188c87b43d22698f2f2c2c221b9539a4e:99112b227c7c36a8e9e407663569d56e06d8c413 | CLEAN (1 MINOR fixed, prose only: sessions from Workspace::session_ids_of_project) |
| Tasks | 1 | 91e2a3d9cf7b87ce83bcc7e5e0c7510556892560:12b46edb6afe9910d2dcd96175cf67c05102539a | CLEAN (3 MINOR fixed, prose only: checklist ticked, CopySelection stub noted in T016, M4b Verify records Part B 1–5 visual pass) |
| Review A M1 (code-review high) | 1 | no snapshot (HEAD 350eb1157) | 10 findings: 6 fixed in 75cbb5030 (word_at inside its line, snap via GraphemeCursor, full-width punctuation, unsnapped test offset, list marker arms merged, shared work_str), 4 declined (below) |
| Review B M1 | 1 | 55ede43b68b9972806d53205fc373e5003804172:75cbb5030640e4bb0c886ec779d3a90794010ddf | CLEAN (1 MINOR fixed: tdd/cycle-log.md written from recovered red evidence) |

## Declined review findings

- Review A M1 #1 (Selection does not snap; TextPos fields public): the contract fixes `pub` fields and `start(at, granularity, expand)` without the text; snapping happens where the text is known, `TextPos::in_text` (doc now says so). The hit mapper (M3) builds positions with it.
- Review A M1 #3 (`u16` path segments overflow past 65,535 lines): data-model.md fixes `Vec<u16>`; a code block or tool output is one unit (R2), so lines are not path segments; diffs are capped by `review::limits`.
- Review A M1 #5 (`in_row` re-computes `bounds()`): MINOR, a handful of small clones per row; revisit in M4a's budget test if it shows.
- Review A M1 #8 (S-5 test holds by construction): it checks that entry ids by key stay put while every recorded fixture streams (a transcript property, not the selection's) and that the selected text of a growing entry is unchanged; `copy_text` stability needs `units()`, which is M2's.
| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
