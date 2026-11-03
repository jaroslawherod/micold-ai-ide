# Autopilot ledger — #1664 <slug> [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1664 (issue #1664: Structured view: raw events are shown inline in the transcript; present them as collapsed summaries); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1664
- **Worktree branch**: feat/1664_structured-view-raw-events-are-shown-inline-in-the
- **Started**: 2026-11-03
- **Phase**: milestone M2 (implementing)
- **Next step**: M2 verify, scoped gate + review A, review B, visual pass, full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1667 | Design | merged | 64ed549a8651dba5cfa433fc833d4d3540904298 |
| #1668 | M1 collapsed summary lines (PR head branch feat/1664_collapsed-raw-events-m1; local worktree branch tracks it) | merged | 4198d80c53a5109eee89f11b0b4cfa8356ad9e36 |
| #… | M2 toggle (PR head branch feat/1664_collapsed-raw-events-m2; push with explicit refspec) | in progress | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T008 | full | Collapsed summary lines | #1668 | merged |
| M2 | T009–T025 | full | Show raw events toggle, persisted, with hidden notice (17 tasks, no shippable split) | #… (head feat/1664_collapsed-raw-events-m2) | in progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 2479fb2a15da66eb0accabf88bb3df2733cb7cd1:606c5bf85706c584f474b0ce2ff8fd50666a5ca5 | CHANGES: 2 MAJOR (fixed: facts match entry fields; error visibility raised as FR-013 marker); MINORs fixed, prose only |
| Plan | 1 | 4000e8699dd9e09f40708486875fbad22d9dbf29:a5e229e469e371ec6969d16d430bbeaf68f9c9b6 | CHANGES: 3 MAJOR (selection units, Settings sites, stale selection), fixed in plan.md and ui-contract.md |
| Plan | 2 | f10850fee75acb8a1900cc086b032d92e3ef5c23:19c23230602f2f7baa0900ae3630e275d1a4d8d3 | CLEAN (1 MINOR into_settings wording, fixed, prose only) |
| Tasks | 1 | 7f490e14daa4fd3665d83223e6898ac8e0ecb4b8:dacfed8994dc737a731872b83cf8f1a69ec11bc9 | CHANGES: 1 MAJOR ([P] tasks sharing a file), 3 MINOR; fixed, prose only; analyze F6 fixed by merging notice into M2 |
| Spec | 2 | a7d89dffe96efd9eed2b34554c01e3d784f407d0:2305bb236cec7443b911cbc0c9b526ac9dd8870a | CLEAN (2 MINOR: FR order fixed; error-footer scenario deferred to FR-013 resolution in clarify) |
| M1 A+B | 1 | 29927781b4b54b1a221637c30fcac1e03b3efb4d:64ed549a8651dba5cfa433fc833d4d3540904298 | A: CHANGES 1 MAJOR (footer details tooltip dropped; now an open-detail unit [4]), fixed; B: CLEAN; MINOR stale doc fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. FR-013 answered by the user (error footers/orphans stay visible); applied to spec.md.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
