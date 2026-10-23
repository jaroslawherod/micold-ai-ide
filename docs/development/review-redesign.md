# Review UI redesign: proposal

Status: proposal. Goal: make the Changes view answer "what did the agent do, is it right, what do I
tell it next" with less hunting. Builds on 482 (review), 946 (replies). Code: `ui/changes.rs`,
`features/changes.rs`.

## What the view does today

Header (entry name, base line, **Review (n)**, **Close**) over three columns, 2 : 3 : 2:

1. **Left:** Committed / Uncommitted / Files toggles, Pull requests section (max 220 px), file list
   (36 px rows: path, kind tag, `+a −r`, pending count).
2. **Middle:** PR header, unified / side-by-side + Whole file chips, the diff with inline comment cards.
3. **Right:** review panel: Clear sent, Discard drafts…, Send review (n), status counts, one row per comment.

## Problems

1. **Three columns squeeze the diff.** The diff gets 3/7 of the window while the file list and the
   panel are mostly idle. Long paths and wide lines truncate or scroll sideways.
2. **No sense of progress.** A file row shows `+a −r` and a draft count, but not whether the file
   was *looked at*. After a 30-file agent run there is no "5 of 30 reviewed", no next-unreviewed jump.
3. **File list is flat and path-heavy.** The directory prefix repeats on every row; the file name,
   the thing you scan for, is the least prominent part. No grouping, no sort by size of change, no filter.
4. **Review state is split across two places.** Comments live in the diff, status lives in the
   panel, the send button lives in the panel (or the header when the panel is closed, so it moves).
   The most important action has no fixed place.
5. **The panel mixes two jobs:** your drafts (to send) and the agent's replies (to read). The agent's
   outcome (Applied / Answered) is what you act on after a send, yet it is a 1-line row ellipsis.
6. **No keyboard flow.** Files, hunks and comments need the mouse; reviewing 30 files is slow.
7. **Pull requests and the working tree share one list pane** and the PR section pushes the file
   list down; switching source is a click on a row, not a visible mode.
8. **Setup chrome is always visible:** Committed / Uncommitted / Files, base line and PR list take
   space every time though they are set once.

## Reported: the view is modal and forgets everything

- **It replaces the terminal.** The view takes the session's place, and `Msg::Closed | SessionSelected`
  sets `state.open = None` (`features/changes.rs:1728`): selecting a session, or anything else,
  drops the whole `OpenView` (selected file, scrolls, expansions, whole-file choices, toggles, open
  composer, panel state, browsed file). Only comments and the chosen PR survive (`reviews`, `remembered`).
- **You cannot see the code and the conversation together**, nor read the file while the panel is up.

## Fit with the open layout redesign (#1124–#1128)

