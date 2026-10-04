# Feature 039 M9 window-manager pass: B9, B10 (not run), B11, B13 and the §C4 idea on X11 with a real WM, 2026-10-04

Questions earlier passes could not observe: the window in front/focused under a real window manager on X11 (M6 had no WM),
un-minimise, B11 through the real banner with a project switch, and B13 with a real notification service stopped.
Everything below was seen on this machine; B10 was not repeated (M6 covered it; nothing new to see with a WM).

## What ran

| Part | What |
|---|---|
| Binaries | pinned pair `~/vp/m9bin` (READY present), copied to a scratch dir. `micold-ai-ide` sha256 `9906a0b615fc3f6837ef4ff4d7f9b45ad20c4d6389760095f201640e7bf51ee9`, `micold-daemon` sha256 `d36fe278ec4960390bd44be599c19b73c3b60c89c95beb49af24ffffa574f77e` (re-hashed in `~/vp/m9bin`: same). Debug builds, 0.16.0, `settle_after` present (107 matches in `strings`). Not built here. |
| Compositor / WM | `gnome-shell --headless --wayland --virtual-monitor 1280x800 --unsafe-mode` (no `--no-x11`), GNOME Shell 50.1 / mutter 50.1. It started **Xwayland 24.1.10** as public X display `:2` (`_NET_SUPPORTING_WM_CHECK` window has `_NET_WM_NAME` = `GNOME Shell`; mutter is the X11 window manager; decorated windows, real minimise button). `XAUTHORITY` = the shell's own `.mutter-Xwaylandauth.*`. The user's Xwayland (`:0`, `:1`) was never touched. |
| Notification service | GNOME's own helper `gjs -m /usr/share/gnome-shell/org.gnome.Shell.Notifications` on the private bus: it owned `org.freedesktop.Notifications` (pid recorded) and drew the real banner |
| Input | real pointer clicks (title-bar minimise button, banner, rows, "Take over") through `org.gnome.Mutter.RemoteDesktop` on the private bus (harness as M7, `logs/*.sh.txt`, `pollw.py.txt`) |
| Other application | `gnome-text-editor` 50.0 with `GDK_BACKEND=x11` (an X11 client). **Stand-in step:** to give it the focus before a run I used `xdotool windowactivate` (a window-manager request, not input) |
| Sessions | `claude` is a stand-in (`cat`); a turn is `UserPromptSubmit` then `Stop` POSTed to the session's hook URL of the private daemon. Layout as M7: project `projQ` (and an empty `projP`), sessions C `a0082f83…` (row 1) and D `2497ec46…` (row 2), D selected, C finishes a turn |
| Clients | X11: `DISPLAY=:2`, no `WAYLAND_DISPLAY`, winit took X11 (window class `micold-ai-ide`, maximised by mutter). Wayland: `WAYLAND_DISPLAY=m9-wl` on the same shell. Renderer `WGPU_BACKEND=vulkan` except one run with `ICED_BACKEND=tiny-skia` (software, slow) |
| Environment | `env -i`, private `HOME`, `XDG_*`, `XDG_RUNTIME_DIR=/run/user/1000/m9-x` (0700), two private `dbus-daemon`, `GSETTINGS_BACKEND=memory`, `MICOLD_LOG=info`; the user's bus and `wayland-0` were never used. New daemon (pid 377212) on the private socket |

