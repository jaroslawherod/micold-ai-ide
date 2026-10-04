# M9 quickstart C3: session service in a container (SC-007), 2026-10-04

Tree 0f28777a. Image `micold-daemon:dev` built by `mise run image` from this tree. Client and daemon binaries for the GUI part: the pair pinned in `~/vp/m9bin` (copied, not rebuilt). Docker is the runtime.

## Real-runtime sandbox suite (`mise run test-sandbox`)

EXIT=0. 14 test targets, 29 tests, 29 passed, 0 failed (`C3-test-sandbox-summary.txt`). Targets: enable 1, handshake 3, lifecycle 8, storage 1, ai_cli 1, ai_cli_sessions 2, boundary 1, fingerprint 1, idle 2, limits 2, mcp 1, parity 1, session_start 1, staleness 4.
None of them drives attention notifications; they cover the sandbox, not feature 039.

## GUI route (what is real and what is a stand-in)

| Part | Real | Stand-in |
|---|---|---|
| Session service | `micold-daemon` in container `micold-sandbox` (image `micold-daemon:dev`), brought up by the client itself from `settings.json` `daemon.placement = local_sandbox`, `image.kind = local_build` | |
| Client | the real client on Xvfb `:192` (private bus, HOME and XDG dirs), talking to the container on 127.0.0.1:7727 | not a real display, no window manager |
| Sessions | started from the sidebar `+`; spawned by the in-container daemon | `claude` in the container replaced by `sleep` (`docker exec -u 0`); turns are `UserPromptSubmit` then `Stop` POSTed to the session's loopback hook URL from inside the container (`docker exec ... node`, the host cannot reach that port) |
| Notification service | none installed | python3 dbus-python object on the private bus: `Notify` logged, `ActionInvoked(id, "default")` emitted on demand |

Setup: projects P and Q (git). P has A (selected) and B (not selected, first row); Q has C.

## Results

| Step | Result | Evidence |
|---|---|---|
| B1 | PASS. B's turn (A selected): exactly one Notify, id 1, summary `New session is waiting for input`, body `P — Default`, actions `default`/`Open`. Same shape as M8 step 4 (`<session name> is waiting for input`, `<project> — <worktree>`; the session is unnamed so its name is `New session`). | `C3-notify-service.txt`, `C3-B1-B6-rows.png` |
| B6 | PASS. B's row shows the attention marker and the unread dot, A's row none. Project panel: `P 2 running / 1 unread`. With Q active the header chip shows `Q  ● 1`. | `C3-B1-B6-rows.png`, `C3-B6-panel-count.png` |
| B9 | PASS for content; front/focus NOT OBSERVED. P active, C of Q finished (Notify id 4, `Q — Default`). `ActionInvoked(4, default)` on the bus: Q active, C selected, C's unread dot still shown. After the window was given input focus (`xdotool windowfocus`) the dot went; the attention marker stayed. | `C3-B9-1-before-click.png`, `C3-B9-2-after-click.png`, `C3-B9-3-after-focus.png`, `C3-dbus-monitor.txt` |

## Against the host results

B1, B6 and B9 equal the host results in M2 (notification title/body/actions, one per event), M5 (unread mark and panel count) and M6 (click switches project and selects the session; mark stays until the window has focus, then clears). I saw no difference attributable to the container. Window raise and input focus are NOT OBSERVED, as in M6 (no window manager); that part of C3 still needs a real desktop.

## Seen on the way (not asked)

- Two clients on the same display (I had killed the wrapper pid of the first launch, not the client, so it kept running) split the work: the click was handled by the other window and the visible one did not change. That is the multi-window rule (the holder window raises and handles), not a container effect; after the stray client was killed by pid the result above was reproduced cleanly. The first two clicks (ids 2 and 3) are from that state and are not counted.
- The client showed the banner `The sandbox is out of date: The sandbox does not yet share every registered project. Restart it to apply.` for the whole run although the container had been recreated with both project mounts (`docker inspect` lists P and Q). Not examined; I did not click Restart (it would have dropped the stand-in `claude`).
- Rendering of the window only updates on input events on this display: after an `ActionInvoked` the screenshot showed the old state until the pointer moved. Screenshots above were taken after a pointer nudge.

## Not observed

Raise/focus on a real desktop; a real notification service; light scheme; a second window; Pi/Copilot sessions in the container.

## Cleanup

Container `micold-sandbox` stopped and removed; Xvfb, dbus-daemon, stand-in and both clients stopped by recorded pid; `~/vp/m9c3` and `/tmp/vp192` removed. `micold-daemon:dev` image left. Not mine and untouched: the other containers and the `micold-sandbox-net` network (created 2026-09-26), `~/vp/m9bin`, `~/vp/m9run`.
