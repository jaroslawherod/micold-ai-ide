# Micold AI IDE — Documentation

User-facing documentation for Micold AI IDE. Per the project constitution
(Principle VII), documentation ships in the same change as the code it describes and is
verified in CI.

## User Guide

- [Help & About](user-guide/help-about.md) — the application window, the Help toolbar entry,
  and the About dialog.
- [Project Selection & Workspace Management](user-guide/project-selection.md) — opening a
  project, the known-projects list, git repository marking, and renaming.
- [Appearance & Theming](user-guide/appearance-theming.md) — the Material Design layout, the
  light and dark themes, following the system preference, choosing a theme, and interface motion
  (dialog fades and the app's other animations).
- [Icons](user-guide/icons.md) — the shared Material icon set, where each icon appears, theming,
  licensing, and how to add a new icon.
- [Worktrees & Sessions](user-guide/worktrees-and-sessions.md) — opening a git project, the
  worktree sidebar (including how agent-created worktrees are hidden and how to reveal them),
  creating worktrees (on a new branch, or on one that already exists locally or on a remote),
  and running `claude` sessions in the embedded terminal
  (colored real-terminal rendering, interactive keyboard/mouse input, focus, resize, scrollback,
  and toggling a session's terminal to one or more independent plain-shell instances scoped to
  its worktree, switchable and individually closeable/restartable).
- [Settings](user-guide/settings.md) — the Settings dialog, the terminal scrollback limit, and
  environment-include (auto-picking up your shell environment for sessions, configuring or
  disabling it, and recovering from a failed script).

## The session service (daemon)

- [The Micold session daemon](daemon.md) — the background service that hosts your sessions so they
  survive closing (or crashing) the window: what survives and what doesn't, instant reattach and
  bounded scrollback, project/worktree operations running through the service, unattended crash
  supervision, one-window-per-project with deliberate takeover and half-open-connection detection,
  what a version mismatch looks like and how restart-and-resume behaves, surviving logout on Linux,
  and where the service logs.