How it was observed: `_NET_ACTIVE_WINDOW` and the client's `WM_STATE` / `_NET_WM_STATE` polled with `xprop` every ~20 ms
(`logs/*-active.txt`; wall-clock timestamps, drive-script "press done" = the click), `org.gnome.Shell.Introspect.GetWindows`
(`has-focus`, `is-hidden`), screenshots through `org.gnome.Shell.Screenshot` (grids: before, banner, +0.6 s, +6 s; 50% downscale),
the bus (`logs/dbus-monitor-notifications.txt`), one `strace -f` run of the client, and `WAYLAND_DEBUG=1` for the Wayland client.
The click was always the real banner (the shell's banner drawn at the top, clicked at (640,90) with the pointer).

## Results

| Step | Result | Evidence |
|---|---|---|
| 1. B9 on X11, banner clicked while another application holds focus (wgpu) | **PASS**. Editor active, banner shown, click: `_NET_ACTIVE_WINDOW` = client **24 ms** after the click (repeat: 21 ms), window in front, `FOCUSED`, Introspect `has-focus=True`; C shown (its session id in the terminal), C's trailing unread mark gone at +0.6 s. Within SC-004's 2 s. | `x11-b9-wgpu-grid.png`, `logs/x11-b9-wgpu-active.txt`, `logs/x11-b9-wgpu-run2-active.txt`, `logs/*-drive.txt` |
| 1b. same, `ICED_BACKEND=tiny-skia` | slow: focus **3.7 s** after the click. The window was in front only at the next frame; renderer of that run (M7 saw the same: 0.3 to 1.7 s per frame), not the raise. | `x11-b9-grid.png`, `logs/x11-b9-active.txt` |
| 1c. Q-active / "C shown" | PASS in all runs (C's terminal shown, mark gone) | grids |
| 2. Un-minimise on X11 (client minimised by a real click on its title-bar button, editor focused, C finishes a turn, banner clicked) | **PASS, but slow: 2.7 s** (three runs: 2.75, 2.70, 2.70 s from click to un-minimised+focused; one of them under strace). At +0.6 s and +2 s the client was still `Iconic`/`HIDDEN` and the editor focused; at +6 s `Normal`, `FOCUSED`, in front, C shown, mark gone. Un-minimise and focus came 30 ms apart. So the window is un-minimised, raised and focused, but **SC-004 (2 s) is missed for a minimised window**. Control: `xdotool windowactivate` on the same minimised window is honoured by mutter in 40 ms, so the 2.7 s is the client's. | `x11-unmin-grid.png`, `logs/x11-unmin*-active.txt`, `logs/x11-unmin-control-xdotool-windowactivate.txt` |
| 2b. Is winit's X11 `focus_window` skipped for a minimised window? | **No, not skipped on the wire.** strace of the client: `ActionInvoked` received at 11:34:13.154; the main thread then only woke on a 1 s tick (13.82, 14.82, 15.82) and at 11:34:15.8267 and 15.8270 wrote **two** identical `SendEvent` ClientMessages to the root, atom 314 = `_NET_ACTIVE_WINDOW` (`xlsatoms`): the unminimise step and the focus step. For a window that is not minimised the same two requests went out 13 ms after the click. Which of the two mutter honoured cannot be told (one suffices). What delays the minimised case by ~2.7 s is **not identified** (the click was read at once; the main loop did not act until the third 1 s wake-up). | `logs/x11-unmin3-strace-excerpt.txt` |
| 2c. Un-minimise on Wayland (same shell, `WAYLAND_DISPLAY=m9-wl`) | **PASS, fast**: client minimised by a real click, banner clicked: un-hidden and `has-focus=True` **25 ms** after the click; `xdg_activation_v1.activate("…_TIME7291690")` with the click's token, `configure(…, array[4])`, C shown, mark gone. In this run the seat had no keyboard (harness: pointer only), so no `wl_keyboard.enter` and the client asked for attention 0.41 s later (the M7 run 1 artifact). Repeat with a keyboard held on the seat (`drive.py key:42 sleep:240`): **10 ms**, `wl_keyboard.enter` 0.7 ms after `activate`, **no attention request**. | `wl-unmin-grid.png`, `logs/wl-unmin-focus.txt`, `logs/wl-unmin-kbd-focus.txt`, `logs/wl-unmin-wayland-trace-excerpt.txt` |
| 3a. B11 through the real banner, X11 | **PASS for the cross-window reveal, not the exact scenario.** Two clients on Q: A (pid 518842, started first, holds Q), B (pid 519604, started second, red banner "This project is already open in another window", read-only). The notification for C was raised by **B** (`Notify` sender :1.1365 = pid 519604, the read-only one). Banner clicked with the editor focused: the **holder A came forward and was focused**, 1.29 s after the click (`active=0x800002` at 666.943, click 665.656), showing C, mark gone; B stayed behind. | `x11-b11-grid.png`, `logs/x11-b11-active.txt`, `logs/dbus-monitor-notifications.txt` |
| 3b. same after a real click on "Take over" in B (B holds Q, A read-only) | PASS: `Notify` again from B; click: B in front and focused **30 ms** later, C shown, mark gone. | `x11-b11-owner-grid.png`, `logs/x11-b11-owner-active.txt` |
| 3c. B11 as written ("notification from the first window, second window holds Q") | **NOT OBSERVED.** In both runs the notification came from the later-started client (B); I could not make the earlier client raise a banner after the second window existed (a banner lasts ~4 s, a client takes ~20 s to start; M7 got it with a list entry instead). Why the newer client raises (and not both, or the holder) was not looked into. | `logs/dbus-monitor-notifications.txt` |
| 3d. Project switch on reveal, X11 | **PASS.** Window showing `projP`; C of `projQ` finishes: banner `projQ — Default`, the project button shows `projP ● 1`. Click: `projQ` active, C (not the remembered D) shown, mark gone, window focused **27 ms** after the click. | `x11-switch-grid.png`, `logs/x11-switch-active.txt` |
| 4. B13 with a real notification service stopped | **PASS.** The service (the GNOME helper that owned `org.freedesktop.Notifications`, pid 370590) was terminated; `NameHasOwner` = false. C finished a turn: the client sent `Notify` on the bus (error: no owner), **no banner, no in-app notice** (screenshots), C's row marked. The client log has one line, `desktop notification not shown (no notification service: The name org.freedesktop.Notifications was not provided by any .service files); later failures in this run are not logged`; a second turn end added none (1 match after 2 turns). The file's lines carry no level or timestamp, so "warn" is not confirmed from the file. | `b13-grid.png` (editor covers C's mark there), `b13-4-editor-away.png` (mark on row 1, no notice), `logs/micold-client.log.txt` |
| B10 | NOT RUN here (M6 passed it; no new observation under a WM) | M6 |

## Not observed

- Why the minimised X11 raise takes ~2.7 s, and whether it is specific to the debug build or X11 (Wayland: 25 ms).
- The "first window raises, second window holds the project" banner click (3c).
- A real X11 desktop other than GNOME Shell 50.1/mutter on Xwayland; a physical seat; another workspace; a release build.
- What an attention request looks like on X11 (none was sent; `_NET_WM_STATE_DEMANDS_ATTENTION` was never set in the polled states).

## Files

`x11-b9-wgpu-grid.png`, `x11-b9-grid.png` (tiny-skia), `x11-unmin-grid.png`, `x11-b11-grid.png`, `x11-b11-owner-grid.png`,
`x11-switch-grid.png`, `wl-unmin-grid.png`, `b13-grid.png`, `b13-4-editor-away.png`; `logs/` holds the polls
(`*-active.txt`, `wl-*-focus.txt`), drive excerpts (`*-drive.txt`), bus capture, client/daemon log excerpts, the strace and
Wayland trace excerpts, and the harness as `*.sh.txt` / `pollw.py.txt` (adapted from `../M7/harness-scripts.txt`).
Other files in this directory belong to other passes.
