# Contract: what #944 adds to #939's transcript view

#939 owns the pane, the entries, `DisclosureCard`, the footer and the renderer seam
(`specs/939-structured-transcript-view/contracts/`). #944 adds to them at the five points below
and changes nothing else in #939's drawing. #940–#943 are not assumed: #944 touches no block
renderer and no input row.

| # | Where (#939) | #944 adds | Needs on main |
|---|---|---|---|
| X1 | top of the transcript pane (`ui/transcript/mod.rs` `view`) | the session bar: context meter, session totals, running background-task count, and the folded background-task list under it (`ui/transcript/session_bar.rs`) | #939 M5 (view) and M6 (pane) |
| X2 | the footer entry (`EntryKind::Footer`, its pure view model of #939 T032) | after #939's duration and "session total": the turn's cost (`… est.`) and tokens (`in N · out N`) from `SessionInsights::turn(seq)`; details: cached input, per-model rows, the estimate note (FR-006, FR-026) | #939 M5 |
| X3 | the tool-call card body (`EntryKind::ToolCall`) | for a card whose id has a thread: a nested `DisclosureCard`, folded by default, with the one-line thread summary, whose body draws the open thread's nested `Transcript` through #939's per-entry draw functions and the window's `BlockRenderers`; any depth (FR-014–FR-016) | #939 M4 (`DisclosureCard`), M5 |
| X4 | the tool-call card header (trailing) | the started background task's state, label + icon (FR-021) | #939 M4, M5 |
| X5 | the `SubagentNotice` entry | replaced, in place, by one placeholder `DisclosureCard` per `Unknown` thread naming its parent id, opening like X3 (FR-017) | #939 M5 |

## Rules

- **E-1** Every figure, label and state drawn at X1–X5 comes from a pure function of
  `micold_core::structured::insights` or `features::structured_insights` (tested); the draw code
  only lays them out (Principle I's GUI exception).
- **E-2** A thread's entries are #939's entries of `Transcript::nested(parent)`; #944 adds no
  entry kind and no event → entry rule.
- **E-3** Fold state of threads and of the background-task list is per window
  (`features::structured_insights`), never on the wire (FR-019).
- **E-4** Failure and warning are never colour alone: the meter's warning carries
  `Icon::UsageWarning` and "Near compaction"; a failed task or thread carries "Failed" and #939's
  `Icon::ToolFailed` (FR-003, FR-012 of #939).
- **E-5** If #939's merged view differs from the names above (for example the footer's view model
  takes other inputs), the tasks of the milestone that integrates it adapt to #939's shape; the
  core contract ([insights-model.md](insights-model.md)) does not change.
