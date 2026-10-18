# Autopilot ledger — #968 quick-switcher

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #968 "Quick-switcher to jump directly to a project, worktree or session" (see
  spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #968
- **Worktree branch**: claude/project-thread-ftdy5c
- **Started**: 2026-10-18
- **Phase**: M1 verify
- **Next step**: M1 — verify (scoped gate, review A, review B, visual pass, full gate), PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1036 | design (spec, plan, tasks) | merged | c687792e717f3312881d09e87ed8e6f1ea374022 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020 | full | Chord opens the switcher over the active project's locations; filter, choose = row click, close | — | in progress |
| M2 | T021–T026 | full | Sessions listed under their locations, found by name, kept by a location match | — | planned |
| M3 | T027–T031 | light | Other known projects listed (unavailable badged, unselectable); complete user-guide bullet | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which chord opens the quick-switcher? | Ctrl/Cmd+Shift+P | agent-resolved | spec.md#assumptions; docs/user-guide/terminal-panes.md#keyboard-shortcuts |
| D2 | clarify | Are worktrees hidden by the tag filter listed? | No; #803 Location order (after the filter) | agent-resolved | specs/803-keyboard-navigation/spec.md#terms |
| D3 | clarify | Unavailable projects: listed despite #803 Project order excluding them? | Listed in the switcher's order, badged, not selectable | agent-resolved | spec.md US3 scenario 3, FR-006, FR-008 |
| D4 | clarify | List other projects' locations and sessions? | No, active project only (option A) | default, pending user confirmation | may flip to B (all projects): then US3-5, FR-003, FR-004 change; spec.md#clarifications |
| D5 | clarify | Rank of a session kept by its location's match; highlight past unavailable projects | After its location, location's rank; first selectable entry | agent-resolved | spec.md US2-3, FR-007 |
| D6 | tasks | Split US1 (20 tasks) further? | No: chord, dialog, list and choose are one observable loop (milestones rules 1, 6); sessions and projects are M2, M3 | agent-resolved | tasks.md M1 Note |
| D7 | tasks | speckit-analyze C1/C2: Overview-close and empty-state choice sat in glue | plan D5, data-model, contracts/ui.md U3/U5: `update` returns `Choice { step, terminal_had_focus, close_overview_first }`, `list` returns `Empty`; Tab listed as a close | agent-resolved | speckit-analyze report (tasks unit) |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 06e0b7cb93e4b1df3ad203f637ff3bcdfeb41cb2:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN: 3 MINOR, fixed, prose only |
| Review plan | 1 | 96ee1ce3727ec57086e810a9cf02650703d31bd7:f745f268a3ce6dae0a47b8157e100bb4ce069394 | CLEAN, no findings |
| Review tasks | 1 | e5bd85269a8ad9d34b6748a7abc64f4fcc86a62a:8af03b922fe8dbea230e8f5930cb7cdc4f6d81e1 | CHANGES: 1 MAJOR ([P] on shared test file), 1 MINOR (T016 before T015), both fixed |
| Review tasks | 2 | 2d709358b85fc04a2ea24af63fdf9e8e53f684ca:5a006c9ebfbf54f70f5c50de2febf64543ca50db | CLEAN (sonnet, scoped) |
| Review A M1 (code-review high) | 1 | 249338d7d4d91aafb452af9b3c8b561cb88dc7ef:316d1b6b5767670852fe8e37bf4296598a2695b4 | 1 MAJOR (candidates rebuilt the sidebar per location) fixed; 5 MINOR: 2 fixed (clones, doc width), 3 declined |
| Review A M1 (sonnet, scoped) | 2 | 0d0c5ba3498562676ea8c6a61eacf9ff7f5c3c19:51ebbf93e8f5d60473867be8e60ad3cb9aa8253e | running |
| Review B M1 (conformance, sonnet) | 1 | 0d0c5ba3498562676ea8c6a61eacf9ff7f5c3c19:51ebbf93e8f5d60473867be8e60ad3cb9aa8253e | running |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | MINOR: user-guide row says "location, session or project" while M1 lists locations | the row text is fixed by contracts/keys.md K4 / T020; M2 (sessions) and M3 (projects) make it true before the feature closes. |
| M1 | A | MINOR: stored highlight key not written back when its entry leaves, so it can return | `list()` already shows the first entry while the key is absent, which FR-013 asks; the return-on-reappear case is outside FR-013 and needs a write in a pure read path. |
| M1 | A | MINOR: `terminal_had_focus` taken from the publisher, not `terminal_focused()` at open | mirrors the #803 Overview chord's existing split; latent only, no failing case. |

## Handover

None.

## Open escalation

None. (Q1 answered provisionally as D4; user confirmation pending.)

## Follow-ups not done

None.
