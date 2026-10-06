# Feature 038, milestone M1: recorded visual pass (quickstart B1, B2)

Ran on Linux under Xvfb + lavapipe (not a real display), 2026-10-02, commit 86ea073c. Client against `cli/cli` (1,038 open issues, 1,000 loaded), light and dark theme, window 1600 wide and the 640 minimum; showcase `Typeahead` at 1600 and 420 wide. Crops, not full frames.

| Step | Result | Screenshot | Notes |
|---|---|---|---|
| B1 light | ok | b1-light.png | `#number title` first line; login, ` · ` and labels below in smaller, dimmer text |
| B1 dark | ok | b1-dark.png | same |
| B1 row without labels | ok | b1-nolabel-dark.png | `#8586 X Privacy Policy`: login `Authentiksolid` alone, no separator (light theme seen live, not saved) |
| B1 searched row | ok | b1-searched-dark.png, b1-searched-light.png | `#288 Add support for .netrc` (among the 38 least recently updated, beyond the 1,000 loaded) has the same two lines; the "Showing the 1,000 most recently updated of 1,038" hint is shown |
| B2 narrow (640) | ok | b2-narrow-dark.png, b2-narrow-light.png | long titles and a 4-label row (`#14545`) wrap inside the row; nothing cut or outside the list |
| B2 wide (1600) | ok | b2-wide-dark.png | the dialog keeps its width, so wrapping is the same as at 640 |
| B2 pick + marker | ok | b2-picked-light.png | pick `#14537` (two-line title): ticket and name filled; reopened list shows the picked marker beside its first line. Type stayed empty because its only label (`needs-triage`) maps to no type; picking `#14216` (label `bug`) filled Type `fix`, ticket and name (seen live) |
| B2 showcase light | ok | b2-showcase-light.png | first row picked, second highlighted, dimmed `fix/logout-redirect`, `main`; at 1600 wide nothing wraps (the long title and the label list fit on one line each) |
| B2 showcase dark | ok | b2-showcase-dark.png, b2-showcase-dark-420.png | same pose; at 420 wide the long title and the many-label row wrap, rows differ in height, nothing cut or overlapping (list opened upward) |

# Feature 038, milestone M2: recorded visual pass (quickstart B3)

Ran on Linux under Xvfb + lavapipe (not a real display), 2026-10-03, commit 93db16de. Client against `cli/cli` (1,038 open issues, 1,000 loaded), dark then light theme, window 1600x1400. Every press was captured (30 crops per theme) and read; the files below are a selection. Crops, not full frames.

