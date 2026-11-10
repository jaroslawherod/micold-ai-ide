# Autopilot ledger — #1718 sidebar-stable-order

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1718 "Sidebar: stop grouping worktrees/sessions by state — rows jump between groups and are hard to track"; labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1718
- **Worktree branch**: feat/1718_sidebar-stop-grouping-worktrees-sessions-by-state
- **Started**: 2026-11-06
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1750 | Design PR | merged | 01bc227bc6684dbbcc4dc964b53d0290eb704a03 |
| #1782 | Amend design (tint, cleanup candidate) | merged | 9d65146d1f8bb4dcb76f62c50ce241d65b91b04e |
| #1791 | M2 | merged | 6d3b88f2d4787e6f3c51c97f61762bbca4f979c5 |
| #1800 | Close: tdd-verify fixes, spec Closed | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Flat stable sidebar list, no group residue (US1+US3 are one change) | #1758 | merged 8b1039d2c123f8e414806ecc75ab0cd6ae5a7c3f |
| M2 | T015–T030 | full | Five-state row indicator (StateIcon on CardState) plus row tints | #1791 | merged 6d3b88f2d4787e6f3c51c97f61762bbca4f979c5 |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | plan | Realise the cleanup candidate how? | No sixth state: Done is the candidate; louder tint for Needs action/attention, quieter for Done; closed-unmerged stays Idle (follow-up) | user | research R7 |

## Review rounds

- M1 A and B round 1 snapshot f41ea9a..:4bc70c9.. both CLEAN (A: 2 MINOR, left: run_group_state kept by T007, reveal_ready &mut; B: Verify 64 passed). Gate 1 red on one stale test (key count), fixed

- plan R1 snapshot 9c7a789..:5d02cca.. CHANGES (2 MAJOR fixed); R2 snapshot cdf4399..:a38b989.. CHANGES (2 MAJOR: contradictory Complexity Tracking, incomplete file list; fixed, prose only)

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c5e140ccc4b8e1b8c17dddadfae8ef5d5f94de53:c695061be43a32a00e41c482221be91d470a34bb | CHANGES: 3 MAJOR (persistence story, open markers, bad FR-017 cite); all fixed; no round 2, fixes remove claims |

- M2 A snapshot cc6798975ec5:e085a674f061 CLEAN (3 MINOR); M2 B snapshot d12228da7462:233ff1237cf7 CLEAN (1 MINOR = A's F3, accepted: selected row keeps tint-ink glyph by design). Scoped gate: ui_glyph_literals red on literals in a state_icon test, fixed (Icon variants). Visual pass: showcase only, both themes, specs/1718-sidebar-stable-order/visual-pass/ (real client needs a live daemon, not driven)

- close: tdd-verify FAIL (M1 TEST_AFTER, row wiring untested, 4 tautologies); fixed in tests, findings 8-13 accepted; review snapshot b899b3b..:16d873e.. CLEAN (1 MINOR fixed). Full gate green on the 3rd run (1st: clippy type_complexity; 2nd: cfg(test) seams broke anatomy_call_sites, replaced by pub(in crate::ui) fields; daemon structured_resume flaked once, green on rerun). converge skipped: every milestone has a CLEAN review B.

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1750 | Design PR | merged | 01bc227bc6684dbbcc4dc964b53d0290eb704a03 |

## Handover

None.

## Open escalation

Close commit is local (full gate green, review CLEAN, ledger done) but not pushed: `git push --force-with-lease` was denied by the permission classifier (the remote branch holds pre-squash commits, so a plain push is rejected). Needs the user to allow the force-with-lease push (or push it), then `gh pr create` per pr.md (title `docs(1718): close the spec (#1718)`, body ends `Refs #1718`), then record the PR number in the ledger.

## Follow-ups not done

Closed-unmerged PR as a cleanup candidate: own issue, needs a data-safety design (spec Out of Scope, research R7).

- tasks R1 snapshot 09483c9..:83987a3.. (speckit-analyze: 0 CRITICAL/HIGH; I1,I2,C1,F1 fixed; no plan inconsistency found) -> CLEAN, 3 MINOR path fixes applied. Plan.md line 81 cites `tests/layout_snapshot.rs` without the crate prefix (tasks use the full path).
- amend-design part 2 round 1 snapshot 56bba68..:eaa6b00.. CHANGES (3 MAJOR: tooltip wording source, [P] file overlap, checklist/tdd; fixed, spec-artifact edits only, no behaviour claim) -> no round 2, prose/task-format fixes
