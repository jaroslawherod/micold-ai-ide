# Installing on macOS

This page covers getting Micold AI IDE onto a Mac and running it: what to download, what installing
it involves, what the application starts on your behalf, and which permissions macOS will ask you
about.

## Which Mac, which macOS

**Minimum macOS version: 15.0** (Sequoia). The project supports the two most recent major releases
of macOS, which today means Sequoia and Tahoe.

Any Mac from the last several years will do, of either kind: there is one download and it runs
natively on both Apple silicon (M1 and later) and Intel. You are never asked which you have, and no
compatibility layer is involved on either.

If you open the application on a Mac below the floor, macOS itself stops it and says so — *"You
can't use this version of the application … with this version of macOS"*, naming the version you
need. That check is the operating system's, made from the application's own declaration, so it
happens before anything of ours runs and it cannot be got wrong by a partly working install.

## Download

Releases are on the project's releases page. The macOS download is a single disk image:

```text
MicoldAIIDE-<version>-universal.dmg
```

There is one file, not one per Mac. `universal` means the application contains native code for both
Apple silicon and Intel Macs, so there is no choice to get wrong — macOS picks the right half when
it launches.

Download it with a browser rather than with `curl`. This is not a preference: macOS attaches a
quarantine flag to browser downloads, and the first-launch experience — including the block
described below — depends on it. A file fetched with `curl` behaves differently from the one your
users will actually have.

## Install

1. Open the `.dmg`. A window appears with the application on one side and a shortcut to your
   `Applications` folder on the other.
2. Drag **Micold AI IDE** onto **Applications**.
3. Eject the disk image.

That is the whole installation. Nothing is written outside `/Applications` and your own home
directory, no installer runs, and nothing asks for an administrator password.

**Do not run the application from the disk image.** If you double-click it inside the mounted image
instead of copying it first, the application shows a single screen telling you so and does not
continue. Running from a mounted image half-works in ways that are hard to diagnose: the image can
be ejected out from under a running application, and macOS may relocate a quarantined copy to a
randomised read-only path where nothing you do persists. Copying it to `Applications` first is the
supported way to run it, and the only one.

## Launch

Open it from Launchpad, Spotlight, or the `Applications` folder, the way you would any other
application.

