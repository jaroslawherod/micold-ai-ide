# Quickstart §B close pass (T040)

Every step ran headless with the `visual-pass` recipe: Xvfb with the tiny-skia renderer, client and
daemon pinned from the worktree build, the dev-only `fake_jira` site, a private data dir and a
scratch git repo. B1–B11 ran with their milestones; B12 and B13 ran at close (2026-10-25, binary
built at `b841af58`). Screenshots are in the project files under `issue-1186/`.

| # | Verdict | When | Evidence / note |
|---|---|---|---|
| B1 | PASS | M3 | `m3-B1-no-switch-integration-off.png`. Not run: the GitHub-source variant (the scratch repo had no remote) |
| B2 | PASS | M3 | `m3-B2-basic-switch-list.png`, `m3-B2-advanced-switch-query-run.png`, `m3-B2-advanced-results.png`; rows only after Enter/Run, as basic rows. The scoped query text is covered by the `jira_query` and `jira_http_adapter` tests, not read from the fake's log |
| B3 | PASS | M3 | `m3-B3-filled-form.png` (ticket, name, type filled). Not observable: the sort order, since the fake ignores `ORDER BY`; the kept clause is covered by `jira_query` tests |
| B4 | PASS | M4 | `m4-B4-quote-refusal.png`, `m4-B4-parens-refusal.png`, `m4-B4-length-refusal.png`; fake logged nothing |
| B5 | PASS | M4 | `m4-B5-badjql-message.png`, `m4-B5-fixed-results.png` |
| B6 | PASS | M4 | `m4-B6-unreachable-retry.png`, `m4-B6-after-retry.png` |
| B7 | PASS | M3 | `m3-B7-back-to-basic.png` |
| B8 | PASS | M5 | `m5-B8-reopen-advanced-last-query.png` |
| B9 | PASS | M5 | `m5-B9-second-project-basic-empty.png` |
| B10 | PASS | M5 | `m5-B10-after-restart-advanced-query.png` |
| B11 | PASS | M5 | `m5-B11-after-remove-connection.png`; A's `.query.json` and `.token` gone, B's token kept |
| B12 | PASS | close | Light and dark, at 800 px and 640 px (the app's minimum), plus light at 1400 px: the switch, query field, Run, Jira's error line (`badjql`) and the unreachable notice with Retry fit, without clipping or overlap, in the theme's colours. `close-B12-{light,dark}-{800,640}-advanced-{badjql-error,unreachable-retry}.png`, `close-B12-light-1400-*.png` |
| B13 | PASS (Linux) | close | `close-B13-perms-linux.txt`: `jira/` 0700, `<id>.query.json` 0600, written by the app after an Advanced run (the pass pre-created the folder and token, so the folder's mode is the 533 token code's, not shown here). macOS and Windows: missing access, not run |

Noted, not defects of this feature: below about 900 px the tiny-skia software renderer leaves stale
regions after a resize (each screenshot is the first clean frame); each run sends two requests,
because the fake always returns a next-page token.
