# Installing on Windows

This page covers getting Micold AI IDE onto a Windows PC and running it: which file to download, what
the installer does, how to upgrade and remove it, and what the package cannot do for you.

## Before you start

Micold AI IDE runs *your* tools rather than bundling them. Install these first:

- [Git for Windows](https://git-scm.com/download/win) — projects and worktrees are git repositories.
- [Claude Code](https://docs.claude.com/en/docs/claude-code) or
  [GitHub Copilot CLI](https://docs.github.com/en/copilot), signed in and on your `PATH`.

Windows 11, or Windows 10 version 1809 or later, is required.

## Download

Each release carries one installer per processor architecture:

| Your PC | Download |
|---|---|
| Intel or AMD (`x64`), most PCs | [micold-ai-ide-{{MICOLD_VERSION}}-x64-setup.exe](https://github.com/jaroslawherod/micold-ai-ide/releases/download/{{MICOLD_TAG}}/micold-ai-ide-{{MICOLD_VERSION}}-x64-setup.exe) |
| Arm (`ARM64`), such as Snapdragon laptops | [micold-ai-ide-{{MICOLD_VERSION}}-arm64-setup.exe](https://github.com/jaroslawherod/micold-ai-ide/releases/download/{{MICOLD_TAG}}/micold-ai-ide-{{MICOLD_VERSION}}-arm64-setup.exe) |

Not sure which you have? Open **Settings → System → About** and read **System type**: *x64-based
processor* means x64, *ARM-based processor* means ARM64. Picking the wrong one does no harm — the
installer says the PC is not supported and stops before changing anything.

## Install

1. Run the downloaded `.exe`.
2. **Windows protected your PC** appears. This is SmartScreen, and it appears because the installer
   is not code-signed: the project has no signing certificate, so Windows cannot attribute the file to
   a publisher. Click **More info**, then **Run anyway**.
3. Follow the wizard. It asks for no administrator rights and shows no UAC prompt — Micold AI IDE
   installs for your account only.
4. On the last page, tick **Launch Micold AI IDE** if you want to start it straight away.

What that leaves on your PC:

- the application and its session service, in `%LOCALAPPDATA%\Programs\Micold AI IDE`;
- a **Micold AI IDE** entry in the Start menu, and a desktop shortcut if you ticked that option;
- an entry in **Settings → Apps → Installed apps**.

Nothing is added to your `PATH`, and nothing is set to start when you sign in. The application starts
its session service itself the first time it needs one ([The Micold session daemon](../daemon.md)).

## Upgrading

Download the newer installer and run it; there is no need to uninstall first. If the application is
open, the installer asks you to close it. Its ready page says **Running sessions will be stopped**:
the session service is stopped once you continue, so any open sessions end at that point.

Your projects and settings carry over, and **Installed apps** still shows a single Micold AI IDE entry.

Running the installer for the version you already have repairs the install: it puts back the program
files and the Start menu entry.

## Removing

Open **Settings → Apps → Installed apps**, find **Micold AI IDE**, and choose **Uninstall**. As with an
upgrade, a running application and session service are stopped first.

Uninstalling removes the program files, the shortcuts, the Installed apps entry and the session
service's runtime folder, `%LOCALAPPDATA%\micold-ai-ide\run`.

### What uninstall keeps

Your settings and session data stay, so reinstalling picks up where you left off:

- `%APPDATA%\micold-ai-ide`
- `%LOCALAPPDATA%\micold-ai-ide\data`

For a full reset, delete both folders by hand after uninstalling. Your projects are never touched —
they live wherever you cloned them.

## Limits

- **Smart App Control.** If Smart App Control is on (Windows 11 → **Windows Security → App & browser
  control**), it blocks the unsigned installer outright. Unlike SmartScreen, it has no per-app
  override: the only way through is turning Smart App Control off, and Windows cannot turn it back on
  without a reset. On such a PC, build Micold AI IDE from source instead
  ([Build from source](../install.md#build-from-source)).
- **Managed PCs.** A work PC with an AppLocker or Windows Defender Application Control policy may
  refuse the installer. Ask your IT department, or build from source.
- **Sessions end when you sign out.** Closing the window leaves your sessions running, but logging out
  of Windows stops the session service and every session with it. The one exception is running the
  service in a container ([Running the session service in a container](sandboxed-daemon.md)); see
  [The Micold session daemon](../daemon.md) for why.
- **One service per account.** Each Windows account that uses Micold AI IDE runs its own session
  service, and no other account on the PC can connect to it.
- **Unsupported architecture.** Each installer runs only on its own architecture; on any other PC it
  shows the installer's own "not supported" message and changes nothing.
