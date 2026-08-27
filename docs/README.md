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
  light and dark themes, following the system preference, choosing a theme, interface motion
  (dialog fades, easing, and the app's other animations), and how notifications appear one at a
  time and clear themselves.
- [Icons](user-guide/icons.md) — the shared Material icon set, where each icon appears, theming,
  licensing, and how to add a new icon.
- [Worktrees & Sessions](user-guide/worktrees-and-sessions.md) — opening a git project, the
  worktree sidebar (including how agent-created worktrees are hidden and how to reveal them),
  creating worktrees (on a new branch, or by searching for one that already exists locally or on
  a remote),
  and running AI CLI sessions (`claude` or `copilot`) in the embedded terminal
  (colored real-terminal rendering, interactive keyboard/mouse input, focus, resize, scrollback,
  and toggling a session's terminal to one or more independent plain-shell instances scoped to
  its worktree, switchable and individually closeable/restartable).
- [Settings](user-guide/settings.md) — the Settings view: appearance, the terminal scrollback
  limit, environment-include (auto-picking up your shell environment for sessions, configuring or
  disabling it, and recovering from a failed script), and **Session service** — where sessions run,
  which credentials the service may reach, and the resource limits it runs under.
- [Running the session service in a container](user-guide/sandboxed-daemon.md) — turning the
  container placement on, which runtimes work (and how Podman differs), what the container can and
  cannot see, credentials, limits, network posture, working offline, what happens across restarts
  and reboots, and what to do when it will not start.

## Development

- [Client architecture](development/architecture.md) — where a feature lives and why one module
  holds its types and the functions over them; how to add a floating surface (one module, one
  registration line, no central match to extend) and how to add a capability (declare the trait,
  write the fake, choose the real implementation once); and the read/write asymmetry across
  features — why a feature may read any state and write only its own, what an `Outcome` is for, and
  the guard tests that hold each of those lines rather than trusting them.
- [The component library](development/component-library.md) — the two rendering layers, the rule
  that feature modules compose components rather than styling widgets, how that rule is enforced in
  CI, what to do when adding a component, and how to build a picker — searching or choosing — on the
  shared foundation both of the existing ones stand on.
- [The layout snapshot](development/layout-snapshot.md) — the three checks that pin *where things
  are*: the geometry fixture, the text-overflow gate and the containment invariant. What each one
  catches, what none of them do (colour, pixels, scrolling, mid-animation, and the typeface until
  018 ships), the exemptions currently in force, and how to accept an intended layout change.
- [The component showcase](development/component-showcase.md) — the development-only gallery of every
  component in every posed state, in both schemes, on one page (`mise run showcase`): how to launch it,
  how to add a component to it, what each completeness failure means, and what it deliberately does not
  cover.
- [The CI pipeline](development/ci-pipeline.md) — why a change that touches only documentation or
  specs skips the build entirely, where the documentation set is declared, the single status check
  the default branch requires and the two properties that keep it honest, how to force a full run,
  and what to do when the pipeline surprises you.

## The session service (daemon)

- [The Micold session daemon](daemon.md) — the background service that hosts your sessions so they
  survive closing (or crashing) the window: what survives and what doesn't, instant reattach and
  bounded scrollback, project/worktree operations running through the service, unattended crash
  supervision, one-window-per-project with deliberate takeover and half-open-connection detection,
  what a version mismatch looks like and how restart-and-resume behaves, surviving logout on Linux,
  where the service logs, and **where the service runs** — the placement model (on this computer, in
  a container, or reserved for remote), why the container is reached over authenticated loopback TCP
  rather than a socket, and who answers "is this a git repository?" once it is.
