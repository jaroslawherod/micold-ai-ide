# M9 visual pass: quickstart section B, LIGHT scheme, 2026-10-04

HEAD 378c0c38. Client, daemon and showcase built in one invocation (Compiling micold-core, -daemon, -client), pinned to `~/vp/m9bin` (strings check: `desktop_notifications` in client and daemon), run from there. Xvfb + lavapipe, 1280x800 client window: not a real display, no window manager.

## Environment
- Display `:191` (private Xvfb; the user's `:0` / wayland-0 not used, `WAYLAND_DISPLAY` unset).
- Session bus: private `dbus-daemon --session` at `/tmp/vp191/bus`; the user's bus not touched. Private HOME, XDG_DATA/CONFIG/CACHE/STATE, `XDG_RUNTIME_DIR=/tmp/vp191`.
- Light scheme: `settings.json` `"theme": "light"` (Settings -> Appearance -> Theme shows "Light"); dark = `"theme": "dark"` plus a client restart.
- Real: client, daemon, hook receiver. Stand-in: `claude` = `sleep`; turns = POST UserPromptSubmit then Stop to each session's hook URL (from `hooks/<id>.json`); notification service = python dbus-python `org.freedesktop.Notifications` stub logging each Notify (`notify-service.txt`). A real service's rendering is not seen.
- Setup: P (git) with sessions A (selected) and B (first row), Q with session C. All three sessions are named `New session`, so the notification title is `New session is waiting for input` (summary = session name; body = `<project> — Default`).
- Window close (B8) = WM_DELETE_WINDOW sent by a small ctypes script (client exited, daemon kept running).

## Results
| Step | Scheme | Result | Observation | Evidence |
|---|---|---|---|---|
| B1 | light | PASS | A selected, B finished: exactly one Notify, summary `New session is waiting for input`, body `P — Default`, actions default/Open (id=1). B shows mark and unread dot. | `B1-light.png`, `notify-service.txt` |
| B5 | light | PASS | C finished: Notify id=2, body `Q — Default`. Button `P ● 1`, panel `Q  1 running / ● 1 unread`. Tooltip `1 unread session in other projects` showed while the panel was open (overlaps the panel's top edge, hides no row text; same as M5). | `B5-panel-light.png` |
| B6 | light | PASS | B's row: red ring (awaiting input) at leading edge, label heavier than A's, purple mark at trailing edge between `claude` and the close X; panel P `2 running / ● 1 unread`; button still `● 1`. Whether the activity indicator is "unchanged" was judged by its look (red ring), not by diffing before and after. | `B6-rows-light.png` (3x), `B6-state-light.png`, `B5-panel-light.png` |
| B7 | light | PASS | Mark gone at the first sample after the click: 0.15 s (pixel probe on the mark's centre, 1x1 import loop, includes the capture latency). Panel afterwards: P `2 running` only, Q `1 unread` and button `● 1` remain. Row highlight was still mid-transition in the capture. | `B7-after-light.png`, `B7-panel-light.png` |
| B8 (shown session differs) | light | PASS | Window closed, A finished (no Notify, log unchanged at 3 lines), app reopened on B (last selected): A marked, button `P ● 1`. | `B8a-reopen-other-session-marked-light.png` |
| B8 (shown session finished) | light | PASS | Window closed, B (the one reopened on) finished: no Notify; after reopening B is unmarked. A's mark from the first branch stays (so the clean B row is the evidence). | `B8b-reopen-shown-session-unmarked-light.png` |
| B12 | light | PASS | Environment page: "Desktop notifications" checkbox on, with its note, same spacing and type as neighbours. Unchecked + Save (`desktop_notifications: false`): A finished while B in view, 0 new Notify (3 -> 3), A marked. Checked + Save (`true`): A finished again, exactly one Notify (3 -> 4). Page text/switch crisp in light. | `B12-environment-page-light.png`, `B12-switch-off-light.png`, `B12-switch-on-light.png`, `B12-off-A-finished-light.png`, `B12-on-A-finished-light.png` |
| B13 | light | PASS | Stub service stopped; A finished twice: 0 Notify, no in-app notice on screen, A marked (A was already marked from earlier turns, so the mark is not discriminating), exactly one client-log line `desktop notification not shown (no notification service: ...); later failures in this run are not logged`. | `B13-light.png`, `client-warn-line.txt` |
| B14 | light and dark | PASS | Showcase: UnreadMark entry (unread row with trailing dot, read row, switcher button `● 3`, button with none) and the MenuOverlay switcher list (`2 running / ● 1 unread`, `● 2 unread`, unavailable row with warning icon). No glyph collision; `session-daemon-notes` wraps to two lines in the panel in both schemes (known from M5). Three entries = unread row, switcher button, switcher panel rows; one screenshot per scheme each. | `B14-unreadmark-light.png`, `B14-unreadmark-dark.png`, `B14-switcher-panel-light.png`, `B14-switcher-panel-dark.png` |
| B15 | light | PASS | A selected, marks cleared first, B and C finished, no session opened. Read from the sidebar and switcher alone: B's trailing mark (P), button `● 1`, panel `P ● 1 unread` and `Q ● 1 unread` -> B in P, C in Q. About 3.5 s from C's Stop to the panel screenshot (includes opening the panel; tool latency, not human reading time). | `B15-light.png`, `B15-panel-light.png` |
| B15 | dark | PASS | After a restart in dark the same marks persisted and the same reading holds. | `B15-dark.png` |

## Not observed
- B2, B3, B4, B9, B10, B11, B11a: not in this pass's scope (B2-B4 in M2, B9-B11a in M6/M7).
- B8 and B12/B13 in the dark scheme (B12 seen dark in M8); B1/B5/B6/B7 dark only from M2/M5.
- A real notification service's rendering; window raise/focus (no WM); mid-flight animation.

## Seen outside the steps
- Switcher tooltip overlaps the open panel's top edge (as M5), light too.
- Environment page shows the GitHub Copilot / Pi PATH warning (private HOME has no tools), harmless.
- Notification summary is the session name, so identically named sessions give identical titles; the body is what tells projects apart.
- A first seed attempt wrote both projects with the same path (my script bug, not the app); fixed before any step.
