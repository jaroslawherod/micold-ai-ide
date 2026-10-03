# Task: write the `## Milestones` section, with tiers

When: milestones are cut ([milestones.md](milestones.md)).

Append the section to `tasks.md` after "Implementation Strategy":

```markdown
## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — <short name> 🎯 MVP

- **Tasks**: T001–T018
- **Deliverable**: <one sentence: what a user or developer can observe on main after merge>
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001, FR-002, FR-004
- **Verify**: <the command, test name or quickstart section a reviewer runs to see the deliverable>
- **Depends on**: —
- **Tier**: full

### M2 — <short name>

- **Tasks**: T019–T027
- **Deliverable**: …
- **Satisfies**: US2 acceptance scenarios 1–2; FR-003
- **Verify**: …
- **Depends on**: M1
- **Tier**: light
```

**Tier** picks the model that implements the milestone. Cheaper tiers cost less per call; the
session model is for work that needs judgment.

- `docs`: every task's file paths are docs or spec files (no code, tests, scripts, CI or build
  config).
- `light`: every task follows a pattern that already exists in the repo, and plan.md names where.
  At most about 8 tasks. None adds or changes a wire protocol, persistence format, concurrency,
  process or sandbox boundary, security check, or a public API between crates.
- `full`: anything else. M1 is always `full`.

When unsure, pick the higher tier.

Next: the task that sent you here ([tasks.md](tasks.md) or [close.md](close.md)).
