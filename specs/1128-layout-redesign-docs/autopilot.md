# Autopilot ledger — #1128 layout-redesign-docs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1128 "Layout: docs, layout snapshot and visual pass for the redesign". Blocked by: #1124, #1125, #1126, #1127. Update user guide pages (worktrees-and-sessions, terminal-panes, overview, project-selection), regenerate the layout snapshot fixture, and run the visual pass at 1280x800 in both themes. Mockup: https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e; labels: enhancement, flow:feature (in-progress added by orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1128
- **Worktree branch**: claude/project-thread-bznqpy
- **Started**: 2026-10-24
- **Phase**: M1 implemented (T001–T011 ticked); docs gate and review A clean; awaiting review B
- **Next step**: orchestrator dispatches review B for M1 (snapshot in *Review rounds*), then continues the M1 unit for the full gate and the M1 PR. M2–M4 wait for #1125 and #1127 to merge.

Blockers at spec time: #1124 and #1126 merged; #1125 open (M1, M2 on main; M3–M5 todo); #1127 open, no spec. Spec stages the work: stage 1 (status bar, session area) now; stage 2 (sidebar, title bar, whole-window states, fixture, visual pass) after #1125 and #1127 merge.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1365 | Design (spec, plan, tasks) | merged (rebase) | f955fbea3ac738910219cb0daf5c61bdf6a03415 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Status bar and session-area mentions corrected and linked; no bar under the terminal, no unnamed "bar" | — | full gate green at 150a0410 after rebase onto origin/main; pushed, PR to open |
| M2 | T012–T020 | full | Sidebar and title bar in the user guide; FR-009 opening section; media alt text | — | blocked: #1125 and #1127 must merge |
| M3 | T021–T029 | full | `window-session-view`, `window-overview`, gate W, regenerated fixture, `layout-snapshot.md` passage | — | blocked: #1125 and #1127 must merge; after M2 |
| M4 | T030–T034 | full | Visual pass: 4 frames, 20 verdicts, contrast | — | blocked: after M3 |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Visual-pass defect in a shipped region: fix here or record? | Record as *differs by defect* + ledger follow-up; no code change | agent | spec#Context; 1124 ledger follow-ups; 1126 visual-pass.md |
| D2 | clarify | Sweep removed-element mentions in user-guide pages outside the four? | Yes, only those sentences/rows, staged by region | agent | constitution VII; spec#Context (closing pass) |
| D3 | clarify | Add two whole-window covered states or only regenerate? | Add both, then regenerate | agent | layout-snapshot.md#Adding a covered state, #What is covered |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A rounds 1–3 | MINOR: reflow the paragraphs whose added links pushed lines past 100 columns | The pages are not wrapped consistently (hundreds of lines over 100 columns in each touched file); reflowing would widen the diff past FR-011's "that sentence only". |
| M1 | B round 1 | F2 MINOR: worktrees-and-sessions.md "+" bullet restates the tab strip's "+" | Optional (FR-004 judgement call): the bullet is where that page explains opening instances, and it links to the tab-strip section rather than redescribing its layout. |
| — (design) | plan round 2 | MINOR: `contracts/covered-states.md:40` says each anchor is "held by W1–W4" under a W1–W5 table | W5 is a state-level assertion on the `State`, not on an anchor, so W1–W4 are the anchors' properties; the sentence is correct. |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M1: the app's empty-pane text still says "Choose restart below to resume it." (`crates/micold-client/src/ui/terminal.rs` `empty_terminal_message`, test comment ~:1075 "the bar below"), while restart now sits in the tab strip above; FR-020 forbids the fix here — needs its own bug issue.
- M1 PR unit: previous PR #1365 merged as f955fbea; rebased onto origin/main; `scripts/gate.sh` green (disk at 15 GB free; wiping `target-shared/` was refused by the permission classifier, gate passed on the existing cache). `gh` not authenticated: the orchestrator opens the PR.
- M1: `lychee` not installed and `mise` not on PATH in this cloud environment: links (A1) left to CI; the docs gate ran as the `test-scripts` loop directly (28/28 pass). Stage-1 extra hits fixed beyond E1–E12: `settings.md` restart recovery sentence (FR-011), `terminal-panes.md` other-section tab-strip mentions linked, `worktrees-and-sessions.md` shortcut-from-AI-tab bullet (stale since 027 FR-004). Left for stage 2: `appearance-theming.md:58`, `sandboxed-daemon.md:505` ("under the app bar"), `settings.md:4`, `settings.md:52` (app bar).
- Design-PR unit: `speckit-analyze` run inline (no Agent tool to fork it to Sonnet). `gh` not authenticated, so the orchestrator opens the design PR via GitHub MCP.

- Cloud environment: `gh` not authenticated; `branch-start.sh` ran without a previous PR (none). Issue #1127's body was read through the GitHub MCP instead.
- Tasks unit: no Agent tool, so the tasks review was not dispatched; snapshot recorded for the orchestrator. Stopped before `speckit-analyze`, the gate and the design PR, as the orchestrator asked. `branch-start.sh` not run: origin/main is 2 unrelated commits ahead, and a rebase would rewrite the commits the plan-review snapshots name; the design-PR unit rebases. Optional hooks (`docguard.review` before tasks; `tdd.plan`, `docguard.score` after) skipped, non-interactive run. M3 will meet the mandatory `before_implement` `tdd.run` hook.
- Plan unit: no Agent tool in this environment, so the plan review was not dispatched by the unit; snapshot recorded for the orchestrator. `lychee` not installed locally (links check runs in CI).
