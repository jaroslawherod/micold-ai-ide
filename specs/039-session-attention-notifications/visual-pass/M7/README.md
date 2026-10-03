# Feature 039 M7 pass: quickstart §C4 (B9, then B11, on Wayland), 2026-10-03

The real client and session service on a private headless GNOME Shell. No change under `crates/` between the
binaries' sources and HEAD `d5b7d0ef` (the commits after `1128de38` are docs only).

## What ran

| Part | What |
|---|---|
| Binaries | `target-shared/debug/micold-ai-ide` (built 22:49:05, sha256 `ba2324a3398c…`) and `micold-daemon` (22:29:02, `fa33711897d3…`), copied to the scratch directory and run from there. The client has `window_raise::settle_after` (107 matches in `strings`); the pair connected (`client attached to daemon`, `micold-daemon.log.txt`). Version 0.16.0, debug build. |
| Compositor | `gnome-shell --headless --no-x11 --wayland-display=m7c4-wl --virtual-monitor 1280x800 --unsafe-mode`, GNOME Shell 50.1 |
| Notification service | GNOME's own helper, `gjs -m /usr/share/gnome-shell/org.gnome.Shell.Notifications` (gjs 1.88.0), on the private bus |
| Buses | two private `dbus-daemon` 1.16.2 (session and "system"), sockets in `/run/user/1000/m7-c4` |
| Input | real pointer and key events through `org.gnome.Mutter.RemoteDesktop` on the private bus. `org.gnome.Shell.Eval` was not used. |
| Other application | `gnome-text-editor` 50.0 in the private environment |
| Sessions | `claude` is a stand-in (`cat`); a turn is `UserPromptSubmit` then `Stop` POSTed to the session's hook URL of the private daemon |
| Rendering | client 1: `ICED_BACKEND=tiny-skia` (software, 0.3 to 1.7 s per frame in this debug build); client 2: wgpu on lavapipe (fast) |
| Environment | `env -i`, private `HOME`, `XDG_*`, `XDG_RUNTIME_DIR=/run/user/1000/m7-c4` (0700), `DISPLAY` unset, `GSETTINGS_BACKEND=memory`, `MICOLD_LOG=info`: `private-env.txt`. The daemon was a new process (pid 3076768) listening on `/run/user/1000/m7-c4/micold/daemon.sock` (`ss -xlp`). |

How focus was observed: `org.gnome.Shell.Introspect.GetWindows` (`has-focus`) on the private bus
(`drive-and-focus.log`), the clients' own Wayland traffic with `WAYLAND_DEBUG=1`
(`wayland-trace-excerpts.txt`), the bus (`dbus-monitor-notifications.txt`), and screenshots through
`org.gnome.Shell.Screenshot`. **The client writes no log line for a click, a raise or an attention request**
(`micold-client.log.txt` has only the two `attach:` lines), so an attention request is read from the trace:
winit's `request_user_attention` on Wayland is `xdg_activation_v1.get_activation_token` followed by `activate`
with a token that ends in `_TIME0`.

Layout in every run: project `projQ`, sessions C (`60327d15…`, first row) and D (`1545889b…`, second row), D
selected, C finishes a turn. The terminal shows the session id, so which session is shown can be read.

## B9: focus given

Run 4 (client 2, editor focused, a keyboard on the seat), `B9-run4-before-banner-after0.6s-after6s.png`:

- Before the click the editor had the focus: `window banner …578 app=micold-ai-ide.desktop has-focus=False`,
  `…579 app=org.gnome.TextEditor.desktop has-focus=True`.
- The click on the banner: `ActivationToken` `8d0b7838-…` then `ActionInvoked "default"` on the bus. The client:
  `-> xdg_activation_v1#20.activate("8d0b7838-63e0-4d0f-8ac5-c74f15ce4cb8", wl_surface#21)` and 1 ms later
  `wl_keyboard#84.enter(145, wl_surface#21, array[0])`.
- 0.6 s after the click: `window after0.6 …578 app=micold-ai-ide.desktop has-focus=True`, editor `False`. The
  client window is drawn over the editor, C is selected and shown, C's unread mark (the dot at the row's
  trailing edge) is gone. Same at 6 s.
- No `get_activation_token` followed: **no attention request**.

