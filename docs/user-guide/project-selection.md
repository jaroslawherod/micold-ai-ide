# Project Selection & Workspace Management

Micold AI IDE lets you choose a project to work on and set it as your current working space.
This guide covers opening projects, reopening them after a restart, how git repositories are
marked, and renaming a project's display name.

> The application never modifies your folders on disk. Browsing and git detection are read-only,
> and renaming a project changes only the name Micold shows — never the folder itself.

## Opening a project

When you launch Micold AI IDE for the first time, the main window shows an **empty state**
inviting you to open a project. Click **Open a project** to bring up the project selector.

The selector is an in-app folder browser:

- The current folder's path is shown at the top; use **↑ Up** to go to the parent folder.
- Subfolders are listed below — click one to browse into it.
- When you are inside the folder you want to use, click **Open this folder**.
- Click **Cancel** (or press **Esc**) to close the selector without choosing.

Any folder can be a project — it does **not** need to be a git repository. When you open a
folder, Micold creates a project whose name defaults to the folder's name and makes it your
**active working space**. The active project's name is shown in the main window.

To switch to a different folder later, click **Open another project** and choose again. Only
one project is active at a time; opening another replaces the current one.

## Reopening projects

Micold remembers every project you open in a **known-projects list** that is saved locally
and survives restarts — so you don't have to browse the filesystem again. The list appears
in the main window under **Known projects**. Each entry shows the project's name; the
currently active project is marked with a ● dot.

To reopen a project, click **Open** next to it. It becomes your active working space
immediately, without opening the folder browser. Micold also remembers which project was
active last time.

The list is stored on your own machine (no account, no network required — Micold works
fully offline). Opening a folder that is already in the list simply reactivates the existing
entry; it never creates a duplicate.

## Switching projects from the top bar

Next to the menu button in the top bar is the **project switcher** — a button showing a folder
icon and the name of your active project, and the quickest way to change projects without
opening the folder browser or scrolling the main-window list. Click it to drop down a panel
listing your known projects. Each row shows:

- the project's name, with the **active** project marked;
- a **running** count when the project has terminal sessions running in the background
  (for example, "2 running") — so you can tell at a glance where your live work is;
- an **unavailable** badge for folders that are missing on disk (these cannot be selected).

Click any available project to switch to it in a single step. The last row, **Add project…**,
opens the folder browser so you can add a project that isn't in the list yet. The switcher
complements the **Known projects** list in the main window and the folder browser — all three
still work.

**Right-click** any project row for a context menu with **Forget project** — see
[Forgetting a project](#forgetting-a-project) below. (The **Add project…** row is an action, not
a project, so it has no context menu.)

Switching projects this way **does not stop your running terminal sessions**. The project you
leave keeps its sessions running in the background, and returning to it restores them exactly
as you left them. See
[Worktrees & Sessions → Sessions in the background](worktrees-and-sessions.md) for details.

## Unavailable projects

If a project's folder has been deleted, moved, or renamed on disk since you added it, Micold
does not crash. The project stays in the list but is clearly marked **(unavailable)**, and
its **Open** button is disabled — you cannot activate a folder that is no longer there. If
you later restore the folder, restart Micold (or reopen it) and the project becomes
available again.

## Git repositories

While browsing in the selector, folders that are git repositories are marked with a **git**
badge, so you can tell version-controlled folders apart at a glance. The same badge appears
next to git projects in the known-projects list.

A folder is treated as a git repository when it directly contains a `.git` entry. This is
detected when the folder is inspected, so the badge reflects the folder's state at that
moment. You can still open any folder as a project — being a git repository is not required.

## Renaming a project

You can give a project a friendlier name. In the **Known projects** list, click **Rename**
next to a project, type the new name, and click **Rename** (or press **Enter**). Press
**Esc** or **Cancel** to leave it unchanged.

Renaming changes only the name Micold displays — it **never** renames, moves, or otherwise
touches the folder on disk. The new name is saved and persists across restarts. Names do not
have to be unique: two projects can share the same display name and remain distinct by their
folder path. A name that is empty or only whitespace is rejected, and the previous name is
kept.

## Forgetting a project

When you no longer want a project in your list, you can **forget** it, from either of two places:

- **From the Known projects list** — click **Forget** next to a project.
- **From the project switcher** — open the switcher in the top bar, **right-click** the
  project's row, and choose **Forget project**. The menu opens at your pointer, and stays fully
  on screen even when you right-click near the window's edge.

Both routes do exactly the same thing. Micold asks you to confirm first, because forgetting
permanently discards what the app remembers about that project — its custom name, any
worktree-name overrides, and its session records.

**Forgetting never deletes anything on disk.** The folder, its files, and any git worktrees are
left completely untouched — only Micold's own remembered entry is removed. (Deleting a worktree is
a separate, explicitly destructive action; see
[Worktrees & Sessions](worktrees-and-sessions.md).)

On the confirmation dialog:

- Click **Forget** to remove the project. It disappears from the list immediately, and the removal
  is saved right away — so the project does not come back the next time you launch Micold.
- Click **Cancel** (or press **Esc**) to keep the project; nothing changes.

A few details worth knowing:

- **Running sessions are stopped.** If the project has terminal sessions running (including in the
  background), forgetting stops them so nothing keeps running for a project Micold no longer
  tracks. When there are running sessions, the confirmation tells you how many will be stopped.
  Their worktree folders and files are **not** deleted — only the running processes end.
- **Forgetting the active project** clears your active working space: afterward no project is
  active until you open or reopen one. If it was your only project, you return to the first-run
  **empty state**.
- **Unavailable projects can be forgotten too.** Forget is the way to clear out a stale entry whose
  folder is gone from disk — unlike **Open**, the **Forget** button stays enabled for
  **(unavailable)** projects.
- **Re-opening a forgotten folder starts fresh.** If you later open the same folder again, it comes
  back as a brand-new entry with the default (folder) name — the custom name and other remembered
  details from before are gone. (Any AI CLI conversations still present in the folder's worktrees
  on disk may be rediscovered, exactly as they would be for any folder you open that already
  contains conversations — that is the folder's current contents, not Micold remembering the
  forgotten entry.)
