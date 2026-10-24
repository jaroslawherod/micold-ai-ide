# Contract: user-guide outline (FR-001–FR-003, FR-014, SC-005)

Headings are the target; the merged slices' wording of a control wins over this list (spec Context).

## `docs/user-guide/reviewing-changes.md`

| Heading | Says (control or key → visible result) | FR |
|---|---|---|
| Opening a review | header **Changes**, sidebar **Review changes** → open or focus the worktree's **Review** tab; badge *Review · n*; one tab per worktree | 001 |
| Keeping and closing the Review tab | kept across tab, session, worktree, project switches; × or worktree removal drops it; *changed since you looked* on a hidden tab, re-read when shown | 001 |
| Choosing what to review | header source control: **Working tree** (Committed / Uncommitted), **Pull requests**, **Files**; **Whole file**, **Back**, **Open diff**; the error when a PR's diff cannot be read | 001 |
| The file tree | collapsible directories, `+a −r` bar, filter box, sort (path / size / unreviewed first) | 001 |
| Marking files reviewed | unseen / has drafts / reviewed; *n of m reviewed* and bar; a changed file returns to unseen; every window agrees | 001 |
| Reading a diff (and its existing subsections) | unchanged text and ids | — |
| Commenting on lines | adds: drag over line numbers opens the composer; suggestion chips setting (off by default) | 001 |
| Sending a review | review bar above the status bar: counts, **Send review (n)**, *Sending…*, send error, overflow **Clear sent** / **Discard drafts…**; sending from two windows | 001 |
| The comments drawer | **Drafts** / **Replies** tabs over the diff, `Esc` closes; replies on cards; empty tabs | 001 |
| Changed since you looked | the notice *n files changed since you looked*; the open diff does not reflow | 001 |
| Keyboard | the marker table ([guide-shortcut-table.md](guide-shortcut-table.md)); `?` opens the sheet; jump-to field finds files and comments | 001, 004 |
| Two worktrees, two windows | each tab keeps its own state; windows on one worktree share marks and drafts, also during a send | 001 |
| The review message, Statuses and replies, Pull requests (reading a PR's diff), The base line, The Default entry | unchanged text and heading ids | — |

### Must not say (FR-002)

Three columns; a left-column **Pull requests** section; **Committed** / **Uncommitted** / **Files**
toggles in the list; a right-hand review panel (*The review panel* heading goes); a **Close**
button; the view replacing the terminal or closing when a session is picked; re-reading the open
diff within 2 s. quickstart §A2 greps for each.

## `docs/user-guide/terminal-panes.md`

Tab strip section: the **Review** tab is a strip member, with its badge and *changed since you
looked* mark; pane chords and focus while it is shown (as R0 built them). FR-003.

## `docs/user-guide/worktrees-and-sessions.md`

Row menu **Review changes** (line ~978 today) opens or focuses the tab; the header **Changes**
button; the sidebar's *Review · n drafts* line if the build shows it; the link at line ~240 still
resolves. FR-003, FR-014.

## Inbound links (FR-014)

`settings.md:660` and `worktrees-and-sessions.md:240` → `#pull-requests`;
`agent-tools.md:300` → `#sending-a-review`; `agent-tools.md:311` → `#statuses-and-replies`. Each
keeps resolving or is re-pointed; `site/checks/links.sh --sources` passes.
