# Quickstart

## Part A: automated
`mise run test-core`, then `mise run gate`.

## Part B: visual pass
1. Open Settings: the rail lists Appearance, Terminal, Environment, Daemons, GitHub, Jira, no
   Session service.
2. Daemons > edit a container daemon: runtime, image source/reference/file, credentials, network,
   limits, keep-running all present and scroll; an out-of-range limit shows beside the field.
3. Edit a second daemon: its values differ from the first.
4. Tick a credential share in the form: the Daemons rail row shows *Sharing* at once, and from any section after save.
5. Change a limit on a running daemon and save: the confirmation appears.
6. Host daemon: container groups inert, keep-running caution shown. SSH daemon: none shown.

## Part B result

Date 2026-10-17; commit 0e3b543f plus the uncommitted working-tree changes present at the time (the collapsible container sections of feature M4, `git status` showed `daemons.rs`, `settings.rs`, `accordion_section.rs` modified). Run on a private headless sway (pixman) with lavapipe, window 1600x1200, private data home seeded with four daemons (host, two containers, one SSH); not a real display. Screenshots in `visual-pass/`.

1. PASS: rail lists Appearance, Terminal, Environment, Daemons, GitHub, Jira, no Session service (`b1-rail.png`).
2. PASS: Container section open, Credentials/Network/Limits/Sessions closed by default (`b2-default-collapsed.png`); all expand and scroll (`b2-sections.png`); an out-of-range processor limit (".001") with Limits collapsed, then Save daemon, reopens Limits with "Enter between 0.25 and 1024 cores" beside the field (`b2-limits-error-reopens.png`).
3. PASS: Sandbox B shows 4 cores / 4096 MiB / 1024 pids and different credentials, against Sandbox A's 2 / 2048 / 512 (`b3-second-daemon.png`).
4. PASS: ticking Git configuration in the form made the Daemons rail row show *Sharing* at once, with none shown before (`b4-sharing-in-form.png`); after Save daemon the badge stays when Appearance is selected (`b4-sharing-after-save.png`).
5. NOT RUN: needs a running daemon; starting one would pull and run a real container image on this machine, so no daemon was started and the running-daemon confirmation was not seen.
6. PASS: Host daemon form has no container sections and shows the "can't outlive signing out" caution (`b6-host.png`); the SSH daemon form shows host/user/port/key file only, no container sections and no caution (`b6-ssh.png`).
