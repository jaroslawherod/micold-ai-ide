# Worktrees & Sessions

Micold AI IDE organizes your work into **worktrees** (isolated git branches checked out under
your project) and **sessions** (interactive `claude` runs inside a worktree). The left sidebar
shows worktrees at the top level and their sessions as sub-items; the right side hosts the
embedded terminal for the active session. The sidebar also always shows one **Default** entry —
a session location that isn't a worktree at all, for work you don't want to isolate onto its own
branch (see [The "Default" entry](#the-default-entry-sessions-without-a-worktree) below).

## Opening a project (git repositories only)

Open a project with **Open a project** (empty state) or **Open another project** (app bar), then
choose a folder.

- Only **git repositories** can be opened as projects. If you choose a folder that is not a git
  repository, opening is refused with a message and nothing is opened.
- Once opened, the project becomes the active context and the sidebar lists its worktrees.

## Browsing worktrees

- Worktrees discovered under `.claude/worktrees/` appear as top-level items in the sidebar.
- Expand a worktree (the leading toggle) to reveal its sessions.
- A worktree whose directory was deleted outside the app, or that is not a valid git worktree,
  shows its name in the error color with a **missing** or **invalid** status tag — you cannot
  start new sessions on it until it is resolved.

### Reading a worktree: name & tags

Each worktree is shown as a clean, human-friendly **name** on the first line, with small
color-coded **tags** beneath it:

- A **type tag** — the Conventional-Commits type from the worktree's branch (`feat`, `fix`,
  `chore`, `docs`, `refactor`, `test`, `build`, `ci`, `perf`, `style`). Each type has its own
  fixed color, so you can recognize what a worktree is for at a glance.
- An **issue tag** — the Jira-style key (e.g. `ABC-123`) when the worktree's name embeds one.
- A **status tag** — `missing` or `invalid` for a worktree that is not usable (see above).

The name is derived from the descriptive part of the branch: `feat/abc-123-login-page` shows as
**Login page** with `feat` and `ABC-123` tags. The tags are display-only — the underlying branch
and directory names are unchanged, and a worktree that does not follow the naming convention
simply shows no type tag.

The sidebar is intentionally compact — tight left/right padding and a slightly smaller font — so
long names and their tags get as much width as possible. It stays legible in both light and dark
themes.

### Filtering worktrees by tag

Tap the **filter** button in the sidebar header to reveal the tag-filter panel — it's hidden
by default so the list has the full sidebar to itself until you need it:

- Tap a **type chip** (e.g. `fix`) to show only worktrees of that type.
- Tap **issue** to show only worktrees that have a Jira key.
- Tap **untyped** to show worktrees that do not follow the naming convention.
- Multiple filters combine with **OR** — tapping `feat` and `fix` shows both.
- **Clear filters** restores the full list in one tap. If a filter matches nothing, an empty
  message with a clear action is shown.

Only chips for tags actually present in your worktrees are offered. Close the panel by
clicking outside it, pressing `Esc`, or tapping the filter button again — any active filter
stays applied either way. Whenever a filter is active, the filter button itself stays tinted
so you can tell filtering is on even with the panel closed.

### Agent worktrees

Some AI coding tools create their own throwaway worktrees inside your project — one per
background sub-task — using the same `.claude/worktrees/` folder the app manages. They have
machine-generated names such as `agent-a885b42dc521fbda1`, you didn't create them, and they
usually disappear on their own.

**The app hides them.** They don't appear in the sidebar, they aren't counted, and they never
show up as somewhere you can start a session. Your own worktrees are untouched, including any
whose name happens to begin with "agent" — only the machine-generated pattern (the `agent-`
prefix followed by a long run of hexadecimal characters) is treated as reserved.

Hiding is display-only. The app never deletes, prunes, renames, or otherwise modifies an agent
worktree or its branch — that lifecycle belongs to the tool that created it, not to the app. If
you want to see them, `git worktree list` in a terminal still shows everything.

**To see them in the app**, open the filter panel and tap **Show agent worktrees**. They join the
list, each marked with a muted `agent` chip so you can always tell them apart from your own work.
Tag filters apply to them exactly as they do to everything else.

Two things to know about that switch:

- It **resets itself** — every time you restart the app, and every time you switch projects. It
  applies only to the project you turned it on for, so you never land somewhere new with
  unexplained extra rows.
- Revealed rows are **fully live**: you can start a session in one, rename it, or delete it, with
  the same confirmation as any other worktree. There is no extra safety net, so take care —
  deleting a worktree an agent is still using will disrupt whatever it was doing, exactly as it
  would from a terminal.

### Resizing and hiding the sidebar

- **Resize**: drag the thin handle on the sidebar's right edge to make it wider or narrower.
- **Hide**: click the **hide** button (panel-collapse icon) in the sidebar header, next to
  **add worktree**. The sidebar collapses to a thin strip.
- **Show**: click the **show** button (panel-open icon) on the collapsed strip to bring it back.

## The "Default" entry: sessions without a worktree

Above your worktrees, the sidebar always shows one **Default** entry — even before you've
created any worktree at all. Starting a session from it runs directly in your project's own
root directory (whatever branch you currently have checked out there), instead of inside an
isolated worktree.

Use it for work that doesn't need its own branch: quick one-off commands, inspecting or running
the project exactly as it's currently checked out, or anything you deliberately don't want
isolated into a throwaway worktree. Creating a worktree first, just to run a quick command,
is unnecessary overhead this avoids.

- The Default entry is **not a worktree** — it has its own icon (a house) rather than the
  worktree iconography, so it's never mistaken for one, and it has no type/issue/status tags
  and no right-click menu (no rename, delete, or copy-name — those are worktree-only actions).
- Starting a session from it **never creates, modifies, or removes a worktree or branch**.
- It supports the same session actions as a worktree — start, switch, close, and it persists
  and restores across restarts exactly like a worktree-bound session (see
  [Starting, switching, and closing sessions](#starting-switching-and-closing-sessions) below,
  which applies equally here).
- You can run **multiple concurrent sessions** from the Default entry, the same as a worktree.
- Because every Default session shares your project's single checkout, actions that change the
  working tree (like switching branches from within one Default session's terminal) are visible
  to every other Default session too — this is expected, not a bug, since there's no worktree
  isolating them from each other.
- The Default entry is **never hidden by the sidebar's tag filters** — since it isn't a
  worktree, filtering by branch-derived tags doesn't apply to it; it always stays visible.
- Hover any sidebar entry, Default or worktree, to see a tooltip with its location relative to
  the project (e.g. the project root itself for Default, or a worktree's relative directory
  path) — useful for confirming exactly where a session is about to run before you start it.

## Creating a worktree

Click **add** in the sidebar header to open the New worktree form. At the top, two chips choose
where the worktree's branch comes from:

- **New branch** (the default) — describe the work and the app derives a fresh branch name.
- **Existing branch** — pick a branch that already exists. See
  [Working from an existing branch](#working-from-an-existing-branch) below.

### Creating a new branch

With **New branch** selected:

1. **Type** — a select control: click it to open a list of every Conventional-Commits type
   (`feat`, `fix`, `chore`, `docs`, …), then click one to choose it. The control always shows the
   currently chosen type when closed, and marks it in the list when reopened.
2. **Ticket** — optional reference (e.g. `ABC-123`). Leave blank to omit it.
3. **Name** — a short description (e.g. `login page`).

The form shows the derived names before you create:

- **Directory**: `.claude/worktrees/${type}-${ticket}-${name}`
- **Branch**: `${type}/${ticket}-${name}`

For example, `feat` + `ABC-123` + `Login page` creates the branch `feat/abc-123-login-page` and a
worktree at `.claude/worktrees/feat-abc-123-login-page`. With no ticket, `chore` + `cleanup` gives
`chore/cleanup`. Illegal characters in the ticket or name are automatically simplified (slugified).

Creating a worktree makes the new git branch and worktree for you — no manual git commands. If the
derived name collides with a branch that already exists, the app asks what you want to do rather
than refusing — see [Working from an existing branch](#working-from-an-existing-branch). If the
worktree *folder* already exists, creation is blocked with a message, because no branch choice can
resolve that. If anything fails partway, the app rolls back so no half-created branch or directory
is left behind.

If the project uses git submodules, they're fetched automatically as part of creating the
worktree — including submodules nested inside other submodules — so the new worktree is ready to
use immediately, with no extra `git submodule` commands to run yourself. Projects without
submodules are unaffected. While a worktree is being created, the form shows a progress bar
alongside a short description of what's currently happening. That description names the step for
what you actually chose — "Checking out existing branch" when you reused one, "Replacing branch and
creating worktree" when you overwrote one, "Creating tracking branch and worktree" when you
continued from a remote, and "Creating branch and worktree" for an ordinary new branch — followed by
"Setting up submodules" where they apply — the description only
ever names a step that's actually part of this creation, so a repository without submodules never
shows a submodule-related step. This can take a little longer than usual for a repository with
submodules to fetch, so seeing the description change (rather than a single static message) is
expected, not a sign the app is stuck. If creation fails, the progress bar stops and the
description stays on the step where it failed, next to the error message.

If fetching a submodule fails (for example, a network problem or a private submodule remote you
aren't authenticated against), the worktree is not created — the branch and directory are rolled
back the same way any other creation failure is, and the error names the submodule that failed
and why, so you can fix the problem and try again.

> Naming formats are fixed in this version and are intended to become configurable later.

## Working from an existing branch

Work doesn't always start in this app. You might have begun a branch in a terminal, pushed one
from another machine, or been handed one by a colleague. Either route below brings it into a
worktree without leaving the app.

### Picking the branch from a list

Choose the **Existing branch** chip in the New worktree form and search for the branch you want.
The field lists every branch until you type; from the first character on, it narrows to the
branches that match, and the characters you matched are picked out in colour inside each row — so
you can see *why* a branch is in the list, not just that it is.

You don't have to type the name exactly. Three kinds of search work, in this order of confidence:

| What you type | What it finds | Example |
|---|---|---|
| Text that is really in the name | The branches containing it | `report` → `feat/reporting-dashboard` |
| The letters in order, with gaps | Branches you're abbreviating | `frep` → `feat/reporting` |
| The name with one letter wrong, missing or extra | Branches you mistyped | `reportng` → `feat/reporting` |

Exact matches are always listed above approximate ones, so the branch you meant stays at the top.
The emphasis shows which kind of match you got: a solid run of colour means the text was really
there, and scattered letters mean you abbreviated.

Approximate matching needs something to go on, so it starts at **three characters** — one or two
letters find only what literally contains them. Typo tolerance starts at **five**, because below
that one wrong letter is too large a share of what you typed to tell a mistake from a different
branch.

Use **↑** and **↓** to move through the results and **Enter** to take the one you're on;
**Escape** closes the list and leaves your search text alone. Everything else you type goes into
the field, so searching never breaks stride. The **✕** at the end of the field clears the search
in one action.

Each row shows the branch name and, for a branch that only exists on a remote, which remote it
came from:

| Row | Meaning |
|-----|---------|
| `feat/login` | A local branch, ready to use. |
| `feat/reporting · origin` | Exists on `origin`, not yet on this machine. |
| `feat/login · in use by feat-login` | Already checked out in that worktree — not available. |
| `feat/login · in use by a hidden agent worktree` | Held by an assistant's worktree, which the sidebar hides by default. |
| `fix/olx · in use outside this app` | Held by a worktree the app doesn't manage — see below. |
| `main · in use by the project checkout` | The project's own current branch — not available. |

A branch that is in use elsewhere stays in the list — dimmed, and not selectable. It is shown
rather than hidden so you can read *where* it is in use instead of wondering why it is missing.

If nothing matches what you typed, the list says so. Clear the field or shorten your search to see
everything again.

The worktree folder is derived from the branch name — `feat/abc-123-login` becomes
`.claude/worktrees/feat-abc-123-login` — and the form shows it before you create.

> **Remote branches reflect your last fetch.** The app never contacts a remote here; it reads
> only what's already in your repository. Run `git fetch` yourself first if you want the list to
> be current.

### When the name you typed is already taken

If you're creating a new branch and the derived name already exists, the form asks what to do
instead of refusing:

- **Reuse branch** — create the worktree on the existing branch, with all of its commits intact.
  This is what you want to continue work started elsewhere.
- **Overwrite…** — discard that branch and start again from the current checkout. Asks for a
  second confirmation first, because it destroys commits.
- **Cancel** — change nothing. Your form entries are kept, so you can adjust and try again.

If the branch exists only on a remote, the choices are **Continue from `<remote>`** (creates a
local branch at the remote branch's tip and tracks it, so your next push goes back to the right
place) or **Start fresh** (an ordinary new branch at the current checkout, which will diverge
from the remote branch of the same name).

> **Overwrite cannot be undone from the app.** The old commits are no longer reachable from that
> branch. Git's reflog may still hold them for a while, but recovering from it is a manual git
> operation the app does not offer — treat overwrite as permanent.

### When a branch can't be used

A branch that is already checked out somewhere can't back a second worktree — git allows a branch
in only one place at a time. The app says so and identifies where it is. Neither reuse nor
overwrite is offered: open that location to continue there, or pick a different branch.

Where it is can be one of four places, and the message tells you which:

- **Another of your worktrees** — named by its folder, which is its row in the sidebar.
- **The project's own checkout** — the branch the project directory itself is on.
- **A hidden assistant worktree** — one of the app's own, but not currently listed. Turn on
  **Show agent worktrees** in the sidebar to see it.
- **A worktree outside this app** — one git knows about that this app doesn't manage: another
  tool's worktree directory (`.git-paw/worktrees/…` and the like), or a checkout in some unrelated
  folder. These never appear in the sidebar however you filter, so the message gives the **full
  path** instead of a folder name. `git worktree list` in the project directory shows all of them.

That last case is worth knowing about if a branch looks perfectly ordinary and is refused anyway.
It usually means a worktree you created outside the app — or a tool you used before this one —
still holds it. Removing that worktree (`git worktree remove <path>`) releases the branch.

## Managing a worktree (right-click)

Right-click a worktree in the sidebar to open its context menu:

- **Copy name** — copies the worktree's displayed name to the system clipboard, so it can be
  pasted into any other application (browser, chat, terminal, etc.). Useful because the sidebar
  label itself isn't a text field you can select from directly.
- **Rename** — changes only the name shown for the worktree in the sidebar. It does **not**
  rename the folder on disk or the git branch, and the type/issue tags are unaffected (they
  keep deriving from the branch). The custom name is remembered across app restarts. Clearing
  it is not needed — just rename again.
- **Delete** — removes the worktree completely. A confirmation dialog first spells out exactly
  what will be removed: the worktree directory under `.claude/worktrees/` and **all of its
  sessions** — this part is unconditional. If the worktree has an associated git branch, the
  dialog also offers an **"Also delete the branch"** checkbox, **checked by default** so
  confirming without changing anything behaves exactly as before (the branch is deleted along
  with everything else). Uncheck it to keep the branch — the directory and sessions are still
  removed, but the branch remains an ordinary branch in the repository, usable later (for
  example, to create a new worktree from it). Confirming terminates any running sessions in
  that worktree first, then removes the directory, sessions, and (unless unchecked) the branch —
  this cannot be undone. Cancelling removes nothing, including any change you made to the
  checkbox. (A worktree that is already missing/invalid can still be cleaned up this way.)

## Starting, switching, and closing sessions

- Select a valid worktree and use its **start session** action to launch a session. It appears as
  a sub-item and its terminal opens on the right.
- A worktree can host **multiple concurrent sessions** — start as many as you need for parallel,
  non-interfering tasks.
- **Switch** between sessions by selecting them in the sidebar. Background sessions keep running;
  only the displayed terminal changes.
- Right-click a session for **Close** and **Remove**:
  - **Close** stops its `claude` process and hides it from the sidebar. It does not reappear —
    including on a later restart, even though the underlying `claude` conversation itself still
    exists on disk. There is no way to bring a closed session back through the UI.
  - **Remove** permanently deletes the session's record, after a confirmation step. Unlike Close,
    there is no possible recovery path back into the sidebar either. Remove is only offered on a
    still-visible session — a closed session can't be removed separately, since it's already
    hidden.

Session labels come from `claude` itself (its session title); until a title is available a
placeholder is shown.

### Finding the session you are on

The session the terminal is showing is the **current** session, and the sidebar always says which
one that is:

- Its row is highlighted, and its name is set slightly heavier than the rows around it. The weight
  is there so the current session is still identifiable in a screenshot converted to greyscale, or
  by anyone who cannot separate the highlight from the hover shading beside it.
- The location holding it — a worktree, or **Default** — is **opened for you**, so the row is
  actually on screen rather than hidden inside a collapsed entry. This is what tells you where you
  are after switching projects, when every row would otherwise be collapsed.

You keep control of the panel:

- **Collapsing that row closes it for good**, for as long as you stay on the same session. It does
  not spring back open when a worktree is created or re-discovered in the background.
- **Nothing else is opened or closed on your behalf.** Other rows are left exactly as you had them.
- A row opened for you **stays** open when you move on to another session. Ceasing to be current
  takes away the highlight, never the open row.
- **Selecting a session yourself** highlights it and moves nothing — you were already looking at it.
- After a fresh start, no session is current until you pick one or start one, and the sidebar says
  so by highlighting nothing.

## The embedded terminal, resume & restart

- The terminal runs `claude` with its working directory set to the session's worktree, so each
  session is scoped to its own branch.
- Type in the input line and press **Enter** to send input to `claude`; its output streams above.
- If a session's `claude` process exits unexpectedly, it is **automatically restarted** (resuming
  the prior conversation via `claude --resume`). Repeated rapid failures stop the auto-restart and
  mark the session **failed** so you can retry manually.
- **Closing** the active project (or quitting the app) stops that project's session processes but
  keeps the sessions; reopening the project restores them and resumes them via `claude --resume`.
  **Switching** to another project does not stop them — see below.

> Requires the `claude` CLI on your `PATH`. If it is missing, starting a session reports an error.

## Switching to a regular terminal

Each session's terminal can also run a plain shell instead of `claude` — useful for running git
commands, scripts, or anything else scoped to that session's worktree without leaving the app.

- The toggle button in the terminal's bottom bar switches the pane between **AI CLI** mode
  (`claude`) and **Regular Terminal** mode (a plain shell). Its icon changes to show which mode
  you're currently in, and hovering it shows a tooltip naming the current mode and what pressing
  it switches to. This icon+tooltip is the single place to check which process your keystrokes
  are going to — there is no separate indicator, and it always reflects the current mode
  immediately after a switch.
- The shell starts with its working directory set to the session's worktree, same as `claude` —
  so `git status`, build scripts, and so on all run against the right branch.
- **Both processes keep running** while you switch — toggling away from AI CLI mode never stops
  or restarts `claude`, and toggling away from Regular mode leaves the shell running in the
  background. Switching back reattaches to whichever process was already there, exactly as you
  left it.
- If the shell exits (you typed `exit`, or it crashed), a **restart** control appears in the same
  bar so you can start a fresh one; unlike `claude`, the shell never restarts on its own.
- Switching to Regular mode never stops, restarts, or otherwise touches your `claude`
  conversation — even mid-turn. It keeps running in the background exactly as it was, including
  its own crash-auto-restart if it happens to exit while you're looking at the shell, and
  switching back reattaches to that same conversation with nothing lost.

### Running more than one Regular Terminal instance

A session isn't limited to a single Regular Terminal — you can open as many independent shell
instances as you need, side by side.

- Whenever a session is in Regular Terminal mode, an **open a new instance** button sits in the
  bottom bar next to the mode toggle. Press it (or use **Ctrl+Shift+T** / **Cmd+Shift+T** on
  macOS while the terminal has focus) to start another independent shell, scoped to the same
  session working directory as the first. The button is there even when only one instance is
  open, so you can always go from one to two.
- The keyboard shortcut only opens a new instance while the session is already showing a Regular
  Terminal — pressing it in AI CLI mode does nothing and does not switch modes.
- Each instance is a fully separate shell process: running a long command in one never affects
  the others, and closing or restarting one instance never touches its siblings or your `claude`
  conversation.
- Once a session has two or more open instances, a numbered switcher appears in the bottom bar
  (numbered in the order you opened them) — the currently active one is highlighted. Click any
  entry to bring that instance's shell to the front; the one you switch away from keeps running
  untouched in the background. With only one instance open, the switcher stays hidden — the
  terminal looks exactly as it did before this feature.
- The primary AI CLI/Regular toggle always shows whichever instance was last active when you
  switch back into Regular mode — not an arbitrary one.
- Each entry in the switcher has its own close button. Closing a background instance leaves
  everything else exactly as it was. Closing the instance you're currently looking at
  automatically brings up the next instance in the list (or the previous one, if you closed the
  last one in the list) — the pane is never left showing a closed instance. Closing your very
  last remaining instance falls back to AI CLI mode, same as today's single-terminal behavior.
- Each instance tracks its own running/exited state independently, including instances you're
  not currently looking at. If a background instance exits (or crashes) while you're viewing a
  different one, its switcher entry gains its own **restart** button — press it to start a fresh
  shell for just that instance, without switching to it first and without touching any sibling
  instance or your `claude` conversation.

## Sessions in the background

Switching to a different project **does not stop your sessions**. When you change the active
project — from the top-bar project switcher, the **Known projects** list, or the folder browser —
the project you leave keeps all its sessions running in the background: their `claude` processes
stay alive and their output keeps accumulating while you are away.

When you switch back, the project's sessions are still running and the session that was in the
foreground is shown again, exactly as you left it (other sessions stay in the background). Any
number of projects can hold running sessions at once.

If one of a background project's sessions exits unexpectedly while you are away, it is
auto-restarted under the same crash-loop guard as a foreground session. When you return to that
project a short **notice** tells you a background session was restarted — the state never changes
silently. Dismiss it with its **Dismiss** button.

> Background sessions live for as long as the app is running. Quitting the app stops every session;
> on the next launch they are restored and resume via `claude --resume` when selected.

## Colored, real-terminal output

The embedded terminal renders `claude`'s output like a real terminal, not as flat text:

- **Colors and styles** — ANSI foreground/background colors (the standard 16, bright, 256-color,
  and 24-bit truecolor) and text styles (bold, dim, italic, underline, strikethrough, and
  reverse/inverse) appear the same as in a standalone terminal.
- **Theme-aware defaults** — when output specifies no explicit color, the terminal's default
  text and background follow the app's light/dark theme and update when you switch themes. The 16
  ANSI colors use a fixed conventional palette so programs look as their authors intended.
- **Full-screen interfaces** — `claude`'s interactive UI and other full-screen (alternate-screen)
  programs redraw cleanly, with the cursor shown at its current position.
- **Focus** — starting or selecting a session automatically focuses its terminal (a colored
  border marks the focused terminal); you can also click the terminal to focus it.

## Interacting with the terminal

Start a session, or select one in the sidebar, and its terminal is focused right away — just type,
no click needed. (You can also click the terminal to focus it, e.g. after releasing focus.)
Keystrokes stream straight to `claude` as you press them, exactly like a standalone terminal:

- **Everything reaches `claude`**: printable characters, Enter, Backspace, Tab, arrow keys,
  Home/End/PageUp/PageDown, Insert/Delete, function keys, and control chords (Ctrl+C to
  interrupt, Ctrl+D, Ctrl+R, Ctrl+U, …). There is no "type a line and press Enter" box any more.
- **Paste** with the platform paste shortcut (Ctrl+Shift+V, or Cmd+V on macOS); the text is
  inserted into `claude` as input.
- **Select** text by dragging with the mouse (double-click selects a word, triple-click a line);
  the selection is copied to the clipboard automatically on release. **Copy** the current
  selection with Ctrl+Shift+C (Cmd+C on macOS); **middle-click** pastes.
- **Mouse-driven programs**: when the running program turns on mouse reporting, mouse clicks are
  forwarded to it; hold **Shift** while dragging to select text instead.
- **Keys route to `claude` only while the terminal is focused.** When focused, every key —
  including Escape and shortcuts the app would otherwise use — goes to `claude`; when not focused,
  those keys drive the application instead. Input is only delivered while the session's process
  is running (otherwise keystrokes are ignored and the header shows the session status).
- **Leaving focus**: press **Ctrl+Shift+E** (Cmd+Shift+E on macOS), click on empty app chrome
  outside the terminal, or use the **"⎋ release focus"** control in the terminal header. Releasing
  focus never interrupts the running session. (Clicking another session in the sidebar switches to
  it and focuses *its* terminal, rather than just leaving focus.)

## Sizing, resize & scrollback

- The terminal tells `claude` how many rows and columns are actually visible, so its interface
  lays out to fit. **Resizing** the window or dragging the sidebar reflows the terminal and the
  running interface to the new size.
- **Scroll** with the mouse wheel *or* a touchpad to move back through earlier output (up to the
  scrollback limit). Two-finger touchpad scrolling works the same as a wheel, including the fine,
  slow gestures that move less than a line at a time. When a full-screen program has taken over the
  mouse, the scroll is forwarded to it instead.
- A **scrollbar** appears on the right edge of the pane while you are scrolled back, showing where
  you are in the history. Drag it to move, or click the track to page. It hides itself once you
  return to the live bottom, so no scrollbar simply means there is nothing scrolled back.

> The scrollback limit is configurable — see [Settings](./settings.md).
