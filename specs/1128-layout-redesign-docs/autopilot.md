# Autopilot ledger — #1128 layout-redesign-docs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1128 "Layout: docs, layout snapshot and visual pass for the redesign". Blocked by: #1124, #1125, #1126, #1127. Update user guide pages (worktrees-and-sessions, terminal-panes, overview, project-selection), regenerate the layout snapshot fixture, and run the visual pass at 1280x800 in both themes. Mockup: https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e; labels: enhancement, flow:feature (in-progress added by orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1128
- **Worktree branch**: claude/project-thread-bznqpy
- **Started**: 2026-10-24
- **Phase**: M3 implemented, reviews A and B clean, rebased onto origin/main 58ef1146 (after #1429), fixture re-recorded (additions only), full gate green at 65ececb4; pushed, PR to open
- **Next step**: orchestrator opens the M3 PR from the unit's pr-title.md / pr-body.md (gh not authenticated in the unit), label `docs-not-needed`, records its number, waits on CI and merges. M4 (visual pass) after M3 merges.

Blockers at spec time: #1124 and #1126 merged; #1125 open (M1, M2 on main; M3–M5 todo); #1127 open, no spec. Spec stages the work: stage 1 (status bar, session area) now; stage 2 (sidebar, title bar, whole-window states, fixture, visual pass) after #1125 and #1127 merge.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1365 | Design (spec, plan, tasks) | merged (rebase) | f955fbea3ac738910219cb0daf5c61bdf6a03415 |
| #1372 | M1 — status bar and session area in the user guide | merged (rebase) | 1c69ea8bb8e8b6b755b21c167d23307e43b6a46c |
| #1429 | M2 — sidebar and title bar in the user guide | merged (rebase) | 58ef1146da2e1cd31d7def7217c08b7132538dd3 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Status bar and session-area mentions corrected and linked; no bar under the terminal, no unnamed "bar" | #1372 | merged (rebase) as 1c69ea8b |
| M2 | T012–T020 | full | Sidebar and title bar in the user guide; FR-009 opening section; media alt text | #1429 | merged (rebase) as 58ef1146 |
| M3 | T021–T029 | full | `window-session-view`, `window-overview`, gate W, regenerated fixture, `layout-snapshot.md` passage | — | pushed on claude/project-thread-bznqpy; reviews A and B clean; full gate green at 65ececb4; PR to open |
| M4 | T030–T034 | full | Visual pass: 4 frames, 20 verdicts, contrast | — | blocked: after M3 |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Visual-pass defect in a shipped region: fix here or record? | Record as *differs by defect* + ledger follow-up; no code change | agent | spec#Context; 1124 ledger follow-ups; 1126 visual-pass.md |
| D2 | clarify | Sweep removed-element mentions in user-guide pages outside the four? | Yes, only those sentences/rows, staged by region | agent | constitution VII; spec#Context (closing pass) |
| D3 | clarify | Add two whole-window covered states or only regenerate? | Add both, then regenerate | agent | layout-snapshot.md#Adding a covered state, #What is covered |
| D4 | M3 | What does W1/SC-004's "44 px" read now that #1127 shipped `app_bar::HEIGHT` 43 + `DIVIDER` 1? | W1 `title_bar.h == app_bar::HEIGHT` (43, the bar); W3 `title_bar.y + BOTTOM_EDGE (44) == shell.body.y`; the 44 px title bar of SC-004 is `BOTTOM_EDGE`, #1127's "one 44 px title bar, divider included" | agent | anatomy.rs `app_bar` docs; fixture `title_bar -> 0/0/0/0/0` 43, container 44; research R6 "W3 follows what ships" |
| D5 | M3 | Which node is `sidebar`? | `0/0/0/1/0`, the sidebar column with its 6 px resize handle (266 wide): its right edge is where the session area starts (W4); the 260 px surface inside it stops short of it | agent | fixture `window-session-view` |
| D6 | M3 | Overview row selection accessor (#1125 M3)? | None ships: `ui::sidebar::overview_row` draws the row selected exactly while `state.overview.open`, and `features::sidebar::row_selected` deselects every other row then. W5 asserts open, not full screen, a project active, and no session row selected through `row_selected` | agent | sidebar.rs:206, features/sidebar.rs:943 |
| D7 | M3 | `strip_at` carries which fields of the replaced `feat-working` session? | lifecycle, activity and paused, not ActivitySignal alone: the Overview's card state reads `SessionLifecycle::Running` with `Working` | agent | micold-core overview.rs:992 |
| D8 | M3 | Fixture failure named only the first differing state (main-shell-sidebar-expanded), so a 1 px sidebar edit did not name the whole-window states (SC-005) | `describe_difference` now also lists every covered state that differs; test `a_difference_past_the_detailed_ones_still_names_its_state` (red recorded, then green) | agent | mutation sidebar-1 log |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| spec | 1 | `8604b804860bddadd362930f075cce48e8c64032:a08a4b365389835342d3751fd4b69a9a9c05c8b6` | CHANGES: F1 MAJOR fixed (plan usage dropped from FR-010 and the #1127 Context row; edge case added for #1127 shipping it in the title bar); F2 MINOR fixed, prose only (FR-019 now covers every link, as SC-003); F3 MINOR fixed, prose only (in-region banners such as the stored-sessions offer are not removed elements, in Terms and FR-001). Round 2 needed: F1 changes a requirement. |
| spec | 2 | `482d3e84…:e0a83183…` (Sonnet re-review of the diff from round 1) | CLEAN; one MINOR fixed, prose only (Terms "removed element" paragraph rewrapped). |
| clarify | 1 | — | 3 questions, all agent-resolved (D1–D3); FR-011, FR-012, FR-020, SC-001, US4 scenario 4 changed. |
| clarify | 2 | — | No critical ambiguity; unit ends. |
| plan | 1 | `042e8e0058432d5b45e9377611122c12f1cce667:2fa8a2688c30d42e47a2ec0733539e77cba161bc` | CHANGES: F1 MAJOR fixed (`title_bar` anchors the 64 px bar, not its 65 px container; W1 `h == app_bar::HEIGHT`, W3 `title_bar.y + app_bar::BOTTOM_EDGE == shell.body.y`; hairline is the bar's bottom edge for SC-004's gap; follows #1127 if it drops the divider; R6, contract, data model, plan updated); F2 MINOR fixed (W5 in R6 and data model, asserted on `State` not records, selection accessor open until #1125 M3; mutation row: Overview closed → W5); F3 MINOR fixed, prose only (extended `strip_with` replaces `feat-working`'s session, keeping Working, in R5 and plan). Round 2 needed: F1 changes gate assertions. |
| plan | 2 | `2d670a35839ea1afff7c10d0094098102cdaa020:7301669f92b15e41ef74c6fbb0abe48878927cef` (Sonnet re-review of the diff from `042e8e00…:2fa8a268…`) | CLEAN; one MINOR declined (see Declined review findings). |
| tasks | 1 | `d43d91dcbdf609cfb6bd9034bc18cb89f689db50:fdcd26120f013b2724b34a84e104a307cb892afb` | CHANGES: F1 MAJOR fixed (M2 tier docs → full, since `site/media.toml` is not `micold-docs`; *Incremental delivery* says only all-`micold-docs` PRs skip CI and that M2 runs the pipeline; `media-references.test.sh` added to T020 and M2 Verify); F2 MINOR fixed (T007 loses `[P]`, sequenced with T003–T006); F3 MINOR fixed (T026 times the two states warm, with and without, for T028). Round 2 needed: F1 changes a tier and a verify step. |
| tasks | 2 | `3e3714988e7394b14e7a1dcddb5b0f65e0b66e72:708ca247350fd73c1e8dd3852fd6197b4eb7f3be` (Sonnet re-review of the diff from `d43d91dc…:fdcd2612…`) | CLEAN; no findings. |
| analyze | 1 | — | No CRITICAL/HIGH/MEDIUM: 27/27 FR and SC have tasks; no placeholders; T035 (Phase 7) belongs to the close unit, outside M1–M4 by design; checklist has no unchecked items. No change. |
| M1 review A (code-review, high) | 1 | — (working tree on f955fbea) | 10 findings: fixed — `### The tab strip` vs the shortcut-from-AI-tab bullet (stale: the chord opens a shell from the AI tab since 027 FR-004), settings.md restart placement, give-up sentence placing restart in the pane, chord focus condition, E6 rewrite dropping the session group, terminal-panes.md:94 macOS forms, status-bar sentence missing key hints; recorded as follow-up — in-app "Choose restart below"; declined — reflow (MINOR), dup of the first paragraph (MINOR: SC-007 needs the answer there). |
| M1 review A (code-review, high) | 2 | — (fix diff) | 8 findings, all wording precision in round-1 fixes: Cmd+C/Cmd+V (no Shift) on macOS, tab order when a helper is shown, restart only while stopped, "Both restart controls" ambiguity, "+" shown with no instance, layout bullet in appearance-theming.md, '+' bullet focus condition; all fixed; reflow declined again. |
| M1 review A (code-review, high) | 3 | — (fix diff) | 9 findings, wording precision in round-2 fixes (tab placed after itself, other sessions' order main-then-helpers, focus qualifier scope, '+' anchor with no instances, "new terminal"/"unchanged" release-note wording at terminal-panes.md:94, conditional status-bar items, "a single tab" on a new session, how to get a stopped process for the restart); all fixed, prose only. Rated MINOR (no wrong placement of an element remains); not escalated; review B covers the final text. |
| M1 review B (conformance) | 1 | `3740dabadc2d0749a908e15d2ceee93b291d0c05:67674781225eb24388b605834d6641c1e8e6794b` (Sonnet) | CLEAN; 3 MINORs: F1 fixed, prose only (appearance-theming.md notifications "a small bar" → "a small notification"); F2 declined (see Declined review findings); F3 fixed, prose only (settings.md recovery parenthesis dropped: the client has no general stop-session control, only **Stop session** while an interrupt is overdue). |
| M2 review A (code-review, high) | 1 | — (`origin/main...d1e125b3`) | 9 findings: fixed — `Choosing your theme` described the removed overflow theme toggle (now Settings → Appearance), icons.md theme rows, `docs/README.md` "Help toolbar entry", help-about step 1 named one menu item, Overview first paragraph repeated the chord, Title bar intro repeated its bullets; declined — line reflow (as M1), quick-switcher link target. |
| M2 review A (code-review, high) | 2 | — (fix diff `d1e125b3..bbbfc373`) | 9 findings: fixed — picker label is "Follow the system", the `theme-follow` clip showed and pressed the removed toggle (clip, `site/media.toml` entry and `site/capture/scenes/theme-follow.sh` removed), icons.md alignment note contradicted itself and named a wrong path, unrendered Dark/Auto rows dropped, history wording dropped, overview reflow, help-about links Settings and diagnostics. |
| M2 review A (code-review, high) | 3 | — (fix diff `bbbfc373..55f70280`) | 7 findings, none from this diff at BLOCKER/MAJOR: fixed — icons.md contributor paths, alignment note dated separately and records the orphaned Dark/Auto icons; declined or follow-up — see below. Clean. |
| M3 review A (code-review, high) | 0 | — (`origin/main...101e5a00`) | Ran on the whole branch by mistake: its 9 findings are all M2's diff (#1429), none in M3's files; re-run on `3fa50c20...101e5a00`. Not a counted round. |
| M3 review A (code-review, high) | 1 | — (`3fa50c20...101e5a00`) | 10 findings: fixed — W5's `row_selected` loop was vacuous (short-circuits on `overview_open`) and copied the private `row_selection`: dropped, W5 asserts the fields that are the selection (D6); W4 checked heights only: now tops, sidebar left, session area right edge too, W1/W2 left and width; new describe test read the fixture file and raced a regeneration: uses `emit_fixture`; `Window::region` copied the `at()` lookup of three strip gates: extracted `support::layout::anchored`, all four use it; contract W1 said HEIGHT 44: corrected to 43 + DIVIDER (prose). Declined — see below. Round 2 on the fix diff. |
| M3 review A (code-review, high) | 2 | — (fix diff `101e5a00..381f54c8`) | Fixes hold. 7 MINORs: fixed — declined rows moved into the Declined table; data-model.md and research R6 still read HEIGHT 44 (prose); W3 now holds the body's left edge and width; `region` panic names the declared path again; contract W2/W3 rows list the width checks, line re-wrapped. Declined — see below. Clean. |
| M3 review B (conformance) | 1 | `55012b98799571e4ba1c20e7eecb3c04647547bf:fda91d7211b016e78b22856c5c05d8bf9c98b8f3` (Sonnet) | CLEAN, 3 MINORs: F1 A1 link check for `layout-snapshot.md` not observed (lychee missing) — left to CI's link check (the passage adds no link); F2 W5 cannot assert the Overview row's selection (no accessor, D6) — accepted, see Declined; F3 SC-004 said "title bar is 44 px" while W1 checks 43 (HEIGHT) and W3 44 (BOTTOM_EDGE) — fixed, prose only (note on SC-004 pointing at D4). |
| M2 review B (conformance) | 1 | `86ed9f3eebb0fd4ed330dee693d4cddfa1392404:02f788dccc6541db122fee7aa38c8f9b2ce52d3a` | CLEAN, 2 MINORs: F1 icons.md Filter row named the removed tag-filter panel — fixed, prose only (glyph shown only in the component showcase); F2 icons.md "adding an icon" steps re-pathed beyond FR-011 — paths correct, recorded as an extra in *Follow-ups not done*. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A rounds 1–3 | MINOR: reflow the paragraphs whose added links pushed lines past 100 columns | The pages are not wrapped consistently (hundreds of lines over 100 columns in each touched file); reflowing would widen the diff past FR-011's "that sentence only". |
| M1 | B round 1 | F2 MINOR: worktrees-and-sessions.md "+" bullet restates the tab strip's "+" | Optional (FR-004 judgement call): the bullet is where that page explains opening instances, and it links to the tab-strip section rather than redescribing its layout. |
| M2 | A round 1 | MINOR: reflow added lines past 100 columns | As M1: the pages are not wrapped consistently; reflowing whole paragraphs widens the diff past FR-011. |
| M2 | A round 1 | MINOR: quick-switcher link lands on `#interacting-with-the-terminal`, not the bullet | That section holds the quick switcher's owning bullet (R2); a list item has no anchor, and `terminal-panes.md#keyboard-shortcuts` only lists the chord. |
| M2 | A round 3 | `scene_menu_settings_y=145` measured with the theme item first | Pre-existing: the theme item was gone before this branch, and every scene coordinate predates the 44 px title bar and the tab strip; recorded as a follow-up, not caused by this diff. |
| M2 | A round 3 | MINOR: `scene_start --preference` now unused; feature 028/006 tasks still name the theme-follow scene; Choosing your theme has no picture | Dead-option cleanup and a new picture are outside M2 (follow-ups); other features' tasks.md are historical records. |
| — (design) | plan round 2 | MINOR: `contracts/covered-states.md:40` says each anchor is "held by W1–W4" under a W1–W5 table | W5 is a state-level assertion on the `State`, not on an anchor, so W1–W4 are the anchors' properties; the sentence is correct. |
| M3 | A round 1 | MINOR: `describe_difference` skips `## `/`@ ` lines, so an anchor-only change is not named | Pre-existing (not this diff); an anchor-only change still fails the byte compare, and the message says the files differ. |
| M3 | A round 1 | MINOR: the differing-states list names every later state after a line shift | The comparison is positional, as its detailed lines already are; the list still names the states that truly moved first, and a shift is itself the change. |
| M3 | A round 1 | MINOR: gate W's dark pass is redundant with `the_other_scheme_lays_out_identically` | The contract asks gate W in both schemes (US3 s4); the dark records are already cached for that test in a full run. |
| M3 | A round 1 | MINOR: `strip_at` keys sessions by `PROJECT`, replaces only the first session at `dir`, helpers keep the restored lifecycle | Test fixture over a fixed base whose sessions are filed under `PROJECT`; `feat-working`'s card state comes from the main session's carried lifecycle and activity (D7); the fixture records the grouping. |
| M3 | A round 2 | MINOR: W5 reads the State fields, not the code that decides the selection | FR-020 forbids a `src` change, and `ui::sidebar::row_selection` is private; D6 records that `overview.open` with a project active is the selection as shipped. |
| M3 | A round 2 | MINOR: `rail_icons_align.rs` and a `layout_snapshot.rs` closure still look anchors up by hand | Not copies of the block this milestone touched (the closure reads any layer); left for the duplication budget. |
| M3 | B round 1 | F2 MINOR: W5 cannot assert the Overview row's selection | No selection accessor and FR-020 forbids a `src` change; D6 records `overview.open` with a project active as the selection as shipped. |

## M3 implementation evidence

- T021–T023: `strip_at(base, dir, shells, helpers)` (strip_with calls it with `with_project()` and `feat-short`; its states byte-identical), `window_session_view()`, `window_overview()`, `WINDOW_ANCHORS` (paths from the fixture: title_bar 0/0/0/0/0, shell.body 0/0/0/1, sidebar 0/0/0/1/0, session.area 0/0/0/1/1, status_bar 0/0/0/2).
- Red (no `tdd/test-list.md`: `tdd.plan` was skipped at the tasks stage; the red is the fixture and the mutation proof, research R6): registering the states before regenerating failed `the_layout_matches_the_committed_fixture`.
- T024: regeneration only adds lines (0 removed; +804 at 33558, after `status-bar-disconnected`); a second regeneration is byte-identical; the run without the variable passes.
- T026: `layout_snapshot` 146 pass, `layout_text_overflow` 3 pass. Warm cost: 9.49/9.50/8.98 s with the two states, 9.14/9.19/9.02 s without (mean 9.32 vs 9.12, ~0.2 s).
- T027 mutation proof (each reverted): title bar +1 px (`Toolbar` bar height) → fixture names both window states, W1 (44 vs 43), W3 (45 vs 44); status bar +1 px (`Lane::new(HEIGHT + 1.0)`) → fixture, W2 (29 vs 28); sidebar 1 px narrower → fixture names both window states, W1–W5 hold; 1 px spacer under the title bar → fixture, W2 (status_bar 727) and W4 (anchor `sidebar` names no record) — W3 holds because the spacer shifts the column's children so `shell.body` resolves to the spacer; Overview closed in `window-overview` → fixture names `window-overview`, W5.
- T029: `git diff --stat origin/main -- 'crates/*/src'` empty (FR-020). A1: `site/checks/links.sh --sources` exits 2 (lychee not installed); the M3 passage adds no link.

## Handover

M3 unit 2: recorded review B (CLEAN, 3 MINORs; F3 fixed prose only), reset the branch to `m3-1128` and rebased onto origin/main 58ef1146 (M2's commits dropped as applied, no conflict), re-recorded the fixture on main: no change, diff vs main adds only `window-session-view` and `window-overview` (+804, 0 removed). Full gate (`scripts/gate.sh`) red once for the environment only: the signing server returned 503 inside `micold-daemon` mcp test fixtures' `git commit`; re-run with `commit.gpgsign=false` via GIT_CONFIG_*, green at 65ececb4. Pushed; second worktree and `m3-1128` removed. Next: open the M3 PR. No open findings.

## Open escalation

None.

## Follow-ups not done

- M2 (T012): #1127 kept plan usage in the status bar (its FR-002 and *Out of scope*), so FR-001 and FR-010 stand unchanged. Removed list as shipped: the title bar's Overview button and badge, the app-name title text, the switcher button next to the menu button (now the breadcrumb's project segment), the sidebar's filter button and tag-filter panel (now the chip bar and its ⋯ menu), type/issue chips on rows (now two-line rows). Owning sections: title bar → new `appearance-theming.md#title-bar` (#1127 wrote a bullet, not a section); breadcrumb → `project-selection.md#switching-projects-from-the-breadcrumb` (renamed; no inbound links to the old anchor); Overview row → `overview.md#opening-and-closing-it`; chip bar → `worktrees-and-sessions.md#filtering-worktrees-by-tag`; groups → `#browsing-worktrees`; two-line rows → `#reading-a-worktree-name--tags`. Spec Edge case *Project with no worktrees* and doc-edits S15 corrected to what #1125 shipped (Default under its state's group, chip bar with All, "No worktrees yet" hint).
- M2 extra hits beyond S1–S15: `help-about.md` toolbar wording and its About steps, `docs/README.md` Help entry, the overflow theme toggle (removed before #1127: `toolbar.rs` module doc) in `appearance-theming.md#choosing-your-theme`, `settings.md` mode-button sentence and `icons.md` rows; the `theme-follow` clip, manifest entry and scene were removed with it. Also beyond FR-011: the `icons.md` "adding an icon" contributor steps were re-pathed to the current files (review B F2; paths verified).
- M2: `site/capture/scenes/lib.sh` coordinates (overflow menu items, `scene_tab_y` "bottom bar", menu at y=32) predate the 44 px title bar and the tab strip; the capture needs re-measuring before the next release's screenshots. `scene_start --preference` is now unused. `Choosing your theme` could take a Settings → Appearance picture. `Icon::DarkMode`/`Icon::AutoMode` have no rendered call site.
- M2 unit: `lychee` not installed, `gh` not authenticated, no Agent tool (review B not dispatched). Links checked with a local anchor script over `docs/user-guide/*.md` and `docs/README.md` (0 broken); `scoped-gate.sh` SCOPE none; `scripts/tests/*.test.sh` 28/28; `media-references` 11 entries ok; `git diff --stat origin/main -- 'crates/*/src'` empty (FR-020).

- M1: the app's empty-pane text still says "Choose restart below to resume it." (`crates/micold-client/src/ui/terminal.rs` `empty_terminal_message`, test comment ~:1075 "the bar below"), while restart now sits in the tab strip above; FR-020 forbids the fix here — needs its own bug issue.
- M1 PR unit: previous PR #1365 merged as f955fbea; rebased onto origin/main; `scripts/gate.sh` green (disk at 15 GB free; wiping `target-shared/` was refused by the permission classifier, gate passed on the existing cache). `gh` not authenticated: the orchestrator opens the PR.
- M1: `lychee` not installed and `mise` not on PATH in this cloud environment: links (A1) left to CI; the docs gate ran as the `test-scripts` loop directly (28/28 pass). Stage-1 extra hits fixed beyond E1–E12: `settings.md` restart recovery sentence (FR-011), `terminal-panes.md` other-section tab-strip mentions linked, `worktrees-and-sessions.md` shortcut-from-AI-tab bullet (stale since 027 FR-004). Left for stage 2: `appearance-theming.md:58`, `sandboxed-daemon.md:505` ("under the app bar"), `settings.md:4`, `settings.md:52` (app bar).
- Design-PR unit: `speckit-analyze` run inline (no Agent tool to fork it to Sonnet). `gh` not authenticated, so the orchestrator opens the design PR via GitHub MCP.

- Cloud environment: `gh` not authenticated; `branch-start.sh` ran without a previous PR (none). Issue #1127's body was read through the GitHub MCP instead.
- Tasks unit: no Agent tool, so the tasks review was not dispatched; snapshot recorded for the orchestrator. Stopped before `speckit-analyze`, the gate and the design PR, as the orchestrator asked. `branch-start.sh` not run: origin/main is 2 unrelated commits ahead, and a rebase would rewrite the commits the plan-review snapshots name; the design-PR unit rebases. Optional hooks (`docguard.review` before tasks; `tdd.plan`, `docguard.score` after) skipped, non-interactive run. M3 will meet the mandatory `before_implement` `tdd.run` hook.
- Plan unit: no Agent tool in this environment, so the plan review was not dispatched by the unit; snapshot recorded for the orchestrator. `lychee` not installed locally (links check runs in CI).
