# Visual pass, quickstart Part B (#1732, T041)

2026-11-18, cloud session, Google Chrome (Playwright `channel: chrome`), 1280x800. The stack was
`scripts/local-stack.sh` in process mode (real orchestrator, real daemon, real web export); the
daemon ran under a wrapper so it could be stopped alone for the unreachable marker.

| Shot | Step | Seen |
|---|---|---|
| 01-sign-in.png | B2 | sign-in with the one local provider |
| 02-list-connected.png | B2 | daemon `local`, project `demo`, worktrees `demo` and `feature`, both seeded sessions, Overview cards |
| 03-content-view.png | B2 | the session opened: title, Refresh, "This session has no content yet." |
| 04-daemon-unreachable.png | - | daemon stopped: both rows and the open session marked Unreachable in red |
| 05-reconnecting.png | B3 | orchestrator stopped: "Connection lost. Reconnecting (attempt N)…" |
| 06-cannot-reach.png | B3 | retries spent: "Cannot reach the orchestrator" with Retry; the list stays as last known data |
| 07-reconnected-after-retry.png | B3 | orchestrator restarted, Retry: banner gone, list back |
| 08-polish.png | B4 | Polish: Przegląd, Odśwież, Zatrzymana, Strukturalna, Połączony, "Ta sesja nie ma jeszcze treści." |

B5 (second copy refused) and B6 (Ctrl+C stops all three) are covered by
`scripts/tests/local-stack.test.sh`.

Found and fixed: Retry in the unreachable banner did nothing against the real API. The banner
destructured `retry` from the class-based `RealOrchestratorApi`, so it ran without `this` (the mock's
plain-object API hid it). Regression test in `tests/components/ConnectionBanner.test.tsx`.

Noted, not fixed here: in 04 the daemon header still reads "Online" while its sessions are
Unreachable; after the orchestrator restarts, the snapshot reports the daemon's real status.
