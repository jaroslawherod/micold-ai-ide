# Autopilot ledger — #1804 staged-review

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot: staged review (facts by code, plan by strong model, probes by cheap workers), issue #1804 (see issue body); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1804
- **Worktree branch**: feat/1804_autopilot-staged-review-facts-by-code-plan-by
- **Started**: 2026-11-10
- **Phase**: 4-milestone M2
- **Next step**: M2 PR #1824 open, wait CI and merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1809 | Design | merged | e82cc8df6bddd76f87e2d0786098f9d4ceaf8b28 |
| #1820 | M1 | merged | 547ce702ba18af1020797f46d9adcf45462d474e |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 (incl. T007a) | full | review-facts.py writes the facts file; tests in autopilot.test.sh | #1820 | merged |
| M2 | T011–T015 | full | three skill files describe the staged flow, fallback, shadow | #1824 | open |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 |  |  | e7c0eefe9082f5ab845344855d1e3a448d0f1837:653769a81adcc13e2f9695de8f9ade892d15cf58 | CHANGES: 4 MAJOR (fixed) |
| Spec | 2 |  |  | 5cde486ee9e7e9df34b7a01fbc085ba42eb9247d:3682d99c9b3cdda2488e886dad070dbbc3470657 | CLEAN (1 MINOR fixed) |
| Plan | 1 |  |  | 1a62212de3d302e24a02e30ded3cafd5b57b808e:2cb1d380997d85aa1003d496bb6d0592f93454e8 | CLEAN (3 MINOR fixed, prose only) |
| Tasks | 1 |  |  | 556972415e82386de902014ef228fc9a6f47699e:69e09ead864f03119471a114f103cb65c96a9ce3 | CLEAN (2 MINOR, noted) |
| Code M1 A | 1 |  |  | 7a69cc9c5b80df71c7dabc6a2ece54b92a6f37da:27245460b828b59289090067c3903b7231e2f3f0 | 10 findings, none BLOCKER/MAJOR by rubric; fixed: char literals, deletions, git-config prefixes, lcov errors, callers budget (tests added); left: raw/multiline strings, attribute lines, bare-name callers (approximate by contract) |
| Code M1 B | 1 |  |  | same | CLEAN (2 MINOR: char literals fixed; no tdd cycle-log in this feature) |
| Code M2 A | 1 |  |  | b4cf939544170bab3a03969ffdb29ac69e6d0736:547ce702ba18af1020797f46d9adcf45462d474e | 3 MINOR: --base-snapshot value named (fixed); NEEDS-MORE token and tighter greps left (wording contract) |
| Code M2 B | 1 |  |  | same | CLEAN |
| Code M2 A+B | 2 | post-rebase full diff | c0809662d0414b3e40a6ff4fcc6dac7d96b05a82 | 93d3ae4a366406afc1e6949c5dcc517bb7962b8d:c0809662d0414b3e40a6ff4fcc6dac7d96b05a82 | CLEAN (1 MINOR left: dense lines at ceiling) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec 1 | F2 implementation names in FRs | Feature changes autopilot's own tooling files; naming them is scope. SC-004 reworded. |

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

- US3/SC-005 shadow run and decision (T016-T017) are outside the milestones; tracked by follow-up issue #1808, after M2 merges and three code-diff reviews have run.
- Tasks unit: speckit-analyze had no CRITICAL/HIGH; MEDIUMs fixed in tasks.md and contracts.