The first launch is different from every launch afterwards: macOS blocks it. This is expected, it is
not a sign that anything is wrong with the download, and one gesture clears it — see
[The first launch](#the-first-launch) below for exactly what you will see and what to do about it.
Every launch after that is ordinary and silent.

## The first launch

Double-clicking the freshly installed application does not open it. macOS shows a dialog roughly
like this one — the exact wording moves between macOS versions, the meaning does not:

> **"Micold AI IDE" Not Opened**
>
> Apple could not verify "Micold AI IDE" is free of malware that may harm your Mac or compromise
> your privacy.
>
> [ Move to Trash ]  [ Done ]

Neither button is the one you want. Click **Done**, then:

1. Open **System Settings › Privacy & Security**.
2. Scroll to the **Security** section. It now says *"Micold AI IDE" was blocked to protect your
   Mac.*
3. Click **Open Anyway**, and confirm with your password or Touch ID.

The application opens. That is the whole gesture: it happens once, in the operating system's own
interface, and no terminal is involved.

Do it in that order — dismiss the dialog first, then go to System Settings. The **Open Anyway**
button only appears there for the most recent block, so going to System Settings before you have
tried to open the application will show you nothing to click.

There is a `xattr -d com.apple.quarantine` incantation that does the same thing from a terminal. It
works, and it is not the documented route: the route above is the one that works for everyone, on
every supported version of macOS, without a terminal. If you already know that command you do not
need this page.

### Why macOS blocks it

The application is signed, but *ad hoc* — the signature proves the code has not been altered since
it was built, and names no developer. That is what macOS is telling you: not that it found anything
wrong, but that it cannot attribute this application to a registered developer, because the project
has no Apple Developer account and the release process holds no signing certificate.

The alternative to trusting the download is not to download it. Build the application from source
yourself — the repository's `README.md` has the instructions — and no block appears, because a copy
you built never carried the download mark that triggers it.

If the project ever obtains a Developer ID and notarizes its releases, this section stops applying:
the first launch becomes silent, like any other application's. Nothing else about installing it
changes.

### Why it comes back after an update

The block is attached to the *copy*, not to the application. macOS marks anything a browser
downloads, and clearing the block clears that mark on the file you cleared it on.

A new release is a new download and therefore a new copy, so it arrives with the mark set and is
blocked once, exactly as the first one was. Repeat the gesture above. It is not a regression, and it
does not mean the previous approval failed to stick.

### Why clearing it on the disk image does not help

If you double-click the application while it is still inside the mounted disk image, you can clear
the block *there* — on a copy that lives on a read-only volume you are about to eject.

The copy you then drag to `Applications` is a different file, still carrying the download mark, and
is blocked again. It looks as if the approval was ignored. It was not; it was spent on a copy that
no longer exists.

So install first, eject the disk image, and clear the block on the copy in `Applications`. That
copy is the one you keep. (The application itself will tell you if you get this wrong: run from the
disk image, it says so and refuses to pretend to be installed, rather than letting you work in a
window whose contents disappear on eject.)

## What starts automatically

Micold AI IDE runs your terminal sessions in a separate background process — the *session daemon* —
rather than inside the window. It is what keeps a session's shell alive while you switch projects
or close and reopen a window.

You do not install, start, or configure it. The first time the application needs it, it starts the
daemon itself, from the copy that ships **inside** the application bundle:

```text
Micold AI IDE.app/Contents/MacOS/
├── micold-ai-ide     the application you launch
└── micold-daemon     the session daemon it starts
```

Both files are part of the application. The application finds the daemon beside itself, which is
why the bundle must be copied whole rather than by picking a file out of it.

The daemon listens on a socket in your home directory:

```text
~/.micold/run/d.sock
```

It is owned by you and readable by nobody else. Nothing listens on a network port, and nothing is
sent off the machine.

## What happens when you log out

Closing the window leaves your sessions running. That is what the session daemon is for: your shells
keep going, and reopening the window finds them where you left them.

Logging out is different. On macOS, sessions survive closing the window but not logging out when the
session service runs directly on your computer; running it in a container is the supported way to
survive logout there.

The reason is that macOS ends what your login session was running when that session ends, and there
is no way for an ordinary application to opt out of that — the mechanisms that exist for it need
privileges this application does not ask for and would not be right to ask for, so it does not
pretend otherwise. Nor does it on any other platform: a session service running directly on your
computer does not outlive a logout on Linux or Windows either.

What that means in practice: log out or restart, and the shells your sessions were running are gone.
The sessions themselves are not. They come back listed as interrupted, with their history, and one
click resumes each one.

If you need more than that, run the session service in a container instead of on the computer — the
setting is in **Settings › Session service**. The container runtime keeps it going across logout and
reboot, because the runtime is itself a service the system already keeps running. See
[the session service guide](../daemon.md#surviving-logout-run-the-service-in-a-container) for
what that placement changes.

## Updating to a new version

Updating is the same gesture as installing. Download the new disk image, drag **Micold AI IDE** onto
**Applications**, and let Finder replace what is there.

Finder offers **Replace** or **Keep Both**. Choose **Replace**. Keeping both leaves two copies with
the same name in different places, and the one you launch from Spotlight afterwards is whichever it
happens to find first — which is a confusing way to discover that you are still running the old
version.

Everything you care about survives, because none of it is inside the application:

| What | Where it lives |
|---|---|
| Your projects and the sessions in them | `~/Library/Application Support/micold-ai-ide/` |
| Your settings | `~/Library/Application Support/micold-ai-ide/settings.json` |
| Your conversations | with the AI CLI that created them (Claude Code keeps its own under `~/.claude`) |

The new copy is a new download, so its first launch is blocked once, exactly as the first one was —
see [Why it comes back after an update](#why-it-comes-back-after-an-update).

### "The session service is a different version"

The session daemon from the previous version may still be running when you launch the new
application: it belongs to your login session, not to the bundle you just replaced, so replacing the
bundle does not stop it. When that happens the application says so, at the top of the window:

> **The session service is a different version**
>
> This app speaks contract v9; the running service (micold-daemon 0.10.0) speaks v8. Restart the
> service to match — running processes stop, but your sessions are preserved and resumable.
>
> [ Restart service ]

Click **Restart service**. That stops the old daemon and the application starts a matching one on
its next connection, which happens by itself. There is nothing to find in Activity Monitor and no
command to run — that is the whole remedy, and it is the only one offered on purpose.

What "running processes stop" means: anything a session's shell was executing at that moment is
ended. The sessions themselves come back as interrupted and resumable, with their history, and one
click starts each one again.

A release that does not change how the two halves talk shows the gentler version of the same banner
— **"A newer session service is installed"** — with the same button. Either way, restarting is safe
and your sessions are unaffected.

## Removing it

Drag **Micold AI IDE** from `Applications` to the Trash. There is no uninstaller, nothing to
deregister, and no administrator password.

Quit the application first, or the session daemon it started will keep running until you log out.
Nothing registers it to start again — no login item, no launch agent, nothing that survives a logout
— so logging out or restarting ends it for good even if you forget.
[What starts automatically](#what-starts-automatically) above is the whole of it.

Your data is deliberately not in the Trash with it. Remove what you want to remove:

| Location | What is there | Safe to delete |
|---|---|---|
| `~/Library/Application Support/micold-ai-ide/` | projects, settings, session records, per-project state, and the application's own logs | yes — this is everything the application keeps |
| `~/.micold/run/` | the daemon's socket and its pid file, recreated on each launch | yes; it is empty once the daemon has exited |
| `~/.claude/` (or your AI CLI's own directory) | your conversations with the AI CLI | **only if you mean it** — this belongs to that tool, not to this application, and other tools you use with it read the same history |

To remove everything this application put on the Mac:

```bash
rm -rf ~/Library/Application\ Support/micold-ai-ide ~/.micold
```

Your projects and their git repositories are never touched by any of this. The application opens
directories where you keep them; it does not copy them anywhere, and removing it leaves them exactly
as they are.

## If your home directory path is unusually long

macOS caps the path of a socket like the one above at 103 bytes — a limit of the operating system's
socket API, not of this application. The endpoint path is kept short deliberately (`d.sock` rather
than a longer name) to leave as much of that budget as possible for your home directory.

`/.micold/run/d.sock` uses 19 of those bytes, so your home directory path has 84 to itself. A normal
`/Users/<name>` is nowhere near that, and you would have to be well out of the ordinary — a deeply
nested network home directory, say — to run out. If you do, the application says so explicitly,
naming the length and the limit, instead of failing to start a session for no stated reason. The fix
is a shorter home directory path; no setting can raise an operating-system limit.

## Permissions

macOS keeps certain folders behind a per-application permission, and it does not tell an application
that a folder exists until permission is granted. Micold AI IDE opens projects wherever you keep
them, so it asks for the ones people keep projects in — and only when you actually try to open
something there.

| Folder | When you are asked | What the prompt says |
|---|---|---|
| Documents | The first time you open a project under `~/Documents` | "Micold AI IDE needs access to your Documents folder to open projects you keep there." |
| Desktop | The first time you open a project under `~/Desktop` | "Micold AI IDE needs access to your Desktop to open projects you keep there." |
| Downloads | The first time you open a project under `~/Downloads` | "Micold AI IDE needs access to your Downloads folder to open projects you have just cloned or unpacked." |
| Removable volumes | The first time you open a project on an external or USB disk | "Micold AI IDE needs access to removable drives to open projects stored on an external disk or USB drive." |
| Network volumes | The first time you open a project on a mounted network share | "Micold AI IDE needs access to network volumes to open projects stored on a shared or mounted network drive." |

The prompt is the system's own; the sentence inside it is ours. Each one names the folder and the
reason in the terms of what you were just doing, because a permission prompt that says only that an
application "would like to access" something gives you nothing to decide with.

There is no prompt at launch. If your projects live in, say, `~/code`, you will never see any of
these.

### If you decline

Declining is a real answer and nothing breaks. That folder stays invisible to the application: the
project you were opening will not open, and the application tells you which permission is missing
and where to grant it, rather than reporting a generic error or implying the project is broken.

To change your mind later, open **System Settings › Privacy & Security**, find the entry for the
folder (Documents Folder, Desktop Folder, Downloads Folder, Removable Volumes, or Network Volumes),
switch **Micold AI IDE** on, and reopen the application. macOS applies these changes when an
application starts, so a running application will not pick them up.

### Full Disk Access (optional)

macOS also offers a single **Full Disk Access** grant that covers all of the above at once, in
System Settings › Privacy & Security › Full Disk Access.

It is entirely optional. Micold AI IDE never asks for it, does not check for it, and installs and
runs without it — everything above works through the individual prompts. It exists here only
because some people prefer to answer once instead of five times.

Be clear about what it grants before you use it: Full Disk Access is not "the five folders above".
It gives the application access to every file your user account can reach, including your Mail,
Messages, Safari and Time Machine data, which this application has no use for. Granting the
individual folders you actually keep projects in is the narrower choice, and the one we would
suggest.
