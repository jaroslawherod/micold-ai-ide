# Visual pass, feature 491 milestone M3 (Settings -> Daemons)

Date 2026-10-11. Real client (`micold-ai-ide` + `micold-daemon`, debug build of this worktree at HEAD b68020eb),
private headless sway 1.11 + pixman + lavapipe, 1600x1200, throwaway HOME / XDG dirs, virtual pointer
(`vptr.py`, plus a copy with a right-click command), keyboard via `wtype`. Not a real display or GPU.
Settings seeded with a host daemon, a container daemon and a container daemon with a very long name
(containers never started: state Stopped).

## Results (quickstart Part B, Daemons parts of items 1-4)

| Check | Result | Evidence |
|---|---|---|
| List rows: name, runtime chip, endpoint, state chip, bound worktrees | PASS | m3-01 |
| Details open (errors + log) | PASS | m3-02 |
| Add form (name, runs in, container name, port, image, connect-at-launch) | PASS | m3-03 |
| Removal confirmation: counts and "nothing on disk is deleted"; Cancel leaves all unchanged; Continue -> "no daemon" chip | PASS | m3-04, m3-11, m3-06 |
| Empty state | PASS | m3-05 |
| Picker "Bind to daemon..." with daemons, and with none configured; binding shows "daemon <name> unavailable" for a stopped container | PASS | m3-07, m3-08, m3-09 |
| "No daemon" notice in New worktree form; "+" on Default with no daemon gives a snackbar | PASS | m3-10 |
| Item 1: watch Starting -> Connected after adding a container daemon | NOT RUN | no container runtime/image used; only Stopped and Connected (host) states seen |
| Item 3: stop container daemon, worktrees show unavailable | PARTLY | "unavailable" chip seen for a Stopped container; live stop of a running one not run |
| Item 2: two worktrees of one project, one per daemon | NOT RUN | only one worktree created; container daemon cannot run here |

Dialogs (removal confirm, bind picker, new worktree) are centred in the window (x centre 800 of 1600).

## Observations (cosmetic, not failures)

- Connected and Stopped chips are both muted purple-grey; the text tells them apart, the colour barely does.
- Opening Details widens the toggle ("Hide details") and shifts Remove right by ~30px.
- The "Connect when the application starts" checkbox sits ~8px above the Save daemon button (tight).
- Removal dialog title is "Daemon <name>" and says "container is not stopped or deleted" even for a host daemon; "0 worktrees will show..." reads oddly.
- With no daemon, Attach existing shows both an inline "not connected to the session service" and a "no daemon" snackbar.
- Harness note: a 50 ms `click` often failed to activate menu items; press/release with 150 ms did. Not an app finding.
