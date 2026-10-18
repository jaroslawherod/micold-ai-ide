# Quickstart Part B close pass (T066)

Run 2026-10-17 on headless sway (pixman) + lavapipe, dark scheme only, client and daemon from the worktree build, `fake_jira` on 127.0.0.1:8099, private data home, scratch git repo with a GitHub origin. B3, B5, B6 and the wrong-site arm of B7 are in [M3.md](./M3.md).

- B1 PASS: default settings show no Jira control but the off switch; dialog has only New branch / Existing branch / GitHub issue. Evidence: B1-dialog-default.png, B1-settings-pages.png, B1-settings-jira-off.png.
- B2 PASS: enable, connect, save, restart: Settings > Jira shows "Connected to http://127.0.0.1:8099 - ABC", fields kept, token field empty with "A token is stored". Evidence: B2-after-restart.png.
- B3 PASS: see M3.md.
- B4 PASS: GitHub source (rate-limit/timeout notice) switched to Jira replaces the list and copy; query "login" is cleared by GitHub then Jira; reopened dialog still on Jira. Evidence: B4-1-github-source.png, B4-2-switched-to-jira.png, B4-3-query-cleared-on-switch.png, B4-4-reopened.png. Reopen resets the mode chip to New branch (source remembered).
- B5 PASS: see M3.md.
- B6 PASS: see M3.md.
- B7 FAIL: two windows ran (second is read-only until Take over). (a) Turning the switch off over a stored connection and pressing Save does nothing visible: the Keep-or-Remove question is raised but is only rendered inside `connection_form`, which `ui/settings/jira.rs` view() draws only when `draft.jira.enabled`, so with the switch off the prompt is invisible and the switch can never be saved off (US2 AS5). Evidence: B7-switch-off-save-no-prompt.png (settings.json stayed jira_enabled true, no daemon write). (b) Propagation could therefore only be measured with Remove connection: the other window's open Jira picker was still shown 1.05 s after the click and gone by ~4 s (no frame between); SC-006 "within a second" not shown. Evidence: B7-B8-left-window-t0.34-t1.05-t4.png. Wrong-site arm: PASS in M3.md.
- B8 PASS: Remove connection + Save: token file deleted, jira_connections empty, dialog shows no Jira chip. Evidence: B8-token-file-gone.txt, B7-B8-left-window-t0.34-t1.05-t4.png.
- B9 PASS (Linux): jira dir 0700, token file 0600, owner jaro; token text only in the token file (not in settings.json or logs). Evidence: B9-token-perms-linux.txt. macOS and Windows: missing access.
- Light scheme, narrow window: missing access / not run.

## B7 re-run after the Keep-or-Remove fix (T079, 2026-10-18)

Same setup (headless sway, two windows, `fake_jira`), client built at the M6 tree.

- Switch off over a stored connection + Save: PASS. The question "Keep the Jira connection for repo?" with Keep and Remove is drawn. Evidence: B7r-1-keep-or-remove-prompt.png.
- Keep: PASS on state (jira_enabled false, connection and token file kept; B7r-2-keep-state.txt). Remove: PASS on state (connections empty, token file gone; switching back on returns the picker).
- SC-006 propagation: NOT MET. The daemon wrote the setting 0.13 s after the click, but the other window dropped its picker 3.3 to 3.7 s after it (five measurements, B7r-timings.txt, B7r-3-remove-other-window-t3.35-picker-still.png, B7r-3-remove-other-window-t3.49-picker-gone.png). Cause not found: `adopt_daemon_settings` runs `sync_jira_availability` synchronously, so the lag is in the echo or the redraw, not the Jira code; unchecked whether a non-Jira setting shows the same lag. Open follow-up; the earlier run (B7, 1.05 s still shown) saw the same gap.
