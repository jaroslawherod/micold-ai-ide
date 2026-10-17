# Visual pass: quickstart Part B, steps 1-5 (feature 805)

Date 2026-10-17. Real client (`micold-ai-ide`, debug build of this branch at 7746676b, client and
daemon built together) on a private headless sway (pixman + lavapipe), private XDG data and runtime
dirs, pointer by `vptr.py`, keys by `wtype`. Not a real display.

| Step | Result | Evidence |
|---|---|---|
| 1 Add daemon, Container, empty Port: hint says a free port is chosen; Save: no port error | pass | b1-port-hint.png (focused Port shows "A free port is chosen if empty"), b1-saved-with-picked-port.png (saved as 127.0.0.1:40035, no error) |
| 2 Edit: Port shows a number; clear and save: unchanged | pass | b2-edit-shows-port.png (40035), b2-after-clear-and-save-unchanged.png (still 40035) |
| 3 Second container daemon, empty port: different number | pass | b3-second-daemon-different-port.png (40035 vs 45819) |
| 4 Port `0`: "Enter a port between 1 and 65535." | pass | b4-port-zero-rejected.png |
| 5 Occupied port, start daemon: error names port, says change in Settings > Daemons | not run (inconclusive) | Port 40035 was held by `python3 -m http.server`, then Start was clicked on p1. The row stayed "Starting" for over 4 minutes with no container created and no error; Details showed "state: Starting", no connection errors. The expected error never appeared, so the text was not observed. Likely needs a working image pull/container runtime (docker is the podman shim, ghcr image `micold-daemon:0.18.0`, not available here). b5-start-stuck-starting.png |

Notes: the first typed character was sometimes dropped by `wtype` (names are p1/p2/p3). "Connect when
the application starts" was unchecked on the saved daemons. The hint is the field placeholder, so it
shows only while the field is empty.
