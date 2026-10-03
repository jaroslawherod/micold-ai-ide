# Task: cut tasks and milestones, open the design PR (tasks unit)

When: the plan unit committed the plan and it passed its review. Read the plan with
`brief.py section`, not whole.

1. `speckit-tasks`, then cut milestones by [milestones.md](milestones.md) and write them in the
   format of [milestone-format.md](milestone-format.md). Add them to the ledger, with their
   **Tier**.
2. `speckit-analyze`, and fix what it finds. A fresh reviewer then checks the result:
   [review.md](review.md) with [../rubrics/tasks.md](../rubrics/tasks.md). A finding that the plan
   itself is wrong: fix the plan, and say so in the ledger.
3. **Close checklists.** `speckit-implement` stops on unchecked checklist items, so resolve them
   here. For each unchecked item, either:
   - a reviewer subagent confirms the artifacts satisfy it, and you tick it; or
   - you fix the spec or plan until they do; or
   - it needs a decision, and you escalate.
4. Local gate: [gate.md](gate.md), *Specs-only*.
5. Open the **design PR** per [pr.md](pr.md), with the spec, its clarifications, plan, research,
   contracts and tasks.

Hands on: return `DONE` with the PR number. Next unit, after the merge: milestone M1.
