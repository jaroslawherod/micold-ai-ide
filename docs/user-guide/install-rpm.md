# Installing on Fedora, RHEL and openSUSE

This page describes release **{{MICOLD_VERSION}}**. It covers installing Micold AI IDE from the
`.rpm` package on distributions that use RPM: Fedora, Red Hat Enterprise Linux and its rebuilds, and
openSUSE. On Debian, Ubuntu and derivatives use the `.deb` described in
[Installing Micold AI IDE](../install.md).

## Download

Each release carries an `.rpm` built natively for the two architectures the project publishes, and
only for 64-bit systems:

| Architecture | Download |
|---|---|
| 64-bit Intel/AMD (`x86_64`) | [micold-client-{{MICOLD_VERSION}}-1.x86_64.rpm](https://github.com/jaroslawherod/micold-ai-ide/releases/download/{{MICOLD_TAG}}/micold-client-{{MICOLD_VERSION}}-1.x86_64.rpm) |
| 64-bit Arm (`aarch64`) | [micold-client-{{MICOLD_VERSION}}-1.aarch64.rpm](https://github.com/jaroslawherod/micold-ai-ide/releases/download/{{MICOLD_TAG}}/micold-client-{{MICOLD_VERSION}}-1.aarch64.rpm) |

## Install

Install it with your package manager, which pulls in the libraries it needs:

```console
$ sudo dnf install ./micold-client-{{MICOLD_VERSION}}-1.x86_64.rpm
```

On openSUSE:

```console
$ sudo zypper install ./micold-client-{{MICOLD_VERSION}}-1.x86_64.rpm
```

The package is not signed, so `zypper` asks whether to trust it; `dnf` needs no extra flag for a
local file. Installing with `rpm -i` instead leaves the libraries unresolved.

It gives you:

- `micold-ai-ide` and `micold-daemon` on your path. The application starts the session service
  itself the first time it needs one ([The Micold session daemon](../daemon.md)).
- a desktop entry with the application's icon.
- the README under `/usr/share/doc/micold-ai-ide/`.

It installs no service unit and registers nothing to start at login.

## Run

Start it from your desktop's application menu, or run `micold-ai-ide`. It needs a graphical session,
Wayland or X11, and the libraries for both are among the package's requirements. You also need an
AI CLI of your own; see *What you need beside it* in [Installing Micold AI IDE](../install.md).

## Upgrade and remove

To upgrade, install the newer `.rpm` the same way. To remove it:

```console
$ sudo dnf remove micold-client
```
