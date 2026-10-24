# Contract: visual-pass record (M4)

File: `specs/1128-layout-redesign-docs/visual-pass.md`. Screenshots:
`specs/1128-layout-redesign-docs/visual-pass/<board>-<theme>-1280.png`, `<board>` one of
`session-view`, `overview`.

## Header

- **Commit**: the full `HEAD` SHA the frames were drawn at (main after #1124–#1127, plus M3).
- **How it was run**: the route of [research R8](../research.md#r8--how-the-visual-pass-draws-the-window),
  the states drawn (`window-session-view`, `window-overview`), the window size, the renderer.
- **Not checked**: macOS and Windows by eye (Linux only); sizes other than 1280×800; failure states
  (held by `specs/1124-bottom-status-bar/visual`).
- **Mockup**: *Micold Layout Redesign*, boards *Proposal: session view* and *Proposal: Overview*.

## One table per screenshot (4)

| Region | Verdict | Reason | Contrast |
|---|---|---|---|
| Title bar | matches / differs on purpose / differs by defect | for *on purpose*: the decision and its spec (for example `specs/1125-sidebar-state-groups/spec.md` Clarifications); for *defect*: the region's issue and the follow-up line | the composition-contrast test cited, or the measured ratio (R9) |
| Sidebar | … | … | … |
| Session area (or Overview) | … | … | … |
| Status bar | … | … | … |
| Whole window | … | alignment between regions; no stacked chrome row; nothing overlaps | — |

4 tables × 5 rows = 20 verdicts (SC-006). Every non-*matches* row has a reason.

## Defects

Each *differs by defect* also gets a line in the ledger's *Follow-ups not done*: region, issue,
what is wrong, the screenshot. No code changes (D1, FR-020). Body text under 4.5:1 is a defect
(FR-016).
