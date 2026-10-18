# Autopilot ledger — #802 soft-limit-confirm

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #802: Make one session per worktree a soft limit: confirm before creating a parallel session. labels: enhancement, flow:feature, effort:low
- **Kind**: feature
- **Effort**: default
- **Issue**: #802
- **Worktree branch**: feat/802_make-one-session-per-worktree-a-soft-limit-confirm
- **Started**: 2026-10-18
- **Phase**: milestone M1
- **Next step**: wait for M1 PR CI and merge, then milestone M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1031 | Design | merged | 03e676bfe979097b0f524af2259b71f56059408d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | Person start on an occupied location asks to confirm | #PR | open |
| M2 | T011–T014 | light | Agent rule documented in tool descriptions and guide | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks | 1 | 471eeafeb7ec5564f4a5f78ffedb530b70558563:5c04aae81ae8fdebe67666ab2815c46dd9d39475 | CLEAN (analyze MEDIUMs fixed first; spec gained 2 edge cases) |
| Plan | 1 | 4e8358f18d72867574a948bd7aa91fab461bf1c0:81b665a7eb821c4ee43acafbca13247dc284ad2e | CHANGES: 2 MAJOR (F1 stale dialog text, F2 automatic start origin) fixed in plan/research/data-model/contracts; MINORs F3-F5 fixed; prose-only plan changes, no round 2 |
| Code B M1 | 1 | 55678ee3a850a990aec7eb5a59c7288b89cb81a5:155043e438a3e9c8712d12d5498171e842989d40 | CLEAN |
| Code A M1 | 1 | daeb028058883f37750a2bed804c6bd6ba5ab39a:046bf60ba73da5ef82989b82b70e0ea6b57a9ad4 | code-review high: fixed F1 (project stored in PendingOp, refusal reopens only for the same project), reducer-from-shell rule (AdditionalOpened message); rest declined/MINOR |
| Spec | 1 | 19c41bceb94cb337a85c1abb9c7f545c8e17ea66:1800df279b79d4dcbfab1a413ced53e497f5fa90 | CHANGES: 2 MAJOR (F1 routes table, F2 FR-022 wording) fixed; MINORs F3-F5 fixed; claims about behaviour re-checked by me against code; prose-only per rules, no round 2 |

## Declined review findings

- A-F3 (open_in guard dropped): needed so a refusal can open the dialog before the catalog lists the occupant; stale case indistinguishable.
- A-F2 (late refusal closes other dialogs), A-F4 (double press), A-F5 (UUID detail contract), A-F6, A-F7, A-F8: MINOR/by design (plan: typed fields from 776's refusal; automatic stays a notice by FR-004); A-F9 (pub module) needed by tests in tests/.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
