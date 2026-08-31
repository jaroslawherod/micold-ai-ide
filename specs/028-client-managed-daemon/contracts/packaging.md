# Contract: What an Install May Leave Behind

**Feature**: 028-client-managed-daemon

Checkable from outside the application, on a machine, with no source access. This is the half of the
feature a user can verify without opening the app.

## §1 A fresh install

1. The package MUST install exactly two executables (the client and the session service), the
   desktop entry, the icons, and documentation. It MUST NOT install a unit, plist, login item,
   scheduled task, or any other service-manager artefact for the session service.
2. `/usr/lib/systemd/user/micold-daemon.service` and `.socket` MUST NOT exist after installing.
3. Installing MUST start no process.
4. The operating system's list of registered user services MUST be unchanged by the install.

## §2 An upgrade over a release that shipped the units

5. Upgrading MUST remove both unit files. dpkg does this for files the new version no longer ships;
   no maintainer script is required and none is added.
6. A per-user *enablement* left by the removed opt-in — the symlink under
   `~/.config/systemd/user/sockets.target.wants/` — MUST be removed by the application at its next
   start, not by the package, because a root maintainer script cannot reach a per-user manager.
7. That removal MUST run before the application connects or auto-spawns, MUST require no command
   from the user, MUST ignore every failure silently (no user manager is not a fault), and MUST NOT
   be repeated once there is nothing enabled.
8. Persisted state — catalog, sessions, settings, logs — MUST be untouched by the upgrade.

## §3 Uninstall

9. Removing the package MUST leave no service-manager artefact, because §1 installed none.
10. A service still running at uninstall is not the package's to stop; it stops itself under
    `contracts/lifecycle.md` §3 within 30 minutes of the last client.

## §4 What is no longer claimed

11. The application MUST NOT offer to make a host-process service survive logout, on any platform.
12. The documentation MUST state that a directly-hosted session service does not survive logout, and
    MUST name the sandboxed placement as the supported way to get that.
13. The sandboxed placement's survive-logout/reboot opt-in remains, is presented as belonging to
    that placement, and MUST state that a kept sandbox is not stopped when idle.
