# Feature 039 M6 visual pass (click on a notification), 2026-10-03

Two runs of the same pinned pair (`~/vp/m6bin`, client + daemon built in one invocation from `d5f91412`; no
change under `crates/` since). Run 1 (`notes.md`, Xvfb :161, dark) covers B9, B10, B11, B11a. Run 2 (this file,
Xvfb :163, light, `run2-*`) repeats B9 and B10 with the bus and the raise requests recorded, and adds the
repeat-click checks.

What was real and what was a stand-in:

| Part | Real | Stand-in |
|---|---|---|
| Session bus | a private `dbus-daemon --session` (`/tmp/vp163/bus`) | not the desktop's bus |
| Notification service | none (no dunst, xfce4-notifyd installed) | python3 dbus-python object owning `org.freedesktop.Notifications`: `Notify` returns an id, `GetCapabilities` has `actions` |
| Click | none: no banner is drawn | the stand-in emits `ActionInvoked(id, "default")` / `NotificationClosed(id, 2)` on the bus |
| Client path | `Notifier::listen`, `clicks()`, `SessionReveal` → `RevealSession`, `window_raise::raise` ran against a real bus and a real X server | |
| Window manager | none installed (no openbox, xfwm4, metacity, mutter) | `xev -root -event substructure` records the `_NET_ACTIVE_WINDOW` requests the client sends to the root window; nothing honours them |
| Display | Xvfb + lavapipe | not a real display |
| Session turns | daemon hook receiver, real | `claude` is a `sleep` stub; `UserPromptSubmit` then `Stop` POSTed to the session's hook URL |

Not observable here: the window coming to the front, taking input focus, being unminimised (`_NET_WM_STATE` and
`WM_STATE` are not set without a window manager; the xterm kept the input focus after every click). Whether
`focus_window` is skipped for a still-minimised window stays open. These need quickstart §C on a real desktop.

| Step | Result | Evidence |
|---|---|---|
| B9 | PASS (project switch, selection, mark); front/focus NOT OBSERVED. P active, xterm focused, C of Q finished: Notify id 1 with actions `default`/`Open`. `ActionInvoked(1, default)`: Q active, C selected (Q's remembered session was D), 2 `_NET_ACTIVE_WINDOW` requests (unminimise + focus) = one raise. C's mark stayed while the window had no focus and went when it got focus (id 3). | `run2-B9-click-switches-project-before-after.png`, `run2-B9-mark-clears-on-focus-then-C-closed.png` (rows 1, 2), `run2-xev-root.txt`, `run2-notify-service.txt` |
| one click, one reveal | PASS: 2 requests per click, never 4; ids 1, 3, 4 gave 6 in total. | `run2-xev-root.txt` |
| second click of the same id | PASS: after switching back to P, `ActionInvoked(1)` again: no pixel changed, no raise request. Same for id 4. | `run2-repeat-click-and-closed-click-no-change.png` (rows 1, 2) |
| click after `NotificationClosed` | PASS: `NotificationClosed(2, 2)` then `ActionInvoked(2)`: no pixel changed, no raise request. | same image (row 3) |
| B10 | PASS: D selected, C finished (id 4), C closed with its row's close button, id 4 clicked: D still selected, snackbar `That session is no longer available.`, one raise requested. | `run2-B10-removed-session-notice-before-after.png` |
| B11 | PASS for content (run 1 only): the second window, holding Q, switched to the notified session; the read-only window did not change. "Comes forward" NOT OBSERVED. | `B11-before-after.png`, `notes.md` |
| B11a | PASS (run 1 only): raising window destroyed with `xdotool windowclose`, then the click: the other window kept its selection, the session kept its mark. | `B11a-before-close-after-click.png`, `notes.md` |

Seen on the way:

- On the click for a background project the client log has `switch: entered …/projQ -> active_session=Some(D) choice=Some(Remembered(D))`
  before C is selected (`run2-micold-client.log.txt`): the ordinary switch enters the remembered session first.
- The click path writes no log line in the client or the daemon, at `MICOLD_LOG=debug` too: a click that does
  nothing cannot be told from one that was never received, except on the bus.
- Run 1: destroying the X window with `xdotool windowclose` ended the client with a wgpu panic
  (`wgpu_core.rs:3793`, `Surface::configure … Surface does not support the adapter's queue family`). The window
  was destroyed under the client, not closed by it; lavapipe on Xvfb.

Screenshots are 50% downscales of a 1200x900 window.
