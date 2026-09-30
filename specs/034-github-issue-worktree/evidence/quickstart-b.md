# Quickstart §B — manual pass, milestone M2 (rows B1–B5, B7–B9)

Date 2026-09-30, commit c3818410, dev build (client + daemon built in one invocation, copied to a private
pin dir; the pair connected: "client attached to daemon"). Ran on private Xvfb :97 + lavapipe, NOT a real
display, on a loaded machine (a java and a mongod process were busy), so input/redraw lag of several
seconds is a harness artefact and not evidence about the app. Private XDG data/runtime dirs.
Projects: /tmp/issue-demo (shallow clone of cli/cli, origin reset to https://github.com/cli/cli.git;
the machine's git config rewrites https://github.com to ssh, so `git remote -v` prints an ssh URL) and
/tmp/no-github (`git init` + empty commit). Type is chosen by hand (M4 not yet built).

| Row | Verdict | Notes / screenshots (evidence/) |
|---|---|---|
| B1 | PASS | Three chips, GitHub issue disabled, caption "This repository has no GitHub remote." `b1-no-github-chip-dark.png`, `b1-no-github-chip-light.png` |
| B2 | PASS | Caption before choosing "GitHub issue reads open issues of cli/cli from GitHub." (`b2-caption-before-choosing-dark.png`). After choosing: notice "Reads open issues of cli/cli from GitHub.", progress bar + "Loading issues from GitHub…" (`b2-loading-dark.png`), then rows `#n title · labels` (`b2-issue-list-dark.png`). "No request before choosing" was not independently observed (no process/log probe). Type/Ticket/Name are covered by the open list while it is showing (dropdown overlay) and visible below once an issue is picked. |
| B3 | PASS | Title narrowing with emphasis (`b3-title-narrow-dark.png`), label narrowing with emphasis on the label (`b3-label-narrow-dark.png`), number narrowing (`b3-number-narrow-dark.png`). Down moves the highlight (`b3-arrow-down-dark.png`); Enter picks and the caret stays in the Issue field. |
| B4 | PASS (with observations) | Ticket = number, Name = title shortened, Type unset (expected) and, after choosing `fix` by hand, preview Directory `.claude/worktrees/fix-14370_b5-demo` / Branch `fix/14370_...` (`b4-picked-dark.png`, `b4-type-fix-preview-dark.png`). See D1, D2. |
| B5 | PASS (with note) | Edited name "b5 demo" created worktree `fix/14370_b5-demo` (git worktree list confirms; `b5-created-dark-small.png`). Second attempt with the same ticket+name showed "A worktree folder named 'fix-14370_b5-demo' already exists. Choose a different name, or remove the existing folder first." with OK only (`b5-second-attempt-dark.png`). That is the folder-collision error, not a reuse/overwrite prompt: the branch is held by the existing worktree so reuse is not offered (worktree_form.rs FR-021 path). Judged as the existing behaviour; flagging in case the spec expects a prompt. |
| B7 | PASS | HTTPS_PROXY=http://127.0.0.1:9 (refused): "Couldn't reach GitHub. Check your connection, then retry." + Retry (`b7-offline-error-retry-dark.png`). Blackhole proxy 10.255.255.1: "GitHub didn't answer within 10 seconds." at ~10.3 s + Retry (`b7-timeout-10s-dark.png`). New branch then created `feat/offline-ok` normally. Substitution: proxy env vars instead of disconnecting the network. |
| B8 | PASS | GH_CONFIG_DIR=empty private dir (substitute for `gh auth logout`, which was NOT run): "Couldn't read issues: you're not signed in to GitHub. Run `gh auth login` in a terminal, then retry." + Retry (`b8-not-signed-in-dark.png`). New branch not re-tested in this instance (proven in B7). |
| B9 | PASS | PATH=/usr/bin:/bin, HOME=empty dir (no gh there; real gh untouched): "Couldn't read issues: the GitHub CLI (`gh`) isn't installed. Install it from cli.github.com, then sign in with `gh auth login`." + Retry, light scheme, legible (`b9-gh-missing-light.png`). |

## Timing (SC-001)
- Chip click to list shown: ~20 s in the app (frames 19.6 s empty, 21.0 s populated) for 1,000 of 1,036 issues, on a loaded machine with a dev build. `gh issue list -L 1000` alone measured 8 s. So SC-001 (< 20 s, form to created worktree) is NOT demonstrated: the list load alone sits at the limit under load.
- Pick to created: create click to worktree directory on disk 0.11 s. The rest is my scripted click latency and is not meaningful (redraw lag of several seconds).

## Defects / observations
- D1 (verify): after Down, Down then retyping a number, Enter picked #14370 (second row) instead of #14371 (first). Possibly the highlight index is not reset when the query changes; could also be redraw lag. Second attempt: no row highlighted after typing until Down, and Enter with nothing highlighted did nothing.
- D2: after picking, the Issue field keeps the typed query ("14371") rather than showing the chosen issue, so it reads as a different issue than the Ticket (14370). Confusing.
- D3: while the list is open it covers Type/Ticket/Name; the dialog also moves vertically as its content changes (loading -> list -> picked).
- D4: the selected source chip has no outline and the unselected ones do, so the selected state is only a faint difference (`t2` state, all screenshots). Check against the design if intended.
- D5: Type dropdown list is clipped at "ci" with a scrollbar (8 of the types visible); minor.
- Not run: "no request before choosing" observation (B2), B6, B10–B12, full B13 (light scheme covered only for B1, B9 and the pre-choice caption `b13-caption-light.png`; error + Retry covered in both schemes: B7/B8 dark, B9 light). No clipping or overlap seen at the 520 px dialog width in any capture.

## §B6 — search beyond the 1,000 loaded issues (milestone M3)

Date 2026-09-30, commit c1c43e14, dev build (client + daemon built in one invocation, copied to ~/vp/b6bin; "client attached to daemon" in the daemon log). Private Xvfb :96 + lavapipe, NOT a real display, on a heavily loaded machine (load average ~20), so redraws lag by seconds and screenshots often show a stale frame. Private XDG dirs. Project /tmp/issue-demo (cli/cli, 1,036 open issues). Beyond-cap issue from `gh issue list --search "sort:updated-asc" -L 1`: #925. Also tried #1469 and #2325.

| Criterion | Verdict | Notes / screenshots (evidence/) |
|---|---|---|
| 1 Caption | PASS | With the list closed: "Showing the 1,000 most recently updated of 1,036 open issues — search also looks on GitHub." wrapped onto two lines under the Issue field. `b6-caption-dark.png` |
| 2a "Searching GitHub…" | NOT OBSERVED | Not caught in any frame: typing and redraw lag by seconds under the load, and while the list is open it covers the area under the field, so the line would be hidden by the overlay in any case (see D6). Not a PASS. |
| 2b Issue appears once | PASS | Typing 925 lists #925 "Editor is not passed to the shell" once (`b6-search-925-dark.png`); typing 1469 lists #1469 once at the top; the list was scrolled to its end (about 14 rows) and no repeat (`b6-search-1469-dark.png`). 2325 gives #2325 first of three rows (`b6-search-2325-dark.png`). |
| 3 Pick fills fields | PASS | Click on the #1469 row: Ticket = 1469, Name = "Help output just SLIGHTLY wider than 80 characters" (`b6-picked-dark.png`). Enter not tried. |
| 4 No clip/overlap at 520 px | PASS | Nothing clipped or overlapping in any capture; long rows are ellipsised. |

### B6 observations
- D6: the results overlay sits directly under the field and covers the caption/searching/failed line while it is open, so the "Searching GitHub…" and failure lines can only be seen when the list is closed or empty. Check whether the line is meant to show with the list open.
- Not run: failed-search line (network fault not injected for this row); light scheme; Enter to pick.

## §B4 — labels choose the type (milestone M4)

Date 2026-09-30, commit d721635b, dev build (client + daemon built in one invocation, copied to ~/vp/b4m4bin; the pair connected). Private Xvfb :95 + lavapipe, NOT a real display, on a loaded machine (input and redraw lag of several seconds, so waits were long). Private XDG dirs. Project /tmp/issue-demo (shallow clone of cli/cli, 1,036 open issues). Default label mapping (bug→fix, enhancement→feat, documentation→docs), no settings.json edit. No worktree created.

| # | Criterion | Verdict | Notes / screenshots (evidence/) |
|---|---|---|---|
| 1 | Rows show their labels (AS7) | PASS | Rows read `#n title · label, label`, e.g. `#14528 ... · enhancement, gh-issue`; long label lists are ellipsised. `b4m4-list-labels-dark.png` |
| 2 | Pick a `bug` issue: Type `fix` with no hand choice | PASS | Typed "bug", picked #13022 "Bug with the --exclude option for pr diff" (labels bug, priority-3, ...). Type = fix, Ticket = 13022, Name = title, Directory `.claude/worktrees/fix-13022_bug-with-the-exclude-option-for-pr-diff`, Branch `fix/13022_bug-with-the-exclude-option-for-pr-diff`. `b4m4-bug-narrow-dark.png`, `b4m4-bug-picked-fix-dark.png` |
| 3 | `chore` by hand, then pick an unmapped issue: Type empty | PASS (with note) | First run: #9414 (only `needs-triage`) after fix from the bug pick: Type empty, Create disabled (`b4m4-unmapped-type-empty-dark.png`). Then chose `chore` by hand (form showed `chore/9414_...`), re-picked #9598 (`more-info-needed, needs-triage, gh-auth`): Type empty, Ticket 9598, Name "Host validation", Create disabled (`b4m4-chore-then-unmapped-empty-dark.png`). Note: the form "asks for a type" only by the empty Type field and disabled Create; no explicit message or preview is shown. |
| 4 | Pick an `enhancement` issue: Type `feat` | PASS | #9533 (`enhancement, needs-triage`): Type = feat, Directory `.claude/worktrees/feat-9533_pr-fetch-command`, Branch `feat/9533_pr-fetch-command`. `b4m4-enhancement-feat-dark.png` |

### B4/M4 observations
- No defects found for the M4 behaviour. Dark scheme only; light scheme not run.
- Once the Type is empty the preview (Directory/Branch) disappears and Create is disabled; nothing names the missing field beyond the empty Type box.
- After picking, the Issue field still shows the typed query (D2 above), unchanged by M4.

## §B12 — the mapping in Settings (milestone M5)

Date 2026-09-30, commit d1a67cf4, dev build (client + daemon built in one invocation, copied to ~/vp/b12bin; the pair connected). Private Xvfb :98 + lavapipe, NOT a real display, on a loaded machine (input and redraw lag of several seconds). Private XDG data/runtime dirs (the user's settings.json untouched). Project /tmp/issue-demo (shallow clone of cli/cli, 1,037 open issues). Substitution: cli/cli has no issue labelled `question`, so the mapping also got `tech-debt → chore`, and #13016 (labels `enhancement`, `tech-debt`) stood in for the `question` issue.

| # | Criterion | Verdict | Notes / screenshots (evidence/) |
|---|---|---|---|
| 1 | Settings → GitHub issues is the 5th rail entry (`label` icon); default three entries shown | PASS | bug→fix, enhancement→feat, documentation→docs; up disabled on the first row, down disabled on the last. `b12-defaults-dark.png` |
| 2 | Add entry `question` / `chore`, move to top with the up arrows; up disabled on first, down on last | PASS | Row moved one step per click, keeping its type; final order question, bug, enhancement, documentation. "Move up" and "Delete entry" tooltips seen. `b12-reordered-dark.png` |
| 3 | Save; create-worktree form, GitHub issue source, pick an issue with a mapped label: Type follows the mapping | PASS (with substitution) | No `question` issue exists in cli/cli. Added `tech-debt → chore` above `enhancement` (question, tech-debt, bug, enhancement, documentation); picked #13016 (`enhancement, tech-debt`): Type `chore`, Ticket 13016, Directory `.claude/worktrees/chore-13016_enable-medium-low-effort-linters`, i.e. the higher entry wins over `enhancement→feat`. settings.json held `issue_label_types` in the saved order. `b12-picked-chore-dark.png` |
| 4 | Restart the client; reopen Settings → GitHub issues: order persisted | PASS | Client and daemon restarted; `question` still first, then `tech-debt`, bug, enhancement, documentation. `b12-persisted-dark.png` |
| 5 | Blank label: Save refused, Settings jumps to GitHub issues, row's field shows an error | PASS | Row error "Enter a label, or remove this entry." and red label. Also tried from Appearance: Save jumped to GitHub issues with the same error. `b12-blank-refused-dark.png` |
| 6 | Label changed to `Bug`: refused with “bug” is already mapped above. | PASS | Error text `“bug” is already mapped above.` on that row (case-insensitive match against `bug`). The blank-label error cleared as soon as the field was edited. `b12-duplicate-refused-dark.png` |
| 7 | Restore defaults → the three defaults | PASS | Six draft rows became bug, enhancement, documentation. `b12-restored-defaults-dark.png` |
| 8 | Cancel discards the draft | PASS | After Cancel, settings.json still held the 5 saved entries. |
| 9 | 520 px wide, dark and light: nothing clipped, tooltips on move/delete | FAIL (with rail expanded), PASS (rail collapsed) | Rail expanded at 520 px: label/type fields collapse to ~10 px slivers, their floating labels ("Label", "Type") overlap and the up arrow overlaps "Type" (D1). Rail collapsed: all five rows fit, nothing clipped, in dark and light; tooltips "Move up", "Move down", "Delete entry" show. `b12-narrow-520-dark.png` (expanded, broken), `b12-narrow-520-collapsed-dark.png`, `b12-narrow-520-tooltip-dark.png`, `b12-narrow-520-light.png` |

### B12 observations
- D1 (defect): at a 520 px window with the Settings rail expanded, the GitHub issues rows are unusable: the content column is about 180 px, the two fields shrink to slivers and the row controls overlap them. Other Settings pages wrap their text and stay legible at this width; this one needs the row to wrap (fields on one line, buttons below) or a minimum field width.
- The first and last rows' arrow buttons sit a few px off the middle rows' (a disabled arrow is placed differently), so the Type field of rows 1 and 5 is slightly wider than in the middle rows (visible in every screenshot). Cosmetic.
- Save from another Settings page jumps to GitHub issues on a bad row, as required.
- Not run: light scheme at full width; error rows in light; Save with mapping errors at 520 px; no worktree was created.
- Harness note: clicking the sidebar "+" starts a Claude session in the project root (not the create-worktree form); the form is the header icon left of it. That session was stopped with the daemon.

### §B12 re-check after D1 (commit 0baf811a)

Date 2026-09-30, dev build of commit 0baf811a (client + daemon built in one invocation, pinned to ~/vp/d1bin, the pair connected). Private Xvfb :61 + lavapipe, NOT a real display. Private XDG data/runtime dirs, project /tmp/issue-demo. Settings rail expanded, default three entries plus one added entry; Save pressed with the added entry's label blank.

| # | Criterion | Verdict | Notes / screenshots (evidence/) |
|---|---|---|---|
| 9 | 520 px wide, rail expanded, dark and light: every Label field, Type select and the three buttons fully visible; rows do not overlap; error line readable | PASS | Each row is Label over Type, with move-up / move-down / delete stacked to the right. Floating labels "Label" and "Type" sit clear of the values; nothing overlaps or is clipped at the field edges. Error row: red "Label" plus "Enter a label, or remove this entry." on two lines, readable in both schemes. `b12-d1-520-expanded-error-dark.png`, `b12-d1-520-expanded-error-light.png` |
| — | Window about 1000 px wide still sensible | PASS (dark only) | Label and Type span the full content width, stacked, buttons at the right; error on one line; Add entry / Restore defaults below. `b12-d1-1000-error-dark.png` |

D1 is fixed. Observations:
- At 520 px the Label field is about 100 px wide, so a long label is cut inside the field ("documentation" shows as "documentatio"). Normal text-input clipping, not a layout defect; the full text is reachable by editing.
- At 520 px the Restore defaults button wraps to two lines and its second line sits under the sticky Cancel/Save footer at the bottom of the unscrolled list (scrolling reveals it). Minor.
- The disabled arrow (up on the first row, down on the last) sits a few px left of the enabled ones (cosmetic, as noted in the earlier observations).
- At 1000 px the stacked fields are very wide (about 600 px each); sensible but roomy. Not a defect.
- Not run: light scheme at 1000 px; Save with errors on several rows; the tooltips (unchanged by the fix).

## §B10 (Linux arm) and §B11 — desktop launch and sandbox placement (milestone M6a)

Date 2026-09-30, commit 4988665a (tree = origin/main + M6a docs/test; no app code changed), dev build
(client + daemon built in one locked invocation and copied to ~/vp/m6a/bin, since another worktree was
building into target-shared concurrently). Sandbox image `micold-daemon:m6a` (a private tag, so no other
worktree's `:dev` could swap it) built from the same tree by `MICOLD_IMAGE_TAG=micold-daemon:m6a mise run image`.
Private Xvfb :91 + lavapipe, NOT a real display. Private XDG data/runtime dirs per run. Project
/tmp/issue-demo (shallow clone of cli/cli, origin https://github.com/cli/cli.git). `gh` exists on this host
only at `~/bin/gh`, signed in. Run 1 is the control (terminal launch) the other two are compared with.

| Run | Row | Verdict | Notes / screenshots (evidence/) |
|---|---|---|---|
| 1 | control: terminal launch, B2 | PASS | Caption before choosing "GitHub issue reads open issues of cli/cli from GitHub."; after choosing "Loading issues from GitHub…" then the list within ~30 s. Top rows #14015 "Support Discussion Templates", #14554 "Literal "ESC" text corrupts colorized table and --json output", #14223 "gh pr merge --squash --delete-branch: …". Closed-list caption "Showing the 1,000 most recently updated of 1,036 open issues — search also looks on GitHub." `b10-terminal-list-dark.png` |
| 2 | §B10 Linux: `.desktop` launch, B2 | PASS | A copy of `packaging/micold-ai-ide.desktop` (Exec = the pinned binary) in the run's `$XDG_DATA_HOME/applications`, launched by `gtk-launch micold-m6a` under `env -i` with `PATH=/usr/local/bin:/usr/bin:/bin` (no `~/bin`); `/proc/<client>/environ` confirmed that `PATH`. No "gh isn't installed" error; same top rows and the same "1,000 … of 1,036" caption as run 1. Default settings (environment include on, the user's `~/.bashrc` does not put `~/bin` on `PATH`), so `gh` came from the well-known-directory step. The app rendered light (no theme setting under `env -i`, follows the system). `b10-desktop-list-light.png` |
| 3 | §B11: sandbox placement, B2 | PASS | `settings.json` seeded with `daemon.placement = local_sandbox`, image `local_build` / `micold-daemon:m6a`, network `no_outbound`, no credential shares. `docker ps` showed container `38470b3ca2fe` (`micold-sandbox`, image `micold-daemon:m6a`; no `gh` inside it). Daemon log: `client attached to daemon client_build=micold-ai-ide/0.15.0`; no VersionMismatch. The chip was enabled with the cli/cli caption (`RemoteList` answered by the sandboxed daemon) and the list matched run 1 (same top rows, "1,000 … of 1,036") although the sandbox has no outbound network and no GitHub credential: the fetch runs in the client on the host. `b11-sandbox-caption-dark.png`, `b11-sandbox-list-dark.png` |

### B10/B11 observations
- §B10's macOS and Windows arms are not in this file: they are `crates/micold-core/tests/github_locate_desktop_launch.rs` on CI's macOS and Windows runners (T095, quickstart §B10 note).
- Harness: stopping run 3's container hung once (`docker rm -f` reported "removal already in progress"); it was gone when checked later. Not app behaviour.
- A first stray click on "+" opened a Claude session tab in run 1; harmless, closed with the client.
