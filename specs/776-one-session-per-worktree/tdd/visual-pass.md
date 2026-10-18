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
| B4 two sessions: collapsed expander, "helper, <cli>" | NOT OBSERVED here; observed in the M5 pass below (`b5-helper-added-selected.png`) | no UI path reached a second session until M5 added "Start another session..." (see below) |

## Not observable
A second session on one worktree could not be created through the client: the worktree row menu
(`b4-row-menu-one-session.png`) offers Review changes, Copy name, Rename, Close session, Remove
session, Delete. There is no "Start another session..." entry (quickstart item 5) and the start
control is gone from an occupied row, so the expander and the "helper, <cli>" annotation
(`crates/micold-client/src/ui/sidebar.rs:919`) were not rendered. Not in the showcase either.

## Defects
None attributable to 776. Observation: the "Existing branch" tab shows the selected outline on first
open of New worktree while Create produced a new branch (feat/<name>); likely pre-existing, not 776.

# Visual pass, feature 776 (quickstart Part B items 5 and 6, T049)

2026-10-16. Real client + daemon (pinned copies of this worktree's build, both contain
`allow_additional_session`), private headless sway 1600x1200 + lavapipe, private data dir,
throwaway git repo. Dark scheme only; light not exercised. Right-press needed for row menus, so the
pointer client was a copy of `vptr.py` with an `rclick` command (not committed). Claude Code was the
CLI; sessions sat at its trust prompt, which does not matter here.

| Item | Result | Evidence |
|---|---|---|
| B2 Default with a session: no start action | pass | row shows no "+" or chevron once a session runs |
| B5 menu of an occupied Default row has "Start another session..." | pass | seen in the pass (Review changes, Start another session..., Close session, Remove session); crop not kept |
| B5 confirmation names the existing session, explains helper | pass | `b5-start-another-confirmation-dark.png` |
| B5 Cancel changes nothing | pass | daemon log: still one `session started` |
| B5 Start adds a second, selected, marked "helper, claude", expander shown | pass | `b5-helper-added-selected.png` |
| B6 helper sub-row menu: Make main session / Close / Remove; main's: Close / Remove | pass | `b6-menus-helper-vs-main.png` |
| B6 Make main session: promoted row first and selected, former main "helper, claude" | pass | `b6-after-make-main.png`; project file shows `main: true` on the promoted id |
| B6 Remove on a main with helpers names the successor; Cancel keeps both | pass | `b6-delete-main-with-helper.png` |

## Not covered / observations
Lone-main and helper deletion wording (no successor line) not rendered. Both sessions are titled
"New session", so the successor sentence is not distinguishable by name in this fixture. Row menus
open on right-press only; the earlier left-press click with no preceding hover did not register in
this pointer setup (tool artefact, not the app).

# Visual pass, feature 776 (quickstart Part B item 3, M6)

2026-10-16. Real client + daemon (fresh build of this worktree at 20d031f7, pinned copies), private
headless sway 1600x1200 + lavapipe, private data dir and `CLAUDE_CONFIG_DIR`, throwaway git repo.
Dark scheme only; light not exercised. Seeding: two Claude transcripts with `cwd` = the repo root in
the private config dir. The first was adopted into the catalog as the Default worktree's session
("Refactor the parser module"); the second ("Fix the login bug") stayed stored, so "Attach existing..."
offered it for the occupied Default worktree.

| Item | Result | Evidence |
|---|---|---|
| B3 resume into an occupied worktree shows a confirmation | pass | `m6-b3-confirm-dark.png`: "Replace the session?" modal replaces the list at the dialog's width; text names both sessions ("Refactor the parser module" existing, "Fix the login bug" resumed) |
| B3 Replace / Keep both / Cancel visible and legible | pass | same image: filled Replace, outlined Keep both, text Cancel, one row, nothing clipped |
| B3 Cancel changes nothing | pass | `m6-b3-cancel-back-to-list-dark.png`: back on the stored-sessions list, "Fix the login bug" still offered; catalog file still holds only the first session |
| Replace and Keep both outcomes | NOT EXERCISED | out of scope for this item (covered by `attach_apply.rs` / client tests) |

## Observations
`m6-b3-sidebar-banner-dark.png`: the "Found 1 stored session" banner above Default. With an older
pinned binary (built before the last M6 commits) pressing Resume closed the dialog with no
confirmation and no attach; the fresh build shows the modal, so that was most likely the stale binary (not confirmed).
Right after pressing Resume the list stays up until the daemon answers; a screenshot taken a few
seconds later showed the modal. The Default row of an occupied worktree has no visible session sub-row
in the sidebar (no start action either), as B2 expects.

## Defects
None.
