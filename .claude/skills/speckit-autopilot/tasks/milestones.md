# Task: cut milestones

When: `speckit-tasks` wrote tasks.md, or the close unit found unbuilt behaviour.

A milestone is the unit that reaches `main`. When its PR merges, `main` is green and something new
works that someone can observe. That is the milestone's **deliverable**.

`speckit-tasks` groups tasks into phases (Setup, Foundational, one per user story, Polish). Phases
are the order of work. Milestones are what ships. Map phases to milestones by these rules.

1. **M1 = Setup + Foundational + the P1 user story (the 🎯 MVP).** Setup or Foundational alone is
   never a milestone: nothing to observe.
2. **Each further user story is its own milestone**, in priority order.
3. **Split a story** when it has more than about 10 tasks or its diff would exceed about 800 changed
   lines. Split along an acceptance scenario, so each half has a deliverable. If no such split
   exists, keep the story whole and note why in the ledger.
4. **The last milestone is Polish:** cleanup, cross-cutting docs, convergence tasks. Its deliverable
   is usually "quickstart §B passes" or "the architecture doc describes X". **A Polish that changes
   no code** (quickstart results, doc wording, task ticks) is not a milestone: leave its tasks to
   the close unit, which does them in the close PR.
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
8. **A bugfix is a single milestone.**

Task lists need not be contiguous. When rule 4 pulls a user-guide task forward, list it
(`T001–T020, T030`) and pass the same list to `speckit-implement`.

**Appended later.** When the close unit's `speckit-converge` appends tasks, cut them into a new
milestone (`M<n+1>`) under the same rules, and add it to the section and the ledger.

Next: write them down per [milestone-format.md](milestone-format.md).
