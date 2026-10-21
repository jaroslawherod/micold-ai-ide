# Quickstart: validating session usage, the context meter, subagents and background tasks (#944)

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Insights model on the fixtures | `cargo test -p micold-core --test structured_insights` (and `mise run gate`) | I1–I6 of [contracts/insights-model.md](contracts/insights-model.md): fresh-build equality, meter and turn figures equal the fixtures', totals across 1–50 turns, restart and resume, attribution at every depth, task states, tolerance (SC-001–SC-003, SC-006) |
| Plan-limit parse | `cargo test -p micold-core --test plan_usage_rate_limit_event` | each recorded fixture's event → two windows; malformed → none |
| Plan-usage feed | `cargo test -p micold-daemon --test plan_usage_structured` | on → a reading in a test client; off → none; restored history → none (SC-004) |
| Subagent text forwarded | `cargo test -p micold-daemon --test structured_session` | the stand-in sees `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1` and the seven structured arguments unchanged |
| Figures as text | `cargo test -p micold-core --lib structured::insights` | grouping, cost "est.", percent, absent figures give no text |
| View state | `cargo test -p micold-client --test features_structured_insights` | thread fold per window, open threads follow live events and rebuild after eviction, background list fold, isolation of two sessions (FR-019, Principle II) |
| Shared components | `cargo test -p micold-client --lib ui::material::progress_bar` and the gates | marker inside the track; warning fill legible in both schemes |
| Unchanged | `mise run gate` | every existing terminal-session and plan-usage test passes (SC-007) |

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` (2.1.306 or later) on `PATH`; the dev client against a project
with a structured session; **Show Claude plan usage** on.

1. Send a prompt that reads a file: the session bar shows the meter (tokens / window, a whole
   percent, the threshold tick) and the totals; the footer shows the turn's cost "est." and
   "in · out" tokens beside #939's duration and session total; hover both for details (cached
   input, per-model rows, the estimate note).
2. Before the first reply of a new session: the meter says "No data yet".
3. Ask for a subagent ("use the Agent tool to read notes.txt"): the `Agent` card shows a folded
   thread "Subagent · running · N tool calls · <activity>", which turns to finished; unfold it:
   the subagent's text, thinking and tool cards, indented, in order; nothing of it in the main
   transcript. With a nested subagent (an agent given the Agent tool), the inner thread folds under
   the inner card.
4. Ask for a background command (`sleep 20; echo done` in the background): the bar shows "1
   background task running", the list shows it running then finished with its summary, and the
   `Bash` card shows the task's state. Stop the session while one runs: it shows "ended without a
   report".
5. The app bar's plan-usage indicator updates from the structured session alone (no terminal
   session open); turning the switch off hides it.
6. A second window on the same session: same figures, threads and tasks; folding a thread in one
   window does not fold it in the other.
7. A long session (or `CLAUDE_CODE_AUTO_COMPACT_WINDOW` lowered so compaction comes early): the
   meter's warning look (icon + "Near compaction") appears at the threshold and the meter drops
   after compaction.
8. Light and dark theme; greyscale screenshot of the warning and a failed thread (not colour
   alone).

Record the pass with the `visual-pass` skill into `specs/944-session-usage-subagents/visual-pass/`.
