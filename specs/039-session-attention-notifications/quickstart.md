# Quickstart: Notify When a Session Needs Attention, and Track Unread Sessions

**Feature**: 039 | **Plan**: [plan.md](./plan.md) | Contracts: [wire](./contracts/wire.md),
[desktop notification](./contracts/desktop-notification.md), [unread mark](./contracts/unread-mark.md)

## Prerequisites

- `mise trust` once in a fresh worktree.
- For §B: the `visual-pass` skill's private display, pinned binaries and private data directory.
  A notification service on that display's session bus (`dunst` or `mako`); without one, B1
  checks the FR-010 path instead.
- For §C: a macOS bundle from `mise run app` (ad-hoc signed) and an installed Windows build.

## §A — Automated

```sh
mise run test-core     # in_view, AttentionTracker, notification_text, resolve_reveal, counts, wire
mise run gate          # daemon Views and integration, client reducers, component gates, fmt, clippy
```

Expected: both green. The tests named in [plan.md](./plan.md#test-strategy-by-layer) exist and
fail without the change they cover.

## §B — Recorded pass on Linux (visual-pass)

Two projects, P with sessions A and B, Q with session C. Record a screenshot for each step that
names a look, in the light and the dark scheme.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Select A. Let B finish a turn. | One notification: title `<B> is waiting for input`, body `<P> — <worktree>`. | US1.1, FR-004 |
| B2 | Let A finish a turn with the window focused. | No notification, no mark on A. | US1.2, US2.2 |
| B3 | Focus another application. Let A finish a turn. Return. | One notification; A marked until the window regains focus. | US1.3, US2.6 |
| B4 | Open Settings. Let A finish a turn. Leave Settings. | One notification; A marked, then not. | US1.9, US2.11 |
| B5 | Let C finish a turn. | Notification naming Q. Button shows `● 1`. Panel: Q `● 1 unread`. | US1.4, US2.4, US2.15 |
| B6 | Let B finish a turn. | B's row: mark at the trailing edge, label emphasised, activity indicator unchanged. Panel: P `● 1 unread` under its running count. Button still `● 1`. | US2.1, US2.5, US2.16, FR-018, FR-032 |
| B7 | Select B. | Mark and P's count gone within 1 s. | US2.3, SC-006 |
| B8 | Close the window. Let A finish a turn. Open the application. | No notification, then or now. A is marked unless it is the session shown. | US1.13, US2.18, SC-009 |
| B9 | Click C's notification from another application (X11). | Window in front and focused, Q active, C shown, C's mark gone. | US3.1 to US3.3, SC-004 |
| B10 | Remove C, then click an older notification for it. | Window in front, selection unchanged, notice `That session is no longer available.` | US3.4 |
| B11 | Open a second window on Q. Click a notification for a session of Q from the first. | The second window comes forward and shows it. | US3.6 |
| B11a | Two windows. Let the first raise a notification, close the first, click the notification. | The second window does not change; the session keeps its mark. | N9 |
| B12 | Settings → Environment: the **Desktop notifications** switch is on. Turn it off. Let B finish a turn while not in view. | No notification; B marked. Turn it on: the next change notifies. | US4.1 to US4.5 |
| B13 | Stop the notification service. Let B finish a turn. | No notification, no in-app notice, B marked, one `warn` line in the log for the run. | FR-010 |
| B14 | Showcase: the three `UnreadMark` entries, both schemes. | As in the contract. | FR-030 |
| B15 | With A selected, let B and C finish a turn; open no session. From the sidebar and the switcher alone, say which sessions finished a turn not yet looked at, and in which projects. | The answer is B in P and C in Q, read from B's mark, the button's `● 1` and the panel's `● 1 unread` on P and on Q. | SC-008 |

## §C — By hand

| # | Where | Do | Expect |
|---|---|---|---|
| C1 | macOS bundle | B1, B2, B9. Deny notifications in System Settings, repeat B1. | Shown and clicked as on Linux; when denied, B13's outcome. |
| C2 | Windows, installed build | B1, B2, B9; click a toast from the notification centre. | Shown under the application's name; the click shows the session. Record whether the notification-centre click is reported. |
| C3 | Linux, session service in a container | B1, B6, B9. | Same as with the service on the host (SC-007). |
| C4 | Linux, Wayland session | B9, then B11 (the token crosses to the second window). | Focus given, or the window marked as needing attention; record which, for each. |

## Record

Recorded in M9 from the evidence in `visual-pass/` (M2, M4, M5, M6, M7, M8, M9). Every pass ran on
Xvfb or a private headless GNOME Shell on this machine, with `claude` replaced by a stub and turns
driven by POSTing `UserPromptSubmit` then `Stop` to the session's hook URL; most used a python
`org.freedesktop.Notifications` stand-in, and where a real service ran it is said. "Not observed"
means no run saw it. Dark = dark scheme, light = light scheme.

| Step | Result | Evidence |
|---|---|---|
| B1 | PASS dark (M2), PASS light (M9). Summary `<B> is waiting for input`, body `P — Default`. Real banner (GNOME helper) seen in M7/M9 wm-pass | `M2/B1.png`, `M9/B1-light.png`, `M9/light-pass.md` |
| B2 | PASS dark (M2). Light: not observed | `M2/B2.png` |
| B3 | PASS dark (M2): one Notify with another application focused. Light: not observed | `M2/B3.png` |
| B4 | PASS dark (M2): one Notify with Settings open; the mark clearing on leaving Settings is covered by tests, not seen here. Light: not observed | `M2/B4.png` |
| B5 | PASS dark (M2, M5), PASS light (M9): Notify naming Q, button `P ● 1`, panel Q `1 unread` | `M5/B5-button-dark.png`, `M9/light-pass.md` |
| B6 | PASS dark (M4, M5), PASS light (M9). Activity indicator judged unchanged by eye, not diffed | `M4/B6.png`, `M5/B6-rows-dark.png`, `M9/B6-rows-light.png` |
| B7 | PASS dark (M4, M5), PASS light (M9): mark gone at the first sample, 0.15 s (SC-006: within 1 s) | `M5/B7-before-after-dark.png`, `M9/B7-after-light.png` |
| B8 | PASS dark (M4), PASS light (M9), both branches (shown session finished with no window open: unmarked; another session finished: marked, no Notify) | `M4/B8.png`, `M9/light-pass.md` |
| B9 | PASS on Linux X11 under a real window manager (M9, mutter in a headless GNOME Shell 50.1, real banner, pointer click): in front, focused, Q active, C shown, mark gone, 24 ms (SC-004: within 2 s). PASS on Wayland (M7, GNOME Shell 50.1): focus given, no attention request. **Un-minimise on X11: 2.7 s in three runs (a debug build, mutter on Xwayland), so SC-004's 2 s is missed for a minimised window there**; the window is un-minimised, in front and focused, and the cause is not diagnosed (follow-up; ledger D32). Un-minimise on Wayland 25 ms. Before M9: the selection and mark only, front/focus not observed | `M9/wm-pass.md`, `M7/README.md`, `M6/README.md` |
| B10 | PASS dark and light (M6): selection unchanged, notice `That session is no longer available.`. Not repeated in M9 | `M6/B10-notice-crop.png`, `M6/run2-B10-…png` |
| B11 | PASS for content (M6) and, through a real banner on X11, for the cross-window reveal (M9: the holder window came forward in 1.29 s showing C). The step as written (first window raises, second holds the project) was not observed with a real banner: the later-started client raised the banner. On Wayland (M7) the second window came forward through the notification list | `M6/B11-before-after.png`, `M9/wm-pass.md`, `M7/B11-*.png` |
| B11a | PASS (M6, run 1), with three caveats. The raising window was destroyed with `xdotool windowclose`, not closed with its button, and its client ended with a wgpu panic (`M6/README.md`, *Seen on the way*), so what was seen is a click after the raising process had gone, not after an orderly close. The window that stayed was the read-only one (the screenshot shows its `Another window took over this project` banner). A first attempt, where the second window had raised the notification, is not counted. Not repeated | `M6/B11a-before-close-after-click.png` |
| B12 | PASS dark (M8) and light (M9). Not observed: a Pi or Copilot session; a service restart with the switch off (the client was restarted, the service kept running; the integration tests cover the service restart) | `M8/notes.md`, `M9/light-pass.md` |
| B13 | PASS dark (M2) for the notification part only: no `Notify`, no in-app notice, one log line. `M2/B13.png` shows no mark on B because the M2 build had none (the unread mark came in M4), so "B marked" was not seen there. PASS light (M9, stand-in stopped; the mark was already set, so it does not discriminate). PASS with GNOME's real service stopped (M9): no notification, no in-app notice, the row marked (`M9/b13-4-editor-away.png`; in `M9/b13-grid.png` the editor covers the mark), one `not shown` log line (the line carries no level in the file, so `warn` is not confirmed from it) | `M2/B13.png`, `M2/README.md`, `M9/B13-light.png`, `M9/client-warn-line.txt`, `M9/b13-4-editor-away.png`, `M9/wm-pass.md` |
| B14 | PASS dark and light (M4, M5, M9): the `UnreadMark` entries and the switcher panel, no glyph collision | `M4/showcase-*.png`, `M5/showcase-*.png`, `M9/` |
| B15 | PASS light and dark (M9): B in P and C in Q read from the sidebar and the switcher alone, about 3.5 s from C's Stop to the panel screenshot | `M9/B15-light.png`, `M9/B15-dark.png` |
| C1 | **Follow-up**: no Mac at hand. CI covered the backend's unit tests on macOS (`Test (desktop notification backends)`, success on `7cbb6c76`, and the macOS build and test job). Not run: delivery, the click, denied permission, D26 (response arrives while winit runs the main loop; the removal after one hour). Steps in the ledger's *Follow-ups not done* | ledger |
| C2 | **Follow-up**: no Windows machine at hand. CI covered the backend's unit tests on Windows (same step, success; the Windows build and test job, and the arm64 package and smoke job). Not run: the toast, the click, the click on a toast in the notification centre (research R4 stays **Unverified**; the sentence in `docs/user-guide/install-windows.md` stays as T090 wrote it), D27, the AppUserModelID on the desktop shortcut and the installer's launch | ledger |
| C3 | PASS for content (M9): `mise run test-sandbox` 29 of 29; B1, B6 and B9 with the service in a container gave the same as on the host (SC-007). Raise and focus under a window manager with the container service: not observed. The client showed `The sandbox is out of date: The sandbox does not yet share every registered project` for the whole run. That is the sandbox's `Stale` state (features 027 and 028): the client sets it when a project is registered, or the keep-running setting changes, while its sandbox runs. Which event set it, and why it stayed while the container had both mounts, was not established (two clients ran; the service restarted once, at 09:34:56; the pass did not click Restart). It is not a version mismatch (the handshake succeeded, and that failure has another text), and feature 039 changes no sandbox code (ledger D33) | `M9/container-pass.md` |
| C4 | PASS on Wayland (M7, GNOME Shell 50.1 only): B9 focus given; B11's second window got the focus, and an attention request followed 410 ms later although focus was given (nothing visible). Other compositors, a physical seat, a minimised window on Wayland through a banner: not observed | `M7/README.md` |
