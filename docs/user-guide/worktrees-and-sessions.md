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

Click **add** in the sidebar header to open the New worktree form:

1. **Type** — pick a Conventional-Commits type (`feat`, `fix`, `chore`, `docs`, …).
2. **Ticket** — optional reference (e.g. `ABC-123`). Leave blank to omit it.
3. **Name** — a short description (e.g. `login page`).

The form shows the derived names before you create:

- **Directory**: `.claude/worktrees/${type}-${ticket}-${name}`
- **Branch**: `${type}/${ticket}-${name}`

For example, `feat` + `ABC-123` + `Login page` creates the branch `feat/abc-123-login-page` and a
worktree at `.claude/worktrees/feat-abc-123-login-page`. With no ticket, `chore` + `cleanup` gives
`chore/cleanup`. Illegal characters in the ticket or name are automatically simplified (slugified).

Creating a worktree makes the new git branch and worktree for you — no manual git commands. If the
name collides with an existing worktree or branch, creation is blocked with a message. If anything
fails partway, the app rolls back so no half-created branch or directory is left behind.

If the project uses git submodules, they're fetched automatically as part of creating the
worktree — including submodules nested inside other submodules — so the new worktree is ready to
use immediately, with no extra `git submodule` commands to run yourself. Projects without
submodules are unaffected. While a worktree is being created, the form shows a "Creating
worktree…" state — this can take a little longer than usual for a repository with submodules to
fetch, so it's expected rather than a sign the app is stuck.

If fetching a submodule fails (for example, a network problem or a private submodule remote you
aren't authenticated against), the worktree is not created — the branch and directory are rolled
back the same way any other creation failure is, and the error names the submodule that failed
and why, so you can fix the problem and try again.

> Naming formats are fixed in this version and are intended to become configurable later.

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
  what will be removed: the worktree directory under `.claude/worktrees/`, **all of its
  sessions**, and its **git branch**. Confirming terminates any running sessions in that
  worktree, then deletes the directory, sessions, and branch — this cannot be undone.
  Cancelling removes nothing. (A worktree that is already missing/invalid can still be cleaned
  up this way.)

## Starting, switching, and closing sessions

- Select a valid worktree and use its **start session** action to launch a session. It appears as
  a sub-item and its terminal opens on the right.
- A worktree can host **multiple concurrent sessions** — start as many as you need for parallel,
  non-interfering tasks.
- **Switch** between sessions by selecting them in the sidebar. Background sessions keep running;
  only the displayed terminal changes.
- **Close** a session with its close action; this stops its `claude` process and removes it.

Session labels come from `claude` itself (its session title); until a title is available a
placeholder is shown.

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
