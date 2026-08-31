# Evidence: US1 — nothing to install, nothing to register

**Feature**: 028-client-managed-daemon
**Task**: T028 (quickstart B7, B8)
**Recorded**: 2026-08-31, on the development machine (Linux 7.0.0-30-generic, Debian-family, systemd
user manager present). Baseline for the same machine: [`baseline.md`](baseline.md).

Quickstart B7 and B8 are written for a **fresh VM** and a **root install of a real package**. What
follows separates what was verified here from what still needs that VM, rather than reporting the
whole of either as done.

---

## The package under test

`mise run deb` → `target-shared/debian/micold-client_0.10.0-1_amd64.deb` (7,147,660 bytes), built
from the US1 tree.

### B8 step 3, on the package rather than on a machine

`dpkg-deb -c` — every path the package ships:

```
./usr/bin/micold-ai-ide
./usr/bin/micold-daemon
./usr/share/applications/micold-ai-ide.desktop
./usr/share/doc/micold-ai-ide/README.md
./usr/share/doc/micold-client/copyright
./usr/share/icons/hicolor/{16x16,32x32,48x48,64x64,128x128,256x256,512x512}/apps/micold-ai-ide.png
./usr/share/icons/hicolor/scalable/apps/micold-ai-ide.svg
```

- **Two executables, the desktop entry, the icons, documentation** — and nothing else. Packaging
  contract §1.1.
- **No path under `usr/lib/systemd`**, so `/usr/lib/systemd/user/micold-daemon.service` and
  `.socket` cannot exist after installing this package. Packaging contract §1.2.
- `dpkg-deb --ctrl-tarfile … | tar t` lists **`./control` only** — no `postinst`, `preinst`, `prerm`
  or `postrm`. Two clauses fall out of that single fact: installing runs no maintainer script and so
  **starts no process** (§1.3), and the upgrade adds none (§2.5). It is also why §2.6 has to exist:
  there is no root script here that could reach a per-user manager even if one were wanted.

`systemctl --user list-unit-files | grep micold` being empty after a clean install (B8's own
wording) is implied by the file list but **not measured** — see *What still needs a VM* below.

### What the previous release shipped, for contrast

`git show HEAD:crates/micold-client/Cargo.toml` still carries the two asset lines this feature
deleted:

```
["../../packaging/micold-daemon.socket",  "usr/lib/systemd/user/micold-daemon.socket",  "644"],
["../../packaging/micold-daemon.service", "usr/lib/systemd/user/micold-daemon.service", "644"],
```

dpkg removes files an upgrade no longer ships, which is B7 step 3 and packaging contract §2.5. The
before/after asset lists are the whole mechanism; no maintainer script is involved on either side.

---

## B7's precondition is real on this machine

This machine is a genuine upgrade case, not a constructed one — which is why the baseline recorded
it. The *installed* package is the previous release, and it owns the units:

```
$ dpkg -l micold-client
ii  micold-client  0.12.1-1  amd64  Thin iced client for Micold AI IDE (feature 010) …

$ dpkg -S /usr/lib/systemd/user/micold-daemon.socket
micold-client: /usr/lib/systemd/user/micold-daemon.socket
```

and the per-user enablement §2.6 is about is present:

```
$ systemctl --user is-enabled micold-daemon.socket
enabled
$ ls -l ~/.config/systemd/user/sockets.target.wants/micold-daemon.socket
… -> /usr/lib/systemd/user/micold-daemon.socket
```

**B7 steps 4–6 were deliberately not run here.** The migration's real invocation is
`systemctl --user disable --now micold-daemon.socket micold-daemon.service`; `--now` stops a running
`micold-daemon.service`, which on this machine would kill the developer's live sessions. Running it
to produce a tick in an evidence file is not worth that, and the VM the quickstart asks for costs
nothing to lose.

### What was verified instead, at the level the migration actually decides

The decision logic is render-free and directly tested against a temporary `systemd/user` tree, with
the un-enable injected rather than spawned — `crates/micold-client/src/shell/legacy_units.rs`,
seven tests:

| Test | Clause |
|---|---|
| `an_enabled_unit_is_disabled` | §2.6 — the leftover is found and cleared |
| `an_enablement_under_any_target_is_found` | §2.6 — by unit *name*, not by a hard-coded `.wants/` directory |
| `an_unrelated_unit_is_left_alone` | §2.6 — scope |
| `nothing_enabled_runs_nothing` | §2.7 — no subprocess in the common case |
| `it_does_not_repeat_once_nothing_is_enabled` | §2.7 — "MUST NOT be repeated" |
| `a_machine_with_no_user_manager_is_not_a_fault` | §2.7 — a missing directory is silent |
| `a_failure_is_swallowed_rather_than_raised` | §2.7 — "MUST ignore every failure silently" |

The ordering half of §2.7 ("before the application connects or auto-spawns") is a call site, not a
behaviour, and is read at `crates/micold-client/src/main.rs` — `disable_legacy_units()` is the first
statement of `pub fn main()`, ahead of `shell::startup::run()`.

§2.8 (persisted state untouched by the upgrade) needs no evidence *from this feature*: nothing in
US1 reads, writes or relocates the catalog, sessions, settings or logs.

---

## What still needs a VM

Carry these into the B-part run; none is blocked by anything in the implementation.

- **B7 steps 1–2** — previous release installed, old opt-in enabled, new `.deb` installed over it.
- **B7 steps 3–6** — `ls /usr/lib/systemd/user/` empty; open the client once; `is-enabled` no longer
  `enabled`; `systemctl --user status` shows no failed unit and `journalctl --user -b | grep micold`
  no unit-not-found; sessions and settings intact.
- **B8 steps 1–3** — `list-unit-files | wc -l` unchanged across the install, `grep micold` empty, and
  a session surviving a client restart.

The B8 count is the one measurement the package listing above cannot stand in for: it is a statement
about the *machine's* registry, and only an install can make it.
