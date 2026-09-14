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
| [Environment](#environment) | Which AI CLI a session runs, whether Pi sessions report activity, and the script sourced before it starts |
| [Session service](#session-service) | Where sessions run, and what that service can reach |

<!-- media: settings-view-light -->

Everything you save lands in one file on this computer — see [Where settings are
stored](#where-settings-are-stored) for the path, and for what the app does when that file can't be
read.

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

### Default AI CLI

Which AI coding CLI a new session runs when you don't choose one for it.

- **The choices** are the CLIs you actually have installed where sessions run: on this computer, or
  in the container's image when the [session service](#session-service) runs in one. If only one is
  installed, that is the only option — and nothing else about starting a session changes for you.
- **A note under the field names any CLI that is missing** from there, so you know before you start a
  session rather than when it fails.
- **Default**: Claude Code.
- **Changing it affects new sessions only.** A session's CLI is fixed when the session is created
  and never changes afterwards, so sessions you already have keep running the CLI they started on.
- You can override it per session without touching this setting — see
  [Worktrees & sessions → Choosing which AI CLI a session runs](./worktrees-and-sessions.md#choosing-which-ai-cli-a-session-runs).

**If your default names a CLI that isn't installed, the app keeps it rather than quietly changing
it.** That is deliberate. A CLI can be missing for a moment — a `PATH` that hasn't loaded, an
upgrade in progress — and silently rewriting your preference would lose a choice you made without
saying so. The setting stays as you left it, and when you start a session the app tells you what is
missing and offers the CLIs that are available instead of substituting one.

### Show activity for Pi sessions

On by default. Pi has no built-in way to tell another program whether it is working or waiting
for you, so when you start a Pi session the app loads **a small component of its own into that
Pi process** to report it. That is what drives the session's working/idle badge.

What the component does, and all of what it does:

- It writes one line to a file the app watches each time Pi starts or finishes a turn, starts or
  finishes a tool, settles, or quits. Each line is the event's name and a timestamp — nothing
  else.
- It **never reads or transmits your conversation**: no prompts, replies, file contents or tool
  arguments, and it makes no network connection.
- It is loaded only into Pi sessions this app starts. It is not installed into Pi's own extension
  folders, so running `pi` yourself outside the app does not load it.

Pi gives every extension it loads **full permissions on your system** — the same as Pi itself.
The component uses none of that beyond appending to its one file, but if you would rather Pi load
nothing from this app, turn **Show activity for Pi sessions** off. Sessions started after that run
plain `pi`, and their badge reads **unknown**. That is the expected result of turning it off, not a
fault: the session works exactly as before, the app just has no way to see whether Pi is busy.

It is one setting for the whole app — there is no per-project or per-session copy — and it applies
to Pi sessions started after you save. Claude Code and Copilot sessions are unaffected either way.

### The environment a session starts in

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
  bug report, so the app will tell you when you're on one. Sessions run in this image, so it has to
  provide every AI CLI you want to use. The image this app publishes, and one built from this
  checkout, ships Claude Code, GitHub Copilot and Pi Coding Agent. If the running image lacks one, a
  note under the field names it. That note describes the image the service is running now, not an
  unsaved or not-yet-started reference: a new image is used from the next time the service starts.
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

- **Keep the service running when I'm signed out or away** — off by default. It answers one
  question, and changes two things.

  With it **off** (the default), the service stops when you sign out, and it also stops itself after
  30 continuous minutes with nothing connected. Reopening the application starts a fresh one; a
  session that was running comes back *resumable* rather than lost. See [when the service stops
  itself](../daemon.md#it-stops-itself-when-nobody-has-used-it-for-30-minutes).

  With it **on**, and only in a container: the sandbox is created with a restart policy the runtime
  honours on Linux, macOS and Windows alike, **and** the idle stop does not apply to it — a service
  you asked to keep running is not one to stop for being unused. Because both are fixed when the
  container is created, changing this marks the running sandbox as out of date and takes effect the
  next time it starts. See [keeping the sandbox
  running](sandboxed-daemon.md#keeping-the-sandbox-running).

  A service running **directly on this computer** cannot honour it on any platform — the app is the
  only thing that starts one, and nothing it starts outlives the session it was started from. The
  control says so rather than accepting a choice it cannot keep. Your sessions are still kept and
  come back resumable; only the running processes inside them stop.

## Where settings are stored

Everything on this page is saved to a single file on your own computer — nothing is sent anywhere:

| Platform | File |
| --- | --- |
| Linux | `~/.local/share/micold-ai-ide/settings.json` (or under `$XDG_DATA_HOME`) |
| macOS | `~/Library/Application Support/micold-ai-ide/settings.json` |
| Windows | `%APPDATA%\micold-ai-ide\data\settings.json` |

The file survives restarts and package upgrades. You can read it, and you can copy it to another
machine, but you don't have to edit it by hand — Settings writes every value it holds.

### If your settings can't be read

Two things can go wrong with that file, and the app tells you about both rather than quietly
starting over. Neither one throws your settings away.

**"Your settings could not be read, so defaults are in use."** — shown when the app starts. The
file was there but the app couldn't make sense of it, so it opened with the defaults rather than
refusing to start. If the file was unreadable because its contents were damaged, the message also
names where the original was kept:

> Your settings could not be read, so defaults are in use. The unreadable file was kept as
> `…/settings.json.bak`.

That `.bak` file is the damaged original, untouched. Open it in any text editor to read the values
back out — your container image reference, your startup script path, whatever you'd rather not type
again — then set them in Settings and press **Save**. Once you save, the app writes a fresh
`settings.json` and the message stops appearing. If you don't need anything out of the `.bak`,
you can delete it.

**"Couldn't save your settings: the settings file could not be read, and saving now would replace
it with defaults."** — shown when you press **Save**. Here the file is still on disk and still
intact; the app just couldn't read it this time, usually because the file's permissions or its
folder changed underneath it. Saving would have written the defaults over a perfectly good file, so
the save is refused instead. Your change is not lost — it's still in the form, and the values on
disk are still the ones you set earlier.

To clear it: check that the file listed above exists and that your user account can read and write
it, then press **Save** again. Every settings write — successful or refused — is recorded in the
log, so a save you can't explain has a line to point at (see [Finding the logs and recent
errors](../daemon.md#finding-the-logs-and-recent-errors)).
