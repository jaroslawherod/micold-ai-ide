# Visual pass record — quickstart Part B

Run 2026-10-09 in a private headless sway (lavapipe), binaries built from the branch, scratch
`XDG_DATA_HOME`, fake `pi` CLI. Screenshots are beside this file.

| Step | Result | Evidence | Note |
|---|---|---|---|
| B1 | pass | b1-separator.png | Lines 1-200, one dim separator, new lines 1-200 |
| B2 | pass | b2-first-separator.png, b2-second-separator.png | Both separators read 17:43 (minute granularity); a later restart in B5 read 17:47 |
| B3 | pass | b3-light-separator.png | Readable in both themes |
| B4 | pass | b4-two-windows.png | Same history in both; second window opens read-only |
| B5 | pass | b5-narrow-separator.png | Separator one row |
| B6 | pass | b6-terminal-settings.png (earlier: ../visual/b6-terminal-*.png) | |
| B7 | pass | b7-checkbox-off.png, b7-off-history-still-shown.png | No match for the output; directory empty |
| B8 | pass | b8-off-stop-start-has-separator.png, b8-off-after-service-restart.png | |
| B9 | pass | b9-damaged-notice.png | Warning verified in the daemon log, not a GUI list (see deviations) |
| B10 | pass | b10-session-removed.png | File deleted |
| B11 | pass | none (stat output) | `drwx------`, `-rw-------` |
| B12 | not run | none | Pi Coding Agent not installed |
| B13 | pass | b13-claude-no-flicker-0.png | Earlier conversation above the separator, resumed view below |
| B14 | Claude pass; Copilot not run | b14-claude-default-after-restart.png | Full-screen view, no separator visible while it runs (D11); Copilot not installed |
| B15 | covered | none | Image builds; `sandbox_real_history.rs` in CI's sandbox job covers it |

Manual Windows checks (logout/reboot stop, DACL seen by a second account, stop and start of an AI CLI
session): not run, no Windows machine; covered as far as CI can by the `cfg(windows)` tests of
stop-request §6 and `owner_only`.

## Deviations

- "Restart service" appears only on a version-mismatch banner; SIGTERM was sent to the private service
  instead (what `terminate_daemon` sends on Linux).
- The fake `pi` writes a Pi session file under `$HOME/.pi/agent/sessions/`, otherwise Pi's empty-session
  pruning removes the session and its history.
- "Stop" was the process exiting, then the restart link ("x" removes the session).

## Observation

After a service restart, or a resize while scrolled back, the terminal sometimes paints blank rows for
lines that exist; re-selecting the session repaints them. Seen with saving off and no seed (B8), so
probably not this feature; no baseline from main was built. Follow-up, not in scope.
