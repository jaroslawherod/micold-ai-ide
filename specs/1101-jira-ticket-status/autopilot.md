# Autopilot ledger — #1101 jira-ticket-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Show a Jira ticket's current status (issue #1101); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1101
- **Worktree branch**: feat/1101_show-a-jira-ticket-s-current-status
- **Started**: 2026-10-20
- **Phase**: implement
- **Next step**: M3 implement (T028-T030)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1152 | Design PR | merged | 537bf0d5193e751dbd41b377a70dd6488db31ba0 |
| #1155 | M1 | merged | 473538658f1b4835f0fba0eddf67d95bfe608269 |
| #1190 | M2 | merged | 856f61a1a505e97ddc45dff92c9e1e66eb6e0338 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Status chip in the issue picker | #1155 | merged |
| M2 | T014–T027 | full | Current status on the sidebar | #1190 | merged |
| M3 | T028–T030 | full | Status on Overview cards | | implementing |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| #1152 | Design PR | open | |
| Spec | 1 | 952ec53aa3d23346f6fed8f12d836bcf43d073d0:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CHANGES: 4 MAJOR |
| Spec | 2 | 2791b5d842f73b2d98560fad5de1801a962e828a:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CHANGES: 1 MAJOR (not-found contradiction), 2 MINOR; fixed, checked by author (wording reconciliation only) |
| Plan | 1 | 87105a5c80cf11e780ace91fd3629036eddc60f5:f3cacdb0b33caca2142ce3ac8f3e8ef396c86102 | CHANGES: 3 MAJOR (400 on unknown key, project-open hook, epoch), 3 MINOR; all fixed in plan/research/data-model/contracts (design decisions added: bisection on 400, epoch, daemon_sync hook) |
| Plan | 2 | 1efaa84c3741213ab69f9dbf3dfb2877567fc01f:f3cacdb0b33caca2142ce3ac8f3e8ef396c86102 | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 | 6807400cb49973f201fe92f29c7f7bf9c08cd521:1a784129401f5b853ac8d121376a401053af3260 | CHANGES: 2 MAJOR (shared-file [P], accessible_text test order), 3 MINOR; fixed, checked by author (task wording/ordering only); analyze HIGH C1, C2, I1 fixed |
| M1 review A | 1 | aad5399d8:f11ffb94 | 10 findings; 1 fixed (chip aligned to first line), rest declined/MINOR |
| M1 review B | 1 | aad5399d8:f11ffb94 | CLEAN (2 MINOR) |
| M2 review A | 1 | b90ff0c14cecbbde6895835d88fb8fe279889418:d0923323fa64e10532127c327f54ca41bc28e4c8 | 10 findings; fixed: fmt, per-message cost (dormant skip), connection change drops statuses and re-reads (also covers Jira enabled after open), clock doc; rest declined (below) |
| M2 review B | 1 | 421a6d147b3dcd974dc351c312ad4804fe5e40da:a6da24e84c8e4b28b4081d93e49ac37606053282 | CHANGES: 1 MAJOR (red not recorded; fixed by mutation runs, logged in cycle-log), 1 MINOR (task paths; fixed). Verify green |
| M2 review B | 2 | a1a93f4838d3a7f483a464ee2fe5dc9a19f32d3a:a162785b7ae08f9218cfd80c2aba2e718d8cac52 | CHANGES: 1 MAJOR (no test of the Settings disclosure; fixed with a painted-text test, red by mutation), 2 MINOR (cycle-log notes added). Verify could not run in the reviewer behind the gate lock; the full gate runs the tests |

## Declined review findings

- M3 review B F1 (T029 names `features/overview.rs`, chip is filled in `ui/overview.rs::card_status` where the cards are built): kept, noted here.
- M3 review A F1 (no test of expanded card chip): declined, MINOR.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | stale flag hard-coded false in picker row | stale and the board belong to M2 (T014+) |
| M2 | A | new worktree made after open gets no chip until focus | FR-005: reads start on project open and focus only |
| M2 | A | moved/renamed ticket (returned key differs) loses its chip | edge case; a ticket not returned retires (spec); no stale marker needed |
| M2 | A | a 400 on a single key is read as not-found | contracts/jira-status-source.md §3 specifies exactly this |
| M2 | A | chunking in read_statuses and in JiraCloud::statuses | the adapter's validation guards the bisection; the core chunk keeps the fake and the real source equal; MINOR |
| M2 | A | SystemTime clock can step backwards | at worst one focus refresh is delayed; Instant is not available across the lib/bin split; doc corrected |
| M2 | A | picker prefers the board's older status over the listing's | FR-010: one read path for every surface |
| M2 | A | in_flight could stick if the task is dropped | the task maps a join error to Err, so apply always runs |
| M1 | A | SHORT_NAME char heuristic, double tooltip, Tag line height, contrast, status not searchable | MINOR/taste; status excluded from matching is FR-011; tooltip is FR-013 |
| M1 | B | no tdd/cycle-log.md | tests were written with the code; no red-phase log exists |
| M1 | B | fake_jira example unchanged, no In Progress ticket | statuses reach the example through the shared fixture (To Do, In Review=indeterminate, Done, none) |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
