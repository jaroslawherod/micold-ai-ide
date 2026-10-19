# Visual pass, feature 938 M8 (T046)

Run on a private Xvfb with the software renderer (`ICED_BACKEND=tiny-skia`), a real `claude` on
`PATH`, a seeded one-project catalog. Client and daemon from the same build.

| Step | Result |
|---|---|
| 1. Chevron with only Claude Code installed | Pass: `01-chevron-list.png` — "Claude Code" then "Claude Code (structured)", the other CLIs disabled with their reason. |
| 2. Pick it | Partly: a structured session starts (daemon starts `claude -p --input-format stream-json …`), its AI tab reads `claude, structured`, the pane is the read-only log with no input box (`02-…-light.png`). The log shows "No events yet.": the CLI sends its init line with the first user message, and this milestone has no input to send one. The per-event lines are covered by `features_structured_log` and the `log_line` tests. |
| 3. Plain `+` press | Not run: the plain press is unchanged code (terminal kind) and is asserted by `structured_start_menu`. |
| 4. Stop/start, daemon restart | Not run (no event to keep; covered by the daemon's `structured_saved_history` tests). |
| 5. Second window | Not run (covered by the daemon's late-viewer tests, M5). |
| 6. Light and dark | Pass: `02-…-light.png`, `04-…-dark.png`; the empty-state line and the tab marker read in both. Settings note: `03-settings-note.png`. |

Found by this pass and fixed: the client's catalog fold built every daemon-reported session with
`Session::restored`, so a structured session was a terminal one in the client (tab said `claude`,
pane showed "Starting…"). `catalog_sync` now keeps the summary's kind; test
`a_reported_structured_session_keeps_its_kind` fails without the fix.
