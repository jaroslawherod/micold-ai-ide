# Autopilot ledger — #970 overview-fullscreen

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #970 — "Overview: full-screen mode hides the sidebar, animated" (title and body as in spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #970
- **Worktree branch**: claude/project-thread-ag6ana
- **Started**: 2026-10-18
- **Phase**: clarify
- **Next step**: clarify unit (continued) — apply the user's answers to FR-001 / FR-008 (see Open escalation), then run clarify round 2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 0f936e933b45470ec78960e325652da519ed1345:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CHANGES: 4 MAJOR, 2 MINOR |
| Spec review | 2 | 89236166dc9f65c1c4ccda6390048046cc91bcdd:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CLEAN (2 MINOR, fixed prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 1, product decisions the repo does not settle):

1. **FR-001 — how is full-screen mode entered?**
   - (a) (Recommended) A toggle on the Overview (button in its header + a keyboard shortcut), off each time the Overview opens. Evidence: view state is not persisted in this app (specs/030-settings-rail-slide/spec.md: rail state "stays view state … lasts as long as the process"); feature 813 keeps "remembering the Overview open" out of scope (specs/813-worktree-overview/spec.md#Out of Scope); the issue asks for a mode the user switches into, not a new default.
   - (b) The same toggle, but the last choice is remembered and the Overview reopens in it.
   - (c) No toggle: the Overview always opens full-screen; closing it is leaving the mode.
2. **FR-008 — where does the reduced-motion preference come from?**
   - (a) (Recommended) A new app setting *Reduce motion* in Settings, off by default, applied to this zoom (other animations unchanged; follow-up to extend). Evidence: no reduced-motion source exists (specs/007-motion-overlay-fade/spec.md:115, specs/030-settings-rail-slide/spec.md:240, specs/971-overview-card-ripple/spec.md:161); iced 0.14 / winit expose no OS reduced-motion API and `dark-light` reads only the theme, so (b) needs separate native code per OS (GNOME gsettings, macOS NSWorkspace, Windows SPI) — a setting behaves the same on all three platforms (Principle VI) and is testable.
   - (b) The operating system's reduced-motion preference where the platform exposes one.
   - (c) Both: the app setting overrides the OS preference.


## Follow-ups not done

None.
