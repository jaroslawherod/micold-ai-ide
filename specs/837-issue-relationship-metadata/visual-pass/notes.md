# Visual pass, feature 837, quickstart Part B (steps 1, 2, 3, 5, 6)

Date 2026-10-16. Real `micold-ai-ide` + `micold-daemon` built from the working tree (pair pinned out of
`target-shared`), run on a private Xvfb with `ICED_BACKEND=tiny-skia` (software), input by python3 ctypes over
libX11/libXtst. Not a real display, not a real GitHub: a fake `gh` first on PATH answered `auth status`, the
issue list, the single-issue lookup and the details query with canned JSON (shape of
`crates/micold-core/tests/fixtures/issue_details/every_kind.json`, minus Blocking and the timeline, so
I-blocked has exactly the 7 rows). Remote `https://github.com/example/demo.git`. Window was resized to
1200x1000 with XResizeWindow (no WM). The app repainted very slowly (one core pinned, 5-30 s per frame), and
several screenshots caught the dialog mid-animation or half-repainted; verdicts are from settled frames.

| Check | Verdict | Evidence |
|---|---|---|
| 1 Pick I-blocked: rows in order, red label, "Blocked:", Create enabled | PASS | `b1-full-dialog-light-1200x1000.png`, `b1-blocked-rows-top.png`, `b1-blocked-rows-scrolled.png` |
| 2 Pick I-plain: rows gone, none for I-plain | PASS | `b2-plain-no-rows.png` |
| 3 Create right after picking works | PASS with a caveat | worktree created (git worktree list, sidebar row) |
| 5 Failed details: one muted caption, Create works | PASS | `b5-failed-caption.png` |
| 6 Warning label legible light and dark | PASS | light: `b1-*`; dark: `b6-dark-blocked-label.png` |
| Dialog stays in window with 7 rows | PASS at 1200x1000; NOT CONFIRMED at the default 1024x768 | `b1-full-dialog-light-1200x1000.png` |

Details.

1. Rows, top to bottom: Blocked by ("Blocked: 1 open of 2: #10 Parser rewrite (open), #9 Old spike
   (closed)", label red), Parent, Sub-issues ("1 of 2 closed: ..."), Closing pull requests ("#20 ... (draft)"),
   Assignees (octocat), Milestone (v1.4), Projects ("Roadmap: Status In progress, Estimate 3"). The block
   scrolls (a scrollbar, about 4 rows visible); the first frame shows the first three, after scrolling the last
   five. Create stayed enabled. In the first light frames references read `acme/widgets#10` because my canned
   JSON named a different repo than the remote; I corrected the JSON and later frames (dark) read `#10`.
   Minor look: in `b1-blocked-rows-top.png` the "(closed)" at the end of the Blocked by text sits flush
   against the scrollbar.
2. After picking I-plain the block vanished. Create was disabled there only because I-plain has no label and
   so Type was empty (existing Type rule, not this feature).
3. I-blocked-b / -c were picked and Create pressed 1-2 s later; both worktrees were created
   (`fix-103_...`, `fix-104_...`) and appeared in the sidebar with the usual `fix #103 Host` chips. The fake
   details call was made to hang (slow mode), which the app cut at 10 s ("GitHub didn't answer within 10
   seconds" caption, an extra observation: the timeout text is shown through the same caption). Because of the
   frame lag I cannot prove the click landed during Loading rather than after the 10 s failure, so this shows
   "Create works with the details unresolved/failed", not strictly "during Loading". The earlier attempt
   (103) had in fact clicked after the failure.
5. Fake `gh` failed the details call only (list and lookup still worked). One muted line under the picker:
   "Couldn't read this issue's details. Couldn't reach GitHub. Check your connection, then retry."; no rows.
   Create worked (`fix-101_...` created).
6. The theme is switchable through `settings.json` (`"theme":"dark"`, relaunch). Light (default) and dark:
   the "Blocked by" label is a red tint and legible in both (smaller and lighter on dark, still readable).
7. Not exercised: step 4 (projects scope "Not readable"), not in the requested list; I-blocked with the
   Blocking/timeline rows (more than 7 rows); the 1024x768 default window. At 1024x768 the 7-row dialog
   looked taller than the window (title near the top edge, footer cut off) in the frames I got, but the
   repaint was stale then, so I cannot tell a real overflow from a damage artefact. At 1200x1000 the whole
   dialog, footer included, fits and nothing overlaps the app bar. Recommend a re-check at the minimum
   window size by a human or on a real display.