Run 3 (client 1, tiny-skia), `B9-run3-tinyskia-…png`: the same on the bus, in the trace and in Introspect
(`has-focus=True` at 0.6 s, `wl_keyboard.enter` 0.6 ms after `activate`, no attention request). The screenshot
at 0.6 s shows the window in front but still D, with C's mark; at 6 s C is shown and the mark is gone. The
client's next two frames after the click came at +1.45 s and +2.92 s; an ordinary click on a row in the same
client gave frames 0.3, 1.7 and 1.7 s apart. So this is the renderer of this run, and run 4 is the one that
says when the session is shown.

Run 2, the control (client 1 already focused, D shown, the banner of C clicked): `activate` sent, no
`get_activation_token`, `has-focus=True` before and after, C shown and its mark gone (seen 20 s later; at
1.5 s still D, tiny-skia).

Run 1 (client 1, the seat had **no keyboard**: `wl_seat.capabilities` 0 or 1): the compositor gave the focus
(`has-focus=True` 0.3 s after the click, `xdg_toplevel.configure(…, array[4])` 0.8 ms after `activate`, window
in front, C shown at 3 s). The client was sent no `wl_keyboard.enter`, as there was no keyboard, and 2.79 s
after `activate` it asked for attention (`get_activation_token` … `activate("450b463f-…_TIME0", …)`). A seat
without a keyboard is the harness's doing (a headless seat has one only while a remote-desktop session that
pressed a key is open); runs 2 to 4 kept one.

## B11: focus given to the second window; an attention request followed although focus was given

`B11-1-…png`: client 1 (pid 3076746) held projQ and raised C's notification (`Notify` from `:1.148` = pid
3076746). Client 2 (pid 3090879) was started, read-only, and took the project over (`project attached client=2
… force=true`); client 1 became read-only. A click put client 1's window in front with the focus
(`window w1front …575 has-focus=True`, `…578 has-focus=False`). `B11-2-…png`: the banner was gone by then, so
the notification was clicked in the shell's notification list (the clock's popup).

- The token went to client 1 on the bus (`ActivationToken "17d7458d-f47a-47cf-b072-09aa4300066f"` to `:1.148`)
  and client 2 used it: `-> xdg_activation_v1#81.activate("17d7458d-f47a-47cf-b072-09aa4300066f",
  wl_surface#21)`, then `xdg_toplevel#23.configure(1024, 768, array[4])`. Client 1 sent no `activate` and got
  `xdg_toplevel#21.configure(1024, 768, array[0])`.
- 0.6 s after the click: `window after0.6 …575 has-focus=False`, `…578 has-focus=True`. `B11-3-…png`: the second
  window is in front of the first and shows C (`60327d15…` in its terminal), at 0.6 s and at 6 s. The first
  window did not change (read-only, D).
- **410 ms after its `activate`, client 2 asked for attention**: `-> xdg_activation_v1#15.get_activation_token`,
  `done("e2cf5408-…_TIME0")`, `-> xdg_activation_v1#15.activate("e2cf5408-fb0b-43da-8b75-a8f9e7469887_TIME0",
  wl_surface#21)`. Cause seen in the trace: the shell's popup stayed open after the click and kept the keyboard,
  so client 2 got no `wl_keyboard.enter` until the popup was closed with Escape 23.7 s later
  (`wl_keyboard#84.enter(120, wl_surface#21, array[4])`). The compositor had already made the window the
  focused one (`has-focus=True`, `configure` with the activated state). Nothing visible came of the request:
  no new notification, no change on screen.
- `B11-4-…png`, after Escape: the second window in front, C selected and shown, no unread mark on C.

## Not observed

- Whether C's mark in B11 went before the popup was closed (the popup covered the rows).
- What an attention request looks like in this shell: neither request (run 1, B11) changed anything on screen.
- Any compositor but GNOME Shell 50.1 headless; a physical seat; a minimised window; another workspace; the
  overview; a release build; the banner (not the list) for B11.
- Why the popup stayed open after the click in the list (shell behaviour in this headless session, not looked
  into).
- Project switch on reveal (one project was used; the notified session was a background session of the
  active project).

## Files

`drive-and-focus.log` (input steps and `has-focus` per run), `wayland-trace-excerpts.txt`,
`dbus-monitor-notifications.txt`, `micold-client.log.txt`, `micold-daemon.log.txt`, `private-env.txt`,
`harness-scripts.txt` (the harness, adapted from `../../probe/`). Screenshots are 50% or 70% downscales of the
1280x800 virtual monitor; each `B9-run*` image is before, banner, +0.6 s, +6 s (row by row).
