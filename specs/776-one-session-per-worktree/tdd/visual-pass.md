# Visual pass, feature 776 (quickstart Part B items 1, 2, 4)

Real client + daemon (pinned copies of this worktree's build), private headless sway 1600x1200,
lavapipe, private data dir, throwaway git repo as the project. Screenshots in `visual-pass/`.

| Item | Result | Evidence |
|---|---|---|
| B1 dialog fields, dark | pass | `b1-dialog-dark.png`: "AI CLI" select on the Type row (right half), "First prompt" below Daemon, all inside the dialog; `b1-ai-cli-options-dark.png`: Claude Code, OpenCode, None (None last); `b1-none-hides-prompt-dark.png`: None hides First prompt; `b1-filled-dark.png` |
| B1 dialog fields, light | pass | `b1-dialog-light.png`: same layout, dropdown open |
| B1 confirm creates worktree and starts session | pass | `b2-session-started.png`: Claude Code session running in the new worktree after Create |
| B2 worktree with session: no start action | pass | row hover shows only the delete icon (zoom in `b4-sidebar-one-session.png`) |
| B2 worktree without session: one start action | pass | `b2-row-no-session-hover.png`: one split start control ("+" and chevron CLI picker) |
| B2 Default rule | pass | Default shows the same "+" / chevron control while it has no session (`b2-default-cli-menu.png`) |
| B4 one session: no sub-rows, no expander | pass | `b4-sidebar-one-session.png` |
| B4 two sessions: collapsed expander, "helper, <cli>" | NOT OBSERVED | no UI path reaches a second session (see below) |

## Not observable
A second session on one worktree could not be created through the client: the worktree row menu
(`b4-row-menu-one-session.png`) offers Review changes, Copy name, Rename, Close session, Remove
session, Delete. There is no "Start another session..." entry (quickstart item 5) and the start
control is gone from an occupied row, so the expander and the "helper, <cli>" annotation
(`crates/micold-client/src/ui/sidebar.rs:919`) were not rendered. Not in the showcase either.

## Defects
None attributable to 776. Observation: the "Existing branch" tab shows the selected outline on first
open of New worktree while Create produced a new branch (feat/<name>); likely pre-existing, not 776.