| Step | Result | Screenshot | Notes |
|---|---|---|---|
| Down x15 | ok | b3-{dark,light}-down-03.png, -down-08-scrolled.png, -down-15-bottom.png | each press moved the highlight to the next issue (#14563 ... #14550, 15 rows), none skipped or repeated; the highlighted row was wholly visible after every press, including the scrolled frames and the tall (two-line title) `#14550` at the bottom |
| Up x15 | ok | b3-{dark,light}-up-03.png, -up-12.png, -up-15-top.png | back through the same rows; the highlight passes the picked row `#14526` (marker kept); the first row is reached on press 14 and press 15 stays on it (no wrap), wholly visible |
| Enter on a tall row | ok | b3-{dark,light}-pre-pick.png, -picked.png | highlight on `#14467` (two-line title); Enter fills Type `feat`, Ticket `14467`, Name |
| Reopen | ok | b3-{dark,light}-reopened.png | `#14467` carries the picked marker and is highlighted |

Not covered: scroll smoothness. The list was already open at the start of each run; the dark run began with `#14526` picked from an earlier click.

# Feature 038, milestone M3: recorded visual pass (quickstart B4)

Ran on Linux under Xvfb + lavapipe (not a real display), 2026-10-03, commit 615b5092 (client and daemon built from it and pinned in `~/vp/bin038m3`; pair connected). Client against `cli/cli` (1,038 open issues, 1,000 loaded), light then dark theme, window 1600x1400. Crops, not full frames.

| Step | Result | Screenshot | Notes |
|---|---|---|---|
| B4 empty hint | ok | b4-hint-light.png, b4-hint-dark.png | the empty field reads "Search by number, title, label or reporter" |
| B4 login, lower case | ok | b4-login-lower-light.png, b4-login-lower-dark.png | `bagtoad` narrows the list to BagToad's issues (`#14529`, `#9724`, `#14563` ...); the login is bold and tinted in each row's second line, labels unchanged |
| B4 login, capitals | ok | b4-login-caps-light.png, b4-login-caps-dark.png | `BAGTOAD` gives the same list and the same emphasis |
| B4 searched issue | ok | b4-searched-login-light.png, b4-searched-login-dark.png | `JomeFavourite` lists `#6413 Doc Sidebar Navigation Improvement`, one of the 38 least recently updated (beyond the 1,000 loaded; not in the 1,000 most recent): same two lines, `jomefavourite` emphasised |

Observations, not failures: a searched row emphasises the login only when the typed text matches it, as FR-012/013 say. Typing `billygriffin` did not list his beyond-cap `#3065` because the existing server search for that text does not return it (it did when typed as the number `3065`, with two lines and the login unemphasised); `#6413` for `jomefavourite` was returned. Matching is fuzzy, so a short login such as `travi` also picks up unrelated rows whose titles contain its letters.

Not covered: the 150 ms look of the list narrowing.

# Feature 038, milestone M4: recorded visual pass (quickstart B5)

Ran on Linux under Xvfb + lavapipe (not a real display), 2026-10-04, `micold-showcase` built from the working tree at 7e416cdf and pinned in `~/vp/bin038b5`, light then dark theme, window 1600x1400. Crops, not full frames. Instance: Tooltip's sixth, "after 3 s at rest: hold the cursor still".

| Step | Result | Screenshot | Notes |
|---|---|---|---|
| B5 at rest, 1.5 s | ok | b5-rest-1s5-{light,dark}.png | cursor on the icon button (hover tint shown), no panel |
| B5 at rest, ~3.8 s | ok | b5-rest-open-{light,dark}.png | panel below the button, three lines, the last ending `keys…`, legible on the tooltip surface, trigger uncovered |
| B5 cursor off | ok | b5-moved-off-{light,dark}.png | panel closed |
| B5 moving 10 s | ok | b5-moving-mid-{light,dark}.png, b5-moving-end-{light,dark}.png | 10 px-ish steps every 100 ms inside the button, no panel at 5 s or 10 s |
| B5 click after open | ok | b5-clicked-{light,dark}.png | panel closed right after the click; still closed 4 s later with the cursor staying |
| B5 existing: below (default) | ok | b5-below-default-{light,dark}.png | "Settings" opens within 0.6 s |
| B5 existing: multi-line | ok | b5-multiline-{light,dark}.png | four wrapped lines, uncut |

Not covered: idle redraw. The showcase process used about 4-5 cores' worth of CPU (utime+stime, 2,200-2,500 ticks per 5 s) with the cursor away from every instance as well as while waiting, so the baseline is busy on lavapipe for a reason outside the tooltip (page-wide, likely a live pose animating) and the 3 s wait could not be told apart from it.

# Feature 038, milestone M5: recorded visual pass (quickstart B6–B11)

Ran on Linux under Xvfb + lavapipe (private display `:138`, not a real display), 2026-10-04, client and daemon built from commit c9f9d682 and pinned as a pair (`~/vp/bin038m5`; the client log shows `attach: connected`), window 1600x1400 unless stated. Dark theme for every step; light theme as well for B6 and B8. Client against `cli/cli` (1,040 open issues, 1,000 loaded); B9 against `jaroslawherod/micold-ai-ide`. B10's "before" is a build of 7cbb6c76, a commit on `main` without `bodyText` in the query; the M3 pair `~/vp/bin038m3` was not used. Crops, not full frames.

| Step | Result | Screenshot / data | Notes |
|---|---|---|---|
| B6 timing | ok | b6-trials.txt | 21 trials (rows #14386, #14584, #14529; cursor moved in from outside the list; frames polled every ~35 ms): the panel first appeared 3.04-3.13 s after the move, never before 3.0 s, none missed. Timing is by image polling, not a stopwatch |
| B6 content | ok | b6-dark-rest-2s.png, b6-dark-open-row1.png, b6-dark-open-lastrow.png | nothing at 2 s; at 4 s description text only (no title, labels or login), three lines ending `…` |
| B6 light | ok | b6-trials.txt (second block), b6-light-rest-2s8.png, b6-light-open-row1.png | 20 more trials in the light theme (10 on `#14386`, 10 on `#14529`), same polling (a frame every ~33 ms): the first frame with the panel began 3.04–3.09 s and ended 3.08–3.12 s after the move; the frame before it had none. Crop at 2.8 s: row highlighted, no panel. Crop at 3.4 s: a light panel with a thin outline and dark text, three lines of description ending `…`, below its row and over the next row's title |
| B7 sweep | ok | b7-dark-sweep-5s.png, b7-dark-sweep-10s.png | cursor moved every ~80 ms over the list for 10 s: no panel |
| B7 move / leave / click | ok | b7-dark-1-open.png ... b7-dark-7-after-click.png | open; moved to another row: closed at 0.5 s, still closed at 2.5 s, open at 3.8 s on the new row; left the list: closed; reopened and clicked: issue picked (ticket, name, type filled), list and panel closed. The highlight tint stays on the last row after the cursor leaves (not the tooltip's concern) |
| B8 long / short / empty | ok | b8-dark-1-long-open.png, b8-dark-5-short-rest-5s.png, b8-dark-3-empty-body-rest-4s.png | long `#14386`: three lines, `…`. Short `#9135` (`## Description` + a link): `Description See #9136`, whole, no `…`. Empty: no cli/cli open issue has a blank body, so `#9085` (body only an HTML comment) stands in: no panel after 4 s |
| B8 typing while open | ok | b8-dark-2-typed-9085-0s5.png, b8-dark-4-typed-9135-1s.png | typing `9085` with a panel open closed it at once; after typing `9135` no panel at 1 s, panel at 5 s with the cursor still. Spec leaves open whether it reopens without a move |
| B8 positions | ok | b8-dark-6-lastvisible-1400h.png, b8-dark-7-window720-y618.png, -y256.png | window 1400 high: last visible row's panel opens below, over the backdrop. Window 720 high: last visible row's panel flips above the row; first row's panel opens below it. Both inside the window and clear of their own row |
| B8 light | ok | b8-light-1-long-open.png, b8-light-2-typed-9085.png, b8-light-3-empty-rest-5s.png, b8-light-4-short-open.png, b8-light-lastvisible.png | long `#14386`: three lines, `…`. Typing `9085` with the panel open: gone 0.3 s later. `#9085` under a still cursor for 5 s: no panel. `#9135`: `Description See #9136` on one line, no `…`. Last visible row (`#14526`): panel below the list's edge, over the backdrop, inside the window, clear of its row. The 720-high window was not repeated in light |
| B9 | ok, comment text not confirmed there | b9-dark-317-template.png, b9-dark-548-heading-bullets.png | `#317` (bold, two links, checkboxes): the panel reads `Feature 028 (macOS package) shipped in #284 and its spec is closed. One task was split out because it cannot run without a real Mac: T059, the manual pass in…`. `#548` (`## What happens`, backticks, bullets): `What happens In the component showcase, under ContextMenu, pressing Open a context menu opens two context menu panels at the same time, with the same…`. GitHub's `bodyText` for both, read by hand: no `**`, `[ ]`, `##`, backticks, brackets or link addresses; the `#` of the issue reference `#284` stays, and a trailing bare URL in `#317` stays in `bodyText` past the three lines. No open issue in that repository has an HTML comment in its body (searched 200), so comment text was checked only on `cli/cli` `#9085` (body `<!-- Error while uploading code-insiders -->`, `bodyText` empty, no panel). Quickstart's "template" issue does not exist in that repository; these two stand in |
| B10 load time | **FAIL: 1.88x** | b10-times.txt, b10-before-list.png, b10-after-list.png | five alternating runs, click on **GitHub issue** to the first change of the loading line (list confirmed shown, all 1,000 issues, ten requests): before 13.6, 12.4, 11.3, 11.6, 11.1 s (median 11.6); after 23.2, 22.2, 21.8, 21.8, 19.8 s (median 21.8). Ratio 1.88 > 1.5: **stop and escalate (research R14)**. No timeout failure seen in either build |
| B10 query by hand | same ratio, no request over 10 s | | `LIST_QUERY` through `gh api graphql`, `cli/cli`, 100 issues a page. First page, three runs each: with `bodyText` 341,905 bytes in 2.57, 2.45, 2.97 s; without 23,689 bytes in 0.97, 0.99, 1.02 s. All ten pages, twice each: with 2,083,824 bytes in 20.8 and 20.0 s (pages 1.66–2.53 s); without 236,027 bytes in 10.9 and 10.9 s (pages 0.89–1.28 s): ratio 1.87. The slowdown is in GitHub's answer to the query with `bodyText` (about 1 s more per page, 9 times the bytes), not in the client. Slowest single request seen: 2.97 s |
| B11 CPU | ok | b11-cpu.txt, b11-b-after-30s.png, b11-c-narrowed-9085.png | ticks per 30 s, three rounds: (a) rest beside list 746, 717, 731; (b) rest on a row with a description 695, 731, 730; (c) rest on the no-description row 718, 703, 752. (b), (c) within 10% of (a). Baseline is high (~0.24 core, the field's caret under lavapipe) and the same in all three |
| B11 gh | ok | | `pgrep -xa gh` once a second during all 27 windows (B6-B8 trials were not sampled): only other sessions' processes (`gh pr checks`, `gh pr merge`, `gh run view`, `gh workflow run`, `gh api repos/max-speed/...`), never the client's `gh api graphql`. Six more one-second samples with a light-theme panel open: none |

Seen, though no step asks: in the dark theme the panel is a slightly darker rounded surface with no visible outline, low in contrast against the list, so over rows its text can read as printed on them (b6-dark-open-row1.png); the light panel has an outline. The panel starts about 40 px right of the row's text and cuts the next row's title in two (`#1459` left of it, `…nheritance` right of it). A heading's word stays as text (`Description See #9136`, `What happens In the…`).

Not covered: light theme for B7, B9–B11 and the 720-high window; a stopwatch by a human (B6 is frame polling, ~35 ms a frame); the tooltip's fade or appearance animation; an issue with a truly blank body (none open in `cli/cli`); comment text on a template issue in `small`; `pgrep` during the dark B6–B8 trials; B10 against the M3 pair and on a real display. The B10 failure is the finding to act on.

## Second pass (rework, two-pass build)

HEAD 9eb56cf6 (crates identical to the measured 7d77db4f), dark theme, Xvfb :142 with lavapipe. Data and screenshots: `r2-*`; numbers in `r2-observations.txt`, `r2-b10-times.txt`, `r2-b6-trials.txt`, `r2-b11-cpu.txt`. B10 1.03 (PASS), B6 3.045-3.087 s over 23 trials, B7 PASS, B8 and B9 PASS with the stand-ins noted there, B11 PASS (one (c) run at +17 %, unexplained). Not confirmed: light theme for B7-B11, typing while pages land.

# Feature 038, milestone M6: recorded pass on the merged result (T056, T057)

Linux under Xvfb + lavapipe (not a real display), 2026-10-06, `main` at 2b367040 (M5 merged). Data: `m6-b10-times.txt`, `m6-observations.txt`, `m6-*.png`.

## T056, B10 / SC-008: load time against the true baseline

M5's second pass compared against 7cbb6c76, which already contains M1-M4 (the author and labels in the list query), so it was not the feature's baseline. Here "before" is **a357479f**, the commit just before M1's merge (parent of e6444615), exported with `git archive` and built release; "after" is 2b367040 built release (client and daemon pair; `attach: connected` in every run). `cli/cli` (1,000 loaded), dark theme, five alternating runs, pass-through `gh` shim in both.

| Build | Runs (s) | Median |
|---|---|---|
| before (a357479f) | 10.999, 10.213, 11.875, 11.447, 10.384 | 10.999 |
| after (2b367040) | 11.583, 11.453, 10.884, 10.231, 11.378 | 11.378 |

**Ratio 1.03 (limit 1.5): PASS.** Every run made ten list calls (1,000 issues), all rc=0, plus one to two description calls after; the longest single request was 1.62 s before and 2.44 s after (limit 10 s). Screenshots: `m6-b10-before-list.png`, `m6-b10-after-list.png`.

## T057, quickstart A

`mise run gate`: fmt, clippy and every test binary green except `micold-core/tests/github_locate_desktop_launch.rs::a_desktop_launch_finds_a_working_gh`, which fails on this host only (`gh` is installed under mise's directory, outside the desktop-launch PATH the test models); it does not touch this feature. `cargo test --workspace --no-fail-fast` shows no other failure. `typeahead_budget` (release): 10 passed. `mise run test-scripts`: 4 cases, 0 failures. CI runs the gate on a host where that test passes.

## T057, quickstart B1-B9, B11

Measured on the same issue-list crates (`worktree_form.rs`, `github.rs`, `picker.rs` unchanged since 7d77db4f; later changes on `main` are the sidebar, attention and daemon), so not redone; see the sections above:

| Step | Outcome | Where |
|---|---|---|
| B1 | ok, light and dark | M1 section |
| B2 | ok, narrow 640 both themes; **wide 1600 light now captured** (`m6-b2-wide-light.png`); **pick in dark now captured** (`m6-b2-picked-dark.png`, `m6-b2-reopened-dark.png`: ticket 14407 and name filled, picked marker on reopen) | M1 section, M6 |
| B3 | ok, both themes | M2 section |
| B4 | ok, both themes | M3 section |
| B5 | ok, both themes | M4 section |
| B6 | ok, dark 3.045-3.087 s over 23 trials; light 20 trials 3.04-3.09 s | M5 section |
| B7 | ok dark; **light now ok** (`m6-b7-*-light.png`): sweep 10 s no panel; moved to another row closes, still closed at 2.5 s, open at 3.8 s; leaving closes; click picks and closes | M5 section, M6 |
| B8 | ok, both themes (720-high window dark only) | M5 section |
| B9 | ok dark; **light now ok** with the stand-ins `jaroslawherod/micold-ai-ide` #317 and #548 (`m6-b9-*-light.png`): plain text, no markers | M5 section, M6 |
| B11 | ok dark; **light ok** (ticks per 30 s): list visible rest beside 606, 611, 618, on a row with a description 627, 624, 634 (+2-4 %); list narrowed to 9085 beside 446, 439, 441, on #9085 (no description) 437, 443, 442; 93 one-second `pgrep -xa gh` samples during (b): the client's own `gh` in none | M5 section, M6 |

Caveat: the dialog interior is sometimes captured black after about 1 s of hover or typing, with only redrawn rectangles visible; this also happens in the "before" build, so it is not tied to the feature. Crops are clean frames where one could be had.

Not covered: macOS and Windows (FR-030 rests on the absence of any `cfg` arm and on CI's core suite); a stopwatch by a human; a truly blank-body issue; a template issue in `small`; typing while description pages land.
