# Cutting milestones

A milestone is the unit that reaches `main`. When its PR merges, `main` is green and something new
works that someone can observe. That is the milestone's **deliverable**.

`speckit-tasks` groups tasks into phases (Setup, Foundational, one per user story, Polish). Phases
are the order of work. Milestones are what ships. Map phases to milestones by these rules.

## Rules

1. **M1 = Setup + Foundational + the P1 user story (the 🎯 MVP).** Setup or Foundational alone is
   never a milestone: nothing to observe.
2. **Each further user story is its own milestone**, in priority order.
3. **Split a story** when it has more than about 15 tasks or its diff would exceed about 800 changed
   lines. Split along an acceptance scenario, so each half has a deliverable. If no such split
   exists, keep the story whole and note why in the ledger.
4. **The last milestone is Polish:** cleanup, cross-cutting docs, convergence tasks. Its deliverable
   is usually "quickstart §B passes" or "the architecture doc describes X".
   **The user guide is not Polish work.** CI's user-guide gate (`scripts/check-user-guide-updated.sh`,
   Constitution VII) needs the guide updated in the same `feat` PR, so put each story's user-guide
   task in that story's milestone. Use the `docs-not-needed` label only when no guide update is
   needed.
5. **Dependencies point backwards only.** A milestone may rely on earlier ones, never later ones.
6. **No half-wired UI on `main`.** If a story's UI cannot finish inside its milestone, ship the core
   behaviour and its tests, and keep the UI unreachable until the milestone that completes it. Name
   that milestone.
7. **Tests travel with the code.** Every task a milestone implements has its test task in the same
   milestone (Constitution I).
8. **A bug fix is a single milestone** unless Phase 0 promoted it to a feature.

## The `## Milestones` section

Append it to `tasks.md` after "Implementation Strategy":

```markdown
## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — <short name> 🎯 MVP

- **Tasks**: T001–T018
- **Deliverable**: <one sentence: what a user or developer can observe on main after merge>
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001, FR-002, FR-004
- **Verify**: <the command, test name or quickstart section a reviewer runs to see the deliverable>
- **Depends on**: —

### M2 — <short name>

- **Tasks**: T019–T027
- **Deliverable**: …
- **Satisfies**: US2 acceptance scenarios 1–2; FR-003
- **Verify**: …
- **Depends on**: M1
```

Task lists need not be contiguous. When rule 4 pulls a user-guide task forward, list it
(`T001–T020, T030`) and pass the same list to `speckit-implement`.

## Handing a milestone to `speckit-implement`

`speckit-implement` reads its argument as guidance or a task filter. Pass:

```
Milestone M2 only: tasks T019–T027. Do not start any other task. Stop when these are done.
```

Afterwards, check that every task in the range is ticked and none outside it is. An open task in
the range means the milestone is not done.

## Milestones appended later

When Phase 5's `speckit-converge` appends tasks, cut them into a new milestone (`M<n+1>`) under the
same rules. Add it to the section and the ledger, then return to Phase 4.
