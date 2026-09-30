# Phase 3: design unit

1. `speckit-plan`. A fresh reviewer checks it against the plan rubric.
2. `speckit-tasks`, then cut milestones by [../references/milestones.md](../references/milestones.md).
   Every milestone ships a **deliverable**: something observable on `main`. "Setup" or
   "Foundational" is never a milestone on its own. Add the milestones to the ledger, with their **Tier** field.
3. `speckit-analyze`, and fix what it finds. A fresh reviewer checks against the tasks and milestone
   rubric.
4. **Close checklists.** `speckit-implement` stops on unchecked checklist items, so resolve them
   here. For each unchecked item, either:
   - a reviewer subagent confirms the artifacts satisfy it, and you tick it; or
   - you fix the spec or plan until they do; or
   - it needs a decision, and you escalate.
5. Open **PR 2** (`docs(NNN): clarify, plan and cut milestones for <feature>`) with the clarified
   spec, plan, research, contracts and tasks. Docs-only: the local gate is `mise run test-scripts`.
