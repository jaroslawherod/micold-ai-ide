# Visual pass — Review tab (FR-009, FR-010, SC-003)

- **Date**: 2026-10-27
- **Build**: release `micold-ai-ide` + `micold-daemon` from one build of this branch (fixes of
  f8c46db0 included), copied out of `target-shared/` before launch; the daemon log read
  `client attached to daemon`.
- **Environment**: Linux cloud container, Xvfb `:77` at 1280×800×24, no window manager, software
  rendering (tiny-skia; lavapipe installed). Not a real display: colour and geometry are what the
  renderer drew, frame pacing and animation were not judged.
- **Window**: 1280×800, light theme, then a fresh data directory in the dark theme.
- **World** (T026, all invented): project `widgets` with worktree **A** `feat/scheduler-priorities`
  (15 changed files: nested folders, the deep path `src/server/handlers/v2/internal/admin/audit`,
  the long name `priority_queue_implementation_details/weighted_round_robin_fairness_strategy.rs`, a
  binary `assets/logo.png`, a deleted file; 2 drafts, 2 sent of which 1 answered with a reply;
  3 files marked reviewed and `cache.rs` marked then changed on disk), worktree **B**
  `fix/lexer-unicode` with its own Review tab and `tokens.rs` selected, worktree **C**
  `docs/idle-worktree-with-a-deliberately-long-name-for-the-overview-card` with no changes, a stub
  `gh` first on `PATH` that lists PR #57 with a long title for A and fails its diff (HTTP 502), and a
  stub `claude` that prints forever (never ready, so a send that starts a session stays *Sending…*).
- **Driving**: xdotool (mouse with a held press, keys after `windowfocus`), `import` for frames; each
  crop below was looked at, light and dark.
- **Not run**: macOS and Windows: no host of either was available to this run. The pass was not
  repeated at other widths (FR-009; FR-007's gates cover 640×480).

## Verdicts

| NN | Check | Verdict | Notes | Fix / issue |
|---|---|---|---|---|
| 01 | Review tab and badge in the strip | PASS | `Review` with its `2` badge after the session tab and before **+**; badge legible in both themes; no collision | — |
| 02 | Header: source control and *n of m reviewed* | FAIL → fixed; FAIL → issue | Before the fix the PR row's long title wrapped and its second line was cut by the row's fixed height; now one line ending in "…". Control and *3 of 15 reviewed* fit beside the title. Separately, the control's own tooltip opens over its open list while the pointer stays on it (text overlaps the list's first rows) | f8c46db0 (`PullRequestRow`); #1539 |
| 03 | File tree | PASS, with an issue | ✓ reviewed, ○ unseen, ◉ selected, *2 comments* tag and the changed dot are distinct in both themes; filter box; `+a −r` bars; deep path and long name end in "…" without overlap. The rows run under the list's scrollbar: the selected outline passes under it and the changed dot sits flush against it | #1540 |
| 04 | Diff with cards | FAIL → issue | Bundled monospace face; inline draft card; answered card with its reply. The cards are as wide as the longest diff line, so **Delete** / **Answer…** sit past the viewport until the diff is scrolled sideways | #1519 |
| 05 | Review bar idle | PASS | Above the status bar: *2 drafts · 1 sent · 1 answered*, **Comments**, ⋮, **Send review (2)** | — |
| 06 | Review bar *Sending…* | PASS | **Sending…** (disabled) while the started session is not ready; the bar keeps its height | — |
| 07 | Review bar error | FAIL → fixed | The error ends in "…" past the bar's room; before the fix nothing showed the rest once the snackbar left. Hovering it now shows it whole | f8c46db0 (`ReviewBar`) |
| 08 | Drawer on **Drafts** | FAIL → fixed | Over the diff, the diff not narrowed. Before the fix a draft row's **Edit** / **Delete** were pushed past the drawer's 360 px by the path and tag; now on their own line under the text | f8c46db0 (`ReviewPanelRow`) |
| 09 | Drawer on **Replies** | PASS | Over the diff; the reply whole on the answered row; **Mark applied** / **Answer…** on the sent row | — |
| 10 | Shortcut sheet | PASS | Every key chip legible and inside its chip (checked at 4×), the two-line description wraps, nothing clipped | — |
| 11 | *changed since you looked* | FAIL → fixed | Dot on the hidden Review tab; *1 file changed since you looked* in the header. Before the fix the entry name wrapped when the notice took its room, growing the header and moving the diff; now one line ending in "…" | f8c46db0 (`changes::heading`) |
| 12 | Drag-to-comment composer | FAIL → issue | Composer opens under the dragged lines 10–12, which stay marked; its **Save** is past the viewport, as in 04 | #1519 |
| 13 | Empty review | PASS | *No changes against main*, no tree, *No review comments yet*, **Send review (0)** disabled; the long entry name ends in "…" | — |
| 14 | Empty drawer | PASS | *No drafts* / *No replies yet* with the hint, both tabs at 0 | — |
| 15 | PR diff cannot be read | PASS | *The pull request's diff could not be read; select it again to retry* in the file column; the PR header names #57 | — |
| 16 | Two worktrees | PASS | A keeps its `scheduler.rs` filter and selection, B its `tokens.rs`, across switches both ways | — |
| 17 | Overview board | PASS | Status stripe on a neutral body, the long name wraps on three lines; only the Idle tier is present in this world | — |

Every FAIL has a fix commit or a linked issue. The four fixed FAILs were seen in a first run of the
pass on the unfixed build; each is held by a unit test that failed on the unfixed component
(`tdd/cycle-log.md`, M3). The checks were then re-shot: the crops above are of the fixed build.
