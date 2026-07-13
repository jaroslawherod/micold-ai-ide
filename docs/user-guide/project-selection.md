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
