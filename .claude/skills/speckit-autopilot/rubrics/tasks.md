# Rubric: tasks and milestones (after `speckit-analyze`)

- `speckit-analyze` reports no CRITICAL or HIGH findings.
- Every task has an ID, a file path, and a story label where relevant. `[P]` tasks touch disjoint
  files.
- Test tasks come before the implementation tasks they cover (Principle I).
- The `## Milestones` section follows [../tasks/milestones.md](../tasks/milestones.md):
  - every task is in exactly one milestone
  - M1 includes the P1 story
  - no milestone is Setup or Foundational alone
  - every deliverable is observable and has a **Verify** step
  - dependencies point backwards only
  - every user-guide task is in the milestone that ships its behaviour
- Every item in `checklists/*.md` is truly satisfied (and ticked) or reported as a finding.
