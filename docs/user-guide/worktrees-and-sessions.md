# Worktrees & Sessions

Micold AI IDE organizes your work into **worktrees** (isolated git branches checked out under
your project) and **sessions** (interactive `claude` runs inside a worktree). The left sidebar
shows worktrees at the top level and their sessions as sub-items; the right side hosts the
embedded terminal for the active session.

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
  is shown flagged as **unavailable** — you cannot start new sessions on it until it is resolved.

### Resizing and hiding the sidebar

- **Resize**: drag the thin handle on the sidebar's right edge to make it wider or narrower.
- **Hide**: click the **hide** button (panel-collapse icon) in the sidebar header, next to
  **add worktree**. The sidebar collapses to a thin strip.
- **Show**: click the **show** button (panel-open icon) on the collapsed strip to bring it back.

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

> Naming formats are fixed in this version and are intended to become configurable later.
> Removing a worktree is not yet available.

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
- **Focus** — click the terminal to give it focus (a colored border marks the focused terminal).

## Interacting with the terminal

Click the terminal to focus it (a colored border appears), then type — keystrokes stream
straight to `claude` as you press them, exactly like a standalone terminal:

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
- **Leaving focus**: press **Ctrl+Shift+E** (Cmd+Shift+E on macOS), click anywhere outside the
  terminal, or use the **"⎋ release focus"** control in the terminal header. Releasing focus never
  interrupts the running session.

## Sizing, resize & scrollback

- The terminal tells `claude` how many rows and columns are actually visible, so its interface
  lays out to fit. **Resizing** the window or dragging the sidebar reflows the terminal and the
  running interface to the new size.
- **Scroll** the mouse wheel to move back through earlier output (up to the scrollback limit).
  When a full-screen program has taken over the mouse, the wheel is forwarded to it instead.

> The scrollback limit is configurable — see [Settings](./settings.md).
