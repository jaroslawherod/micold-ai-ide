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
- Closing or switching the active project stops that project's session processes but keeps the
  sessions; reopening the project restores them and resumes them via `claude --resume`.

> Requires the `claude` CLI on your `PATH`. If it is missing, starting a session reports an error.
