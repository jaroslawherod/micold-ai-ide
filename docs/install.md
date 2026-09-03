# Installing Micold AI IDE

This page describes release **{{MICOLD_VERSION}}**. Every published version of the site describes the
release it was published from, so the downloads below are the ones that release actually ships.

## Linux (Debian, Ubuntu and derivatives)

Each release carries a `.deb` built natively for the two architectures the project publishes:

| Architecture | Download |
|---|---|
| 64-bit Intel/AMD (`amd64`) | [micold-client_{{MICOLD_VERSION}}-1_amd64.deb](https://github.com/jaroslawherod/micold-ai-ide/releases/download/{{MICOLD_TAG}}/micold-client_{{MICOLD_VERSION}}-1_amd64.deb) |
| 64-bit Arm (`arm64`) | [micold-client_{{MICOLD_VERSION}}-1_arm64.deb](https://github.com/jaroslawherod/micold-ai-ide/releases/download/{{MICOLD_TAG}}/micold-client_{{MICOLD_VERSION}}-1_arm64.deb) |

Install it with `apt`, which pulls in the packages it depends on — installing with `dpkg -i` instead
leaves them unresolved:

```console
$ sudo apt install ./micold-client_{{MICOLD_VERSION}}-1_amd64.deb
```

That gives you `micold-ai-ide` on your path, a desktop entry with the application's icon, and the
session service — `micold-daemon`, plus the systemd user units it can be started from. The units are
shipped but not enabled: the application enables them for you when you ask it to, in
[Settings → Session service](user-guide/settings.md).

Start it from your desktop's application menu, or run `micold-ai-ide`. There is nothing else to
configure — the application spawns the session service itself the first time it needs one
([The Micold session daemon](daemon.md)).

To upgrade, install the newer `.deb` over the top. To remove it:

```console
$ sudo apt remove micold-client
```

## macOS and Windows

**There is no packaged build for macOS or Windows yet.** The application itself runs on all three
platforms — the code has no Linux-only path, and the tests run on all three — but the release only
carries a Linux package, so on macOS and Windows you build it yourself.

You need [Rust](https://www.rust-lang.org/tools/install) (the version in `rust-toolchain.toml`, which
`rustup` installs for you) and `git`. On Windows, also install the *Desktop development with C++*
workload from the Visual Studio Build Tools — the linker comes from there.

```console
$ git clone --branch {{MICOLD_TAG}} https://github.com/jaroslawherod/micold-ai-ide.git
$ cd micold-ai-ide
$ cargo build --release -p micold-client -p micold-daemon
```

The two binaries land in `target-shared/release/` (the workspace redirects cargo's output there).
Keep them **next to each other**: the client looks for the session service as its own sibling before
it looks on the path, so moving one without the other leaves it unable to start a session.

Run it with `cargo run --release -p micold-client`, or copy both binaries somewhere on your path and
run `micold-ai-ide`.

## What you need beside it

Micold AI IDE runs *your* AI CLI — it does not bundle one and does not carry your credentials. Install
and sign in to whichever you use before starting a session:

- [Claude Code](https://docs.claude.com/en/docs/claude-code) — the `claude` command.
- [GitHub Copilot CLI](https://docs.github.com/en/copilot) — the `copilot` command.

Either has to be on the path of the session service, which is your own path unless you have moved the
service into a container. [Settings](user-guide/settings.md) explains where that path comes from and
how to change it.

## Where to go next

- [Help & About](user-guide/help-about.md) — the window, and how to find the version you are running.
- [Opening a project](user-guide/project-selection.md) — the first thing to do with it.
