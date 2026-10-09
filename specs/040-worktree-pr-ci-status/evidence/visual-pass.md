# Visual pass, quickstart §B (partial)

Date 2026-10-09. Environment: headless sway 1.11 (pixman), grim, lavapipe (Vulkan software), not a real display.
Debug build of this worktree (micold-ai-ide, micold-daemon, micold-showcase), pointer by `vptr.py`. Private data/runtime dirs; no GitHub access, nothing created or pushed.
Only static appearance was judged. B2/B12 ran against a local repo `git init` with one commit; the wrapper `gh` (logs calls, exits 1) was first on the client's `PATH`.

| Row | Ran | Result | Evidence | Notes |
|---|---|---|---|---|
| B1 light | ran | PASS | b1-light.png, b1-light-grey.png | 12 poses present. Greyscale: states differ by shape (fork-arrows open, lines+pencil draft, merge-arrow merged, slashed circle closed); checks differ by shape (tick, clock, circled x). Stale poses (open passing stale, merged stale) grey and dimmer, same shapes. |
| B1 dark | ran | PASS | b1-dark.png, b1-dark-grey.png | Same as light; stale dimmer, same shapes. |
| B2 | ran | PASS | b2-sidebar-fresh.png, b2-settings-github-off.png | Fresh settings: no indicator on the Default row; Settings -> GitHub switch unchecked with its note. Only a local project with no extra worktrees, so "any row" is weakly exercised. |
| B3 | NOT RUN | needs a real GitHub scratch repo with pull requests | | |
| B4-B8 | NOT RUN | need real pull requests (B5 also a stub xdg-open) | | |
| B9 | NOT RUN | needs sidebar rows with indicators; showcase has no narrow sidebar | | |
| B10, B11 | NOT RUN | need indicators shown first (B3) | | B11 half (failing gh): see B12 note, not recorded as B11 |
| B12 | ran | PASS (with caveat) | b12-settings-github-on.png, b12-after-save.png, b12-after-refresh.png | Switch on, saved (settings.json pr_status_enabled true), then sidebar refresh: no indicator, no error; wrapper gh log never created, no gh in client log. Caveat: not proven the client would find the wrapper (it could locate gh by another route); the repo has no GitHub remote. |
| B13-B16 | NOT RUN | need a real GitHub repo / network, launcher, sandbox placement, second client | | |
| B17 | NOT RUN | no pull requests existed | | |
