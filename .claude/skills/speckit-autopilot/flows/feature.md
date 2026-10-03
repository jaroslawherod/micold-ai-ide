# Flow: feature (full)

New behaviour. The whole Spec Kit flow, shipped as reviewed milestones.

| Unit | Task files, in order | `model` | Ends with |
|---|---|---|---|
| Spec | [spec](../tasks/spec.md), [review](../tasks/review.md), [review-rounds](../tasks/review-rounds.md) | omit | reviewed spec, committed; no PR |
| Clarify | [clarify](../tasks/clarify.md) | omit; `"sonnet"` when continuing a handover | `CLEAN`; no PR. Returned without it: dispatch another |
| Plan | [plan](../tasks/plan.md), review, review-rounds | omit | reviewed plan, committed; no PR |
| Tasks | [tasks](../tasks/tasks.md), [milestones](../tasks/milestones.md), [milestone-format](../tasks/milestone-format.md), review, review-rounds, [gate](../tasks/gate.md), [pr](../tasks/pr.md) | omit | the **design PR** |
| Milestone K, one at a time | [implement](../tasks/implement.md), [verify](../tasks/verify.md), review, review-rounds, gate, pr | **Tier** `full`: omit. `light`, `docs`: `"sonnet"` | one PR per milestone |
| Close | [close](../tasks/close.md), review, review-rounds, gate, pr | omit | the close PR with the ledger finished, or `NEW MILESTONES` (run them, then a new close unit). It reads milestones and milestone-format only when it finds unbuilt behaviour |

Omit means the session model. Continued with a red CI log, a unit also reads
[red-ci](../tasks/red-ci.md).

- **Keeps everything:** a fresh reviewer on spec, plan and tasks; reviews A (`high`) and B on
  every milestone; the scoped gate between rounds and the full gate before each push;
  `speckit-tdd-verify` at the close.
- **Effort labels** do not change this flow: each milestone's **Tier** picks its model.
- **Ledger:** `specs/<issue>-<slug>/autopilot.md`, **Kind** `feature`. The issue number is the
  spec's number.
- **Ends with:** the close PR merged, then the handoff. A unit never returns `NEXT:` here; a spec
  unit that finds the work is not new behaviour escalates (category 1).
- A ledger from an older run has **Docs-only** instead of **Tier**: `yes` is `docs`, `no` is `full`.