Open issues already redesign the window around the same mockup (Layout Redesign board "Proposal:
session view"). The review work must sit inside it, not beside it:

| Issue | What it sets | Consequence for review |
|---|---|---|
| #1126 worktree context header + session tab strip above the terminal; header has a **Changes** button (files, +/−) that opens the Changes view | The tab strip is the natural home of the Review tab. | The **Changes** button stops opening a modal view and opens/focuses the **Review** tab in that strip. Header chips (state, branch, PR) stay; the PR chip can open the PR source in the Review tab. Pane chords and focus unchanged. |
| #1124 28 px bottom status bar (daemon, sandbox, CLI, hints) in every body branch, incl. Changes | The window already gets a bottom bar. | The review bar (counts, **Send review**) is scoped to the Review tab and sits *above* the status bar; it is not window chrome. Keep it one row of 44 px. |
| #1125 sidebar grouped Needs action / Working / Idle, Overview first, two-line rows | Sidebar row is the entry point. | **Review changes** stays in the row's menu and opens the tab; the row shows *Review · n drafts* as a third line only when drafts exist. |
| #1127 single 44 px title bar, breadcrumb, jump-to field | Quick switcher lives in the title bar. | Jump-to field gains files of the open review (`g f`) and comments; no review UI in the title bar. |
| #1128 docs, layout snapshot, visual pass | Same checklist applies. | Each slice below updates `reviewing-changes.md` and the layout snapshot; visual pass at 1280×800, both themes. |
| #1150 (in progress) full file and repo browsing in Changes | Whole file, Files, Open diff exist. | The source dropdown and the file tree must keep **Whole file**, **Files**, **Back**, **Open diff**; the tree reuses #1150's `tree_rows`. Do not rework it before #1150 merges. |
| #1260 bundled monospace font | Diffs look the same on every host. | Take it before the diff polish in slice 1; the mockup's diff font is a placeholder. |
| #945 / #941 structured session UI | A session may be a structured transcript, not a PTY. | Review tab is session-kind agnostic; **Send review** goes through the existing send path. In split mode, the structured transcript can sit beside the diff. |

Sequencing: #1126 first (it creates the tab strip), then phase 0 below is a small step: the Review
tab is one more tab in a strip that exists.

## Proposal

### 0. Review is a tab, one per entry, and keeps its state

The entry (worktree) gets a **Review** tab beside its sessions, in the same tab strip, with the
draft count as a badge (*Review · 3*). It is not a mode that replaces the terminal:

- Selecting a session tab shows the session; selecting **Review** shows the review. Neither closes
  the other. **Close** becomes the tab's ×.
- `State.open: Option<OpenView>` becomes `views: BTreeMap<(PathBuf, SessionLocation), OpenView>`
  plus the active entry's tab selection. Switching tab, session, worktree or project keeps each
  entry's `OpenView` (file, scrolls, expansions, whole-file, composer text, picks, drawer, source).
  It is dropped only by × or when the worktree is removed (as comments are today).
- While another tab is shown the view does not re-read the diff on every change; it marks the tab
  *changed since you looked* and re-reads when shown (file watching stays, reads are lazy).
- Opening from the sidebar (**Review changes**) focuses the existing tab or creates it.
- **Split option:** drag the tab to the right half to see the session beside the review (agent
  conversation next to the diff, so a comment can be checked against what the agent said). Follow-up,
  not in the first slice.
- The panel/drawer state (open, tab, scroll) lives in the same `OpenView`, so the reported
  "covered by the comments panel" case is solved twice: the drawer overlays only on demand, and the
  file is one click (or `Esc`) away.

### Layout

Two columns plus a bottom bar; the panel becomes a drawer.

```
┌ Review: fix/1117 · vs main@a3f9c21 ─ [Working tree ▾] ─ 12/30 reviewed ───── [Close] ┐
│ ▸ src/            │ src/daemon/tests.rs        +42 −8   [Unified|Split] [Whole] ⌃ ⌄ ✓ │
│   ✓ lib.rs   +3 −1│ @@ -10,6 +10,9 @@ fn spawn()                                       │
│   ● tests.rs +42 2│  10  10   let a = 1;                                              │
│ ▸ docs/           │  11      -let b = 2;                                              │
│   ○ guide.md +9   │      11  +let b = 3;                                              │
│                   │      ┌ Draft ─ Why 3? ─────────────── Edit  Delete ┐              │
│ [filter files…]   │      └─────────────────────────────────────────────┘              │
├───────────────────┴───────────────────────────────────────────────────────────────────┤
│ 3 drafts · 2 sent · 1 applied · 1 answered   [Comments ▴]   [Discard] [Send review 3] │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

Changes, in order of impact:

1. **Reviewed marks.** Each file gets a state: `○` unseen, `●` has drafts, `✓` marked reviewed
   (button / `v` in the diff header). The header shows *12/30 reviewed*. State is per entry and head
   commit; a file whose diff changes after the mark goes back to `○` with a *changed since* dot.
   Stored in the session service next to comments, so every window agrees.
2. **File tree, not flat list.** Directories collapse (folders with one child chain into `a/b/c`),
   file name first and bold, `+a −r` as a small bar. Sort: path (default), size of change, unreviewed
   first. A filter box (reuse **Open file by path**'s matcher) narrows the tree. Width: fixed
   ~260 px, resizable, collapsible; the diff takes the rest.
3. **Source is one control.** *Working tree ▾* is a dropdown in the header: Working tree
   (committed + uncommitted as two chips inside it), Pull requests (#number · title · checks), Files.
   The PR section leaves the left column. The base line moves into the dropdown's tooltip and the
   header subtitle.
4. **Fixed bottom bar owns review state.** Status counts, **Send review (n)** (always in the same
   place, filled when n > 0), **Discard** and **Clear sent** in an overflow. Never moves when a
   panel closes. Also shows "Sending…" and the send error.
5. **Comments drawer replaces the right column.** **Comments ▴** opens a drawer over the diff's
   right 360 px (not a third column). Two tabs: **Drafts** (edit / delete, jump) and **Replies**
   (the agent's Applied / Answered, full text wrapped, **Mark applied** / **Answer…** inline).
   Opens itself on a send that gets replies; otherwise stays closed so the diff keeps its width.
6. **Next/prev everywhere.** Header buttons and keys: `j`/`k` next/prev file, `n`/`p` next/prev
   hunk, `]`/`[` next/prev comment, `v` toggle reviewed, `c` comment on the picked lines, `g`
   unreviewed next, `Ctrl+Enter` save comment (exists), `Ctrl+Shift+Enter` send review. Shortcut
   sheet on `?`.
7. **Replies visible in place.** A sent comment card shows its status badge and the agent's reply
   under it (first 4 lines, **Show all**); the drawer is the index, not the only place.
8. **Stale-awareness.** When the agent edits while you review, the file row gets a dot and the
   header says *3 files changed since you looked*; the open diff does not reflow until you press
   it (today it re-reads within 2 s, which moves what you are reading). Picked lines and composer
   text are kept either way.
9. **Quick comment.** Click-drag over line numbers picks lines and opens the composer at once
   (today: pick, then **Add comment**). Suggest chips under the box (*Rename*, *Add test*, *Why?*)
   fill common text; off by default in Settings.

## Not changing

Comment anchoring, review message format, statuses, `report_review_comment`, diff rendering,
syntax colours, large / binary file handling, PR diff reading and limits, per-entry storage.

## Phasing (each a separate spec)

| # | Slice | Needs |
|---|---|---|
| 0 | Review as a per-entry tab; views kept across switching (section 0) | client state + tab strip; no protocol |
| 1 | Bottom bar + comments drawer (items 4, 5, 7) | UI only |
| 2 | Source dropdown, 2-column layout (3) | UI only |
| 3 | File tree, filter, sort (2) | `features/changes.rs` tree rows exist (`tree_rows`); reuse |
| 4 | Reviewed marks + progress (1) | core + daemon protocol field, persisted with comments |
| 5 | Keyboard map (6) | focus model in `ui/focus.rs` |
| 6 | Stale-awareness, quick comment (8, 9) | follows 4 |

Slices 1–3 deliver the layout win without protocol change; 4 is the feature that makes reviewing a
large run tractable and is the one to prioritise if only one is built.

## Scope and files

- `ui/changes.rs`: header, two-column body, bottom bar, drawer; `features/changes.rs`: `Panel`
  becomes drawer state with a tab, reviewed set, sort and filter in `OpenView`.
- `micold-core` + daemon: `reviewed` set per entry and head (slice 4).
- `ui/material`: bottom bar and drawer components (check the component library first).
- `docs/user-guide/reviewing-changes.md`, layout snapshot, showcase `sections/review.rs`, visual
  pass at 900 / 1280 / 1920 px wide.

## Open questions

0. Tab strip: does the Review tab sit with the sessions of its worktree, or in a separate row? Kept
   in memory only (lost on restart), or persisted (selected file, source) with the comments?
1. Reviewed marks stored in the daemon (shared across windows, survives restart) or per window only?
   Proposed: daemon.
2. Drawer over the diff, or push the diff narrower when open? Proposed: over, since the diff
   needs its width, and it closes on Esc.
3. Should unreviewed-first be the default sort once any file is marked?
4. Keep **Files** (repo browser) inside the source dropdown, or as its own header button?
5. Is a vim-style key map acceptable as default, or should it sit behind a setting?

## Tasks

All depend on the **app layout rebuild**: #1124 (status bar), #1125 (sidebar), #1126 (worktree header
and session tab strip), #1127 (title bar), #1128 (docs, snapshot, visual pass). #1126 is the hard
prerequisite: the Review tab lives in the strip it creates. Mockup: the *Review tab redesign*
canvas (shared components and `tokens.css`; keep it in step with each task). Each task follows the
repo's acceptance habit: pure logic in `micold-core` with unit tests, docs, layout snapshot and
visual pass at 1280×800 in both themes.

| ID | Task | Blocked by |
|---|---|---|
| #1269 (R0) | **Review as a per-entry tab.** `State.open` becomes a map keyed by entry; the header **Changes** button and **Review changes** open or focus the tab; switching tab, session, worktree or project keeps the view; × or worktree removal drops it; hidden tab marks "changed since you looked" and re-reads when shown. Drafts badge on the tab. | #1126 |
| #1270 (R1) | **Review bar and comments drawer.** Fixed bar above the status bar (counts, **Send review (n)**, overflow with Clear sent / Discard drafts…); right-hand drawer with **Drafts** and **Replies** tabs, overlaying the diff, `Esc` closes; replies shown on cards. | R0, #1124 |
| #1272 (R2) | **Source dropdown and two-column layout.** Working tree (Committed / Uncommitted chips), Pull requests, Files in one control; the PR section and toggles leave the left column; keep Whole file, Files, Back, Open diff. | R1, #1150 merged |
| #1273 (R3) | **File tree, filter, sort.** Collapsible directories, name-first rows, `+a −r` bar, filter box, sort by path / size / unreviewed first; reuse `tree_rows`. | R2, #1150 merged |
| #1271 (R4) | **Reviewed marks and progress.** `reviewed` set per entry and head, stored by the session service with the comments (protocol field, all windows agree); *n of m reviewed*; a changed file drops back to unseen. | R0 |
| #1274 (R5) | **Keyboard map.** `j`/`k` file, `n`/`p` hunk, `]`/`[` comment, `v` reviewed, `g` next unreviewed, `c` comment, `Ctrl+Shift+Enter` send, `?` sheet; jump-to field searches files and comments (#1127). | R3, R4, #1127 |
| #1275 (R6) | **Stale-awareness and quick comment.** "n files changed since you looked" without reflowing the open diff; drag over line numbers opens the composer; optional suggestion chips. | R4 |
| #1276 (R7) | **Docs, snapshot, visual pass, mockup sync.** `reviewing-changes.md`, `terminal-panes.md`, `worktrees-and-sessions.md`, layout snapshot, visual pass; update the canvas to the as-built state. | R0–R6, #1128 |
| R8 | **Diff font** (take #1260 first so the polish is judged on the final font). | #1260 |

Order: #1124–#1127 → #1126 → R0 → R1 / R4 (parallel) → R2 → R3 → R5, R6 → R7. Epic: #1277.

## Overview (earlier redesign) in the same snapshot

The Overview cards from `overview-redesign.md` (state stripe on a neutral card, title first, empty
rows removed; builds on 813, 970, 971, 972, 1024) are part of the same window, so the mockup canvas
now has an **Overview** board and an `OverviewCard` component beside the review boards. Both share
`TitleBar`, `Sidebar` (Overview row selected), `StatusBar` and `tokens.css`. Overview is a body
branch of the new layout (#1125 puts its row first in the sidebar, #1124 shows the status bar in it),
not a separate chrome. The mockup shows three cards per row at 1180 px; five per row applies on wide
windows. Card content is taken from the proposal, not from the current app: check it against the
build when the snapshot is synced (R7).
