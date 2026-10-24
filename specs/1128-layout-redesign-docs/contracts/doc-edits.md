# Contract: user-guide edits

What each docs milestone changes, how the hits are found, and what "done" reads as. Owning sections:
[research R2](../research.md#r2--owning-sections). Line numbers are as found at plan time on main
after #1124 and #1126; each milestone re-runs the patterns and works from its own result.

## Patterns

Run over `docs/user-guide/*.md` and `site/media.toml`, case-insensitive:

| Stage | Pattern (grep -E -i -n) | Finds |
|---|---|---|
| 1 | `under the (terminal\|pane)\|bottom bar\|below the (terminal\|pane)` | the removed bar under the terminal |
| 1 | `\bthe bar\b\|bar's\|in the bar` | FR-003 ambiguous "bar" (four pages only) |
| 1 | `status bar\|connection (state\|item)\|sandbox item\|plan usage\|CLI item` | status-bar mentions to link (FR-004) |
| 1 | `banner\|bring-up row\|under the (app\|top) bar` | a row stacked in the steady state (FR-001) |
| 1 | `tab strip\|header above\|context header\|restart` | session-area mentions to link (FR-004) |
| 2 | `app bar\|top bar\|toolbar\|next to the menu button\|project switcher` | the old title bar (FR-005, FR-007) |
| 2 | `Overview\*\* (button\|in the top)\|press \*\*Overview\*\*` | the removed Overview button (FR-006) |
| 2 | `filter button\|tag-filter\|filter panel\|type chip\|issue chip\|tagged with the kind` | the removed sidebar controls (FR-005) |
| 2 | `quick switcher\|Ctrl\+Shift\+P\|Cmd\+Shift\+P` | FR-008 jump-to field mentions |
| 2 | patterns for anything else #1125 or #1127 removed, from their merged specs | spec Terms: the list is read from what shipped |

A hit is fixed, linked, or kept as *historical* (it says the element is gone, such as
`appearance-theming.md:48` "it is no longer in the app bar"). Nothing else on a page is rewritten
(FR-011).

## Stage 1 — inventory at plan time (M1)

| # | File:line | Kind | Change |
|---|---|---|---|
| E1 | `worktrees-and-sessions.md:1195` | ambiguous bar | "the bar in the error colour" → "the meter's bar in the error colour" |
| E2 | `worktrees-and-sessions.md:1228` | unlinked | "the status bar's" links to `appearance-theming.md#status-bar` |
| E3 | `worktrees-and-sessions.md:1583` | stale (bar under the terminal) | "press **restart** in the bar" → "press **restart** beside the status in the [tab strip](./terminal-panes.md#the-tab-strip-above-the-terminal)" |
| E4 | `worktrees-and-sessions.md:1586` | stale | "the bar's `failed`" → "the tab strip's status `failed`" |
| E5 | `worktrees-and-sessions.md:1603` | stale | "the `restart` control in the bar" → "the **restart** control in the tab strip" (linked) |
| E6 | `worktrees-and-sessions.md:1796–1801` (`### The tab strip`) | describes the element again | the opening paragraph's member list becomes one sentence linking to `terminal-panes.md#the-tab-strip-above-the-terminal`; the instance rules (one tab marked, the AI tab, right-click) stay; the anchor `#the-tab-strip` stays |
| E7 | `worktrees-and-sessions.md:1494, 1698, 1748, 1778, 1788` | unlinked tab-strip mentions | the first mention in each section links to the owning section; text unchanged otherwise |
| E8 | `worktrees-and-sessions.md:2052` | unlinked | "the status bar shows the usage" links to `appearance-theming.md#status-bar` |
| E9 | `overview.md:263` | unlinked | "the status bar at the bottom" links to `appearance-theming.md#status-bar` |
| E10 | `settings.md:292`, `sandboxed-daemon.md:504` | unlinked (outside the four, FR-011) | link added if missing; sentence otherwise unchanged |
| E11 | `worktrees-and-sessions.md:1561` "an error banner" | checked | a failure with an action raised as a banner or notification, as #1124 kept: correct, unchanged (US1 scenario 2) |
| E12 | `worktrees-and-sessions.md:409–416` stored-sessions banner | checked | a banner inside the sidebar, not a removed element (spec Terms): unchanged |

Out of stage 1 though found by its patterns: `appearance-theming.md:58` and `sandboxed-daemon.md:505`
"under the app bar" name the title bar's predecessor, so they change in stage 2 (FR-011 staging).

## Stage 2 — known hits at plan time (M2)

Read again after #1125 and #1127 merge; each may already be fixed by them.

| # | File:line | Now says | Becomes |
|---|---|---|---|
| S1 | `worktrees-and-sessions.md:13` | **Open another project** (app bar) | where #1127 puts it, linked |
| S2 | `overview.md:20` | Press **Overview** in the top bar | the sidebar's Overview row and Ctrl+Shift+O (Cmd+Shift+O on macOS) (FR-006) |
| S3 | `overview.md:36, 43` | the sidebar and the top bar stay | which regions stay, open and full screen, as shipped (FR-006) |
| S4 | `project-selection.md:70–72` | `## Switching projects from the top bar`, next to the menu button | a heading naming the breadcrumb; where its panel opens (FR-007). Inbound links to `#switching-projects-from-the-top-bar` are re-pointed (FR-019) |
| S5 | `project-selection.md:155` | open the switcher in the top bar | the breadcrumb's project part |
| S6 | `worktrees-and-sessions.md:320` | the **filter** button … tag-filter panel | the chip bar (US2 scenario 4) |
| S7 | `appearance-theming.md:4, 12, 58` | top app bar; banner under the app bar | the title bar; banner under the title bar |
| S8 | `icons.md:12, 30, 41, 86` | Top app bar; Sidebar toolbar filter; Top bar **Overview** button | the shipped locations |
| S9 | `settings.md:4, 52` | The app bar … the app bar's mode button | the title bar / where the mode control ships |
| S10 | `sandboxed-daemon.md:505` | a row under the app bar | under the title bar |
| S11 | `site/media.toml` `worktree-sidebar-light` alt | each tagged with the kind of branch it holds | the two-line rows as shipped (R4) |
| S12 | `worktrees-and-sessions.md` new opening section | — | FR-009: names title bar, sidebar beside the session area, status bar, top to bottom, each linked; placed before `## Opening a project (git repositories only)` |
| S13 | quick-switcher mentions | — | jump-to field and Ctrl+Shift+P (Cmd+Shift+P on macOS) both named (FR-008) |
| S14 | empty states: `project-selection.md#opening-a-project`, `worktrees-and-sessions.md#opening-a-project-git-repositories-only` | — | what the empty screen shows (title bar and status bar only) and where the open-project actions are (Edge cases) |
| S15 | sidebar groups | — | a project with no worktrees shows the Overview row, Default and no group; a row moves between groups as its state changes (Edge cases) |

## Done when

- The stage's patterns return only owning sections, linked mentions and *historical* hits (SC-001,
  SC-002).
- `site/checks/links.sh --sources` passes (SC-003, R10).
- Every chord the four pages name has its macOS form (FR-018, R11).
- The five SC-007 answers (open the Overview, switch project, open the quick switcher, see the
  connection state, switch to a shell) are each in the first paragraph of their owning section;
  stage 1 holds the last two.
