# M6 visual pass: B9, B10, B11, B11a (2026-10-03)

Xvfb :161 + lavapipe (not a real display), private runtime dir, data home, HOME and session bus (`dbus-daemon`, `/tmp/vp161/bus`). Client and daemon built from HEAD d5f91412 in one invocation (Compiling micold-core, -client, -daemon) and pinned to `~/vp/m6bin`. Fake `claude` (`sleep`); turns driven by POSTing `UserPromptSubmit` / `Stop` to each session's hook URL. **No dunst/mako here**: a small python (dbus-python) stand-in for `org.freedesktop.Notifications` logged each `Notify` (`notify-service.log`) and emitted `ActionInvoked(id, "default")` on demand. So "click" is the service's signal, not a pointer click on a rendered toast. **No window manager** on the display: stacking/raise and `_NET_ACTIVE_WINDOW` cannot be observed.

## B9 PASS (selection and mark), "in front and focused" NOT COVERED
Q's session C finished a turn while an xterm held focus; Notify id 1 (`New session is waiting for input`, `projQ — Default`, actions `default`/`Open`). `ActionInvoked(1, default)`: Q became the active project, C shown and selected. The mark stayed while the window had no input focus (expected: not in view), and cleared once the window got focus. `B9-click-selects-then-focus-clears-mark.png` (top: right after the click, mark at C's trailing edge; bottom: after focus). Window did not come forward and did not take input focus (xterm kept it): no WM to honour the request. Needs C2/C4-style real display.

## B10 PASS
C removed, then Notify id 2 (raised before removal) clicked with D selected in Q: selection stayed on D, snackbar `That session is no longer available.` with Dismiss. `B10-notice-crop.png`. Window front/focus: not covered (as B9).

## B11 PASS (content), "comes forward" NOT COVERED
Second client process on Q (read-only banner until Take over). After Take over, second window held Q and showed E. Notify id 3 for D clicked: the second window switched to D; the other window (read-only) unchanged. `B11-before-after.png` (top before, bottom after; left = first window, right = second). Which window raised the notification is the project holder in every run; Notify showed one line per event, not one per window.

## B11a PASS
Holder window (W3) raised id 6 for E; W3 closed (xdotool windowclose, i.e. the X window destroyed, not the close button; the client panicked on exit, see run.log; the daemon stayed up). Clicking id 6: the second window kept D selected, E kept its mark. `B11a-before-close-after-click.png` (rows: before close, after close, after click; the black left half is the closed window). A first attempt of B11a was inconclusive (the second window had been the raiser) and is not counted.

## Not covered
Light scheme; a real notification service's rendering; raise/focus on X11 and Wayland (needs a WM or C-section hand test); mid-flight animation.
