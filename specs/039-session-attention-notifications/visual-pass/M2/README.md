# Feature 039 M2 visual pass (notification part), 2026-10-03

Xvfb :79 + lavapipe, no window manager (focus set with `xdotool windowfocus`), private bus
(`dbus-daemon --session`), client + daemon pinned in ~/vp/bin-039m2 from HEAD. Session turns driven by POSTing
`UserPromptSubmit` then `Stop` to the daemon's hook receiver (URL/token from the per-session hooks file);
`claude` was a stub `sleep` script. No dunst/mako installed: a ~20-line python3 `org.freedesktop.Notifications`
stand-in logged each Notify (notify-service.txt) next to a dbus-monitor capture (dbus-monitor.txt).

| Step | Result | Evidence |
|---|---|---|
| B1 | PASS: one Notify, summary `B is waiting for input`, body `P — Default`, hint desktop-entry=micold-ai-ide | notify-service.txt id=1, B1.png |
| B2 | PASS: A finished, window focused, A in view: 0 Notify | B2.png |
| B3 | PASS: other app focused, A finished: 1 Notify (id=2) | B3.png |
| B4 | PASS: Settings open, A finished: 1 Notify (id=3) | B4.png |
| B5 | PASS: C finished: 1 Notify `C is waiting for input`, body `Q — Default` (id=4) | B5.png |
| B13 | PASS: service stopped, B/C/B finished: 0 Notify, no in-app notice, exactly 1 `desktop notification not shown (...)` log line | micold-client.log.txt, B13.png |

Screenshots are 40% downscales. The extra "New session" row in P is a session created while finding the UI.
