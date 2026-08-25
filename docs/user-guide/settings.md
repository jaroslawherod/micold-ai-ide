# Settings

Open **Settings** from the overflow menu (the three-dots button) in the top toolbar. It fills the
main area, with a rail down the left listing four sections. The app bar and the connection strip
stay where they are, so the way back out is always in view.

Editing is one form across all four sections: switching sections never discards what you typed, and
**Save** applies every section at once. If a value is rejected, Settings jumps to the section
holding it and marks the field — press **Cancel**, or Esc, to leave without saving anything.

| Section | What it holds |
| --- | --- |
| [Appearance](#appearance) | The theme |
| [Terminal](#terminal) | The embedded terminal's scrollback limit |
| [Environment](#environment) | The script sourced before each session starts |
| [Session service](#session-service) | Where sessions run, and what that service can reach |

## Appearance

**Theme** — follow the system, or pin light or dark. Following the system switches with it while
the app is running.

The app bar's mode button cycles the same setting, for when you want it in one press.

## Terminal

**Scrollback lines** controls how many lines of earlier output each session's terminal keeps for
scrolling back through (see [Worktrees & sessions → Sizing, resize &
scrollback](./worktrees-and-sessions.md)).

- **Default**: 10,000 lines.
- **Range**: 100 – 1,000,000 lines. Values outside the range (or non-numeric input) are
  rejected with a message and not saved.
- The value is **saved on your machine** and restored the next time you open the app.
- A changed limit applies to sessions started **after** the change; already-running terminals
  keep their current buffer.

## Environment

By default, every session's AI CLI process and regular-terminal process automatically pick up
your normal shell environment — PATH additions from version managers (nvm, pyenv, rbenv),
exported API keys, proxy settings, and anything else your shell's startup file sets. This is done
by actually running that startup file in a real, disposable shell process and capturing what it
changes — not by parsing its text — so conditionals, sourced sub-files, and version-manager init
blocks all resolve correctly.

- **Default**: on, sourcing `~/.bashrc` on Linux/macOS (via bash) or your PowerShell profile on
  Windows.
- Both the AI CLI process and the regular-terminal process for a session see the identical set of
  resolved variables.
- Resolution runs **per project directory**, not once for the whole app: if your startup file uses
  a version manager (mise, asdf, nvm, pyenv, rbenv, …) whose `PATH` additions depend on which
  project you're in, each project's own directory-specific additions are picked up correctly —
  the same way they would be in a regular terminal opened in that project.

This section holds three fields:

- **Source a script before each session** — turn environment-include off entirely (no script is
  sourced) or back on. Turning it off keeps the path, so re-enabling it doesn't mean typing it
  again.
- **Script path**: the file to source. Any path is accepted — whether it resolves to a usable
  script is only discovered when it's actually used, never rejected at save time.
- **Timeout (seconds)**: how long sourcing may run before being treated as hung. **Default**: 10
  seconds. **Range**: 1 – 60 seconds; out-of-range or non-numeric input is rejected with a message
  and not saved (same as the scrollback field).

A saved change takes effect on the next session or terminal launch — no app restart needed.

**Persistence**: only the enabled flag, script path, and timeout are ever saved to disk. The
variables the script resolves — and any diagnostic text captured while troubleshooting a failure
— are held in memory for the running app only and are never written to your settings file, since
they may include secrets (e.g. exported API keys).

### If the script fails

A missing, broken, or hanging script never blocks or fails opening a session — the session opens
normally with whatever environment is otherwise available. The most recent attempt's outcome is
shown at the bottom of this section whenever it didn't succeed — since resolution runs per project
directory, this reflects whichever directory was most recently (re-)resolved (typically your active
project, or the one you just restarted a session in), not necessarily every project you have open:

- **Script not found** — the configured path doesn't exist.
- **Exited with an error** — the script ran but failed; the script's own output is shown verbatim
  underneath, to help you see what went wrong.
- **Timed out** — sourcing didn't finish within the configured timeout and was abandoned.

To recover once you've fixed the script: use the existing **restart** control on the affected
session's terminal (shown whenever that session's process isn't running) — this re-sources the
script fresh and clears the failure note, without needing to restart the whole app. Saving Settings
(even without changing any value) also triggers a fresh re-source.

## Session service

Everything about the process that actually runs your sessions is here, because it is one decision:
sharing your SSH agent means nothing when the service is a plain process on this computer, and means
a great deal when it is a container.

**Where sessions run** — directly on this computer (the default), or inside a container that sees
only your registered projects. Takes effect the next time the application starts. See [Running the
session service in a container](sandboxed-daemon.md) for what changes, what it can and cannot
reach, and how to work offline.

The container settings stay visible and editable whichever placement you pick, and are kept if you
switch back — configuring the container and then trying the host process first doesn't mean setting
it up again.

### Container

- **Container runtime** — Docker (the default) or Podman.
- **Image source** — pull from a registry, load from a local archive (the fully offline path), or
  build from this checkout.
- **Image reference** — a digest or an exact tag. A moving tag like `:latest` can't be named in a
  bug report, so the app will tell you when you're on one.
- **Image file** — the archive to load, when the image comes from a file.

### Credentials

The container starts with **none** of your credentials, and upgrading the app never opts you in.
Each share is separate, and only what you tick is passed in:

- **Git configuration** — `~/.gitconfig`, read-only: your commit identity.
- **SSH agent** — the agent's socket. The socket, never the keys themselves.
- **Git credentials** — the git credential helper's store.
- **AI CLI sign-in** — the AI CLI's own authentication material.

While any of these is on, the section lists exactly which ones by name, and the rail marks
**Session service** with a *Sharing* badge — so you can tell at a glance, from any section, that
something is being shared.

### Sessions

- **Keep sessions running after I sign out** — sessions outlive your sign-out. In a container this
  is honoured on Linux, macOS and Windows alike; the host-process placement manages it only on
  Linux.
