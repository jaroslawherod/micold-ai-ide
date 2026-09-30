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
