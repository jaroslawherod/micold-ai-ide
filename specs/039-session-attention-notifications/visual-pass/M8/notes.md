# M8 visual pass: quickstart B12 (Desktop notifications switch), 2026-10-04

HEAD 3a3fc36f (clean tree). Client and daemon built in one invocation (Compiling micold-core, -daemon, -client), copied to `~/vp/b12m8/bin` and run from there. Dark scheme (theme "Follow the system"). Xvfb + lavapipe, 1280x800 window, not a real display.

## Environment (all private)
- Display `:171` (Xvfb private; the user's is `:0`, Wayland `wayland-0`, unset for the run).
- Session bus `unix:path=/tmp/vp171/bus,guid=...` from a private `dbus-daemon --session`; the user's is `unix:path=/run/user/1000/bus`. Checked in `/proc/<pid>/environ` of the test client and daemon: DISPLAY=:171, that bus, private HOME/XDG_* under `~/vp/b12m8`. (The user's own client/daemon, pids 17982/18384, were not touched.)
- Stand-in `org.freedesktop.Notifications` (python dbus-python, logs each Notify) on the private bus; `claude` = `cat` stand-in; turns = POST UserPromptSubmit then Stop to each session's hook URL (read from the daemon's hooks/<id>.json).
- Project `proj` (git, empty commit), sessions A (selected, 2nd row) and B (1st row), both `New session`.

## Results
1. PASS. Environment page: "Desktop notifications" checkbox, checked, directly under "Let AI sessions manage worktrees and sessions", with note "Notifies you when a session you are not looking at needs you. Off: no notification, for any AI CLI; unread marks stay." Same spacing, type size and note style as the rows above (`step1-rows-crop.png`). Scrolled to bottom: last control (script warning text) fully visible, nothing cut or overlapping (`step1-top.png`, `step1-bottom.png`).
2. PASS. Unchecked, saved (settings.json `desktop_notifications: false`), Settings closed with A in view. B turn: both hooks 200. Stand-in log: no Notify (`step2-notify.log.txt`). B's row shows the attention marker and the unread dot (`step2-sidebar.png`).
3. PASS. Client restarted (new pid); the daemon kept running (same pid, same start time). B's mark persisted; no Notify at restart. Settings -> Environment: checkbox unchecked (`step3.png`).
4. PASS. Checked and saved (`desktop_notifications: true`). 7 s later no Notify for B's earlier turn. Another B turn (B not in view, A selected) then logged exactly one Notify: `summary 'New session is waiting for input'`, body `proj — Default`, actions default/Open (`step4-notify.log.txt`); still 1 line 5 s later.

## Seen, not asked
- With the private HOME lacking `~/.bashrc`, the page shows "Script not found" and a PATH warning; it did not block Save. After I created the file the stale message was still shown until the page was reopened (not examined further).
- The notification body names project and worktree, not the session name; the session name is the summary.

## Not observed
Light scheme; a real notification service's rendering; Pi/other AI CLI sessions; a notify sent while the switch is off then toggled across a daemon restart (FR-027 covered by tests, not here: the daemon was not restarted).
