# Phase 3b: tasks unit

The plan unit committed the plan; it passed its review. Read it with `brief.py section`, not whole.

1. `speckit-tasks`, then cut milestones by [../references/milestones.md](../references/milestones.md).
   Every milestone ships a **deliverable**: something observable on `main`. "Setup" or
   "Foundational" is never a milestone on its own. Add the milestones to the ledger, with their **Tier** field.
2. `speckit-analyze`, and fix what it finds. A fresh reviewer checks against the tasks and milestone
   rubric. A finding that the plan itself is wrong: fix the plan, and say so in the ledger.
3. **Close checklists.** `speckit-implement` stops on unchecked checklist items, so resolve them
   here. For each unchecked item, either:
   - a reviewer subagent confirms the artifacts satisfy it, and you tick it; or
   - you fix the spec or plan until they do; or
   - it needs a decision, and you escalate.
4. Open the **design PR** (`docs(NNN): specify and plan <feature>`) with the spec, its clarifications,
   plan, research, contracts and tasks. Local gate per
   [../references/pr-and-merge.md](../references/pr-and-merge.md) §2, *Specs-only PRs*.
