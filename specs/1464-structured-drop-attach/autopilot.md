# Autopilot ledger — #1464 structured-drop-attach

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1464 (issue #1464: Structured view: drag-and-drop files into the composer and an "Add attachment" button); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1464
- **Worktree branch**: feat/1464_structured-view-drag-and-drop-files-into-the
- **Started**: 2026-10-27
- **Phase**: plan
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | … | #… | pending / in-progress / in-review / ci / merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

- Plan review round limit (3 rounds, MAJOR in round 3: /shortcuts needs DraftCommand::Shortcuts and a daemon arm): accept the amended plan D7 without re-review. decided by user.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 7b24c45ecc84cc6e0d37f9bc349d6d0d966a47f8:09ee4cb188b5b76f59512aa8e506f88ca2d7e7b7 | CHANGES: 2 MAJOR; fixed (cross-platform FR-004/SC-003, FR-013 reworded, MINORs) |
| Spec | 2 | 6c8d0ab0996c5e611c97981c32a28cc98d1c2972:bdcd5311f56230d1579f97ae160b184d9a5a0b96 | CLEAN; 3 MINOR (FR order, SC-006 wording fixed, prose only; hover-cue marker left for clarify) |
| Plan | 1 | 06c9c86a8c1e230677faf25909ca929a1e109ada:bd622f47c7ae0fd825129ea73a4def1463388ab4 | CHANGES: 3 MAJOR (sheet entry point, chord Ctrl+Shift+U, material crate); fixed |
| Plan | 2 | 501be13e873638cb758696fdb28a1dd58ccc655d:47c2152bd2e64ac1b79b75ab1ebc9df9d55ccbef | CHANGES: 1 MAJOR (/shortcuts needs core change); fixed |
| Plan | 3 | 9f004f84f170c66c25c63dbacdd40c8643a97d4b:875a8a2c104e69d83fcbdf940e6cc0e2083b92b7 | CHANGES: 1 MAJOR (DraftCommand::Shortcuts + daemon arm missing from plan); plan amended, NOT re-reviewed (round limit) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
