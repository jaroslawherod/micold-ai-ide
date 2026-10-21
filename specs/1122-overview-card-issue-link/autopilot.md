# Autopilot ledger — #1122 overview-card-issue-link

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1122: Overview cards: link to the worktree's issue. Blocked by: #1120 (the branch line the link sits on). "Issue #N" beside the branch opens the GitHub issue in the browser; N comes from the worktree name, as the issue-title summary already does. A Jira key (MIC-87) in the name links to `<Jira base URL>/browse/<key>`, which needs a Jira base URL setting and shows no link while it is unset. A press on a link does not activate the card. No link on Default or on a worktree with no issue. Acceptance: both links open the right URL; Settings has the Jira field; card activation is unaffected; docs updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1122
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: close
- **Next step**: close: T051 (quickstart §A) and T052 (visual-pass.md) done; tdd-verify r1 requested (prompt `1122-close-prompt-tdd-verify-r1.md` in the session scratchpad). Then: act on its findings, close review r1, Status, PR text.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1180 | Design (spec, plan, tasks) | merged | 9ad05048 |
| #1188 | M1: link rules in core; shared `TextLink` in the showcase | merged | 0164f375 |
| #1195 | M2: the issue link on cards in the showcase | merged | c61380e1 |
| #1200 | M3: GitHub issue links on the Overview | merged | 32b7d671 |
| #1207 | M4: issue links by keyboard | merged | 65cefbbe |
| #1217 | M5: Jira base URL setting and Jira issue links | merged | fd2a64ef |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Link rules proven in core; showcase shows the shared `TextLink` | #1188 | merged |
| M2 | T010–T019 | full | Showcase shows cards with "Issue #1122", click opens only the link | #1195 | merged |
| M3 | T020–T030 | full | GitHub issue links on the Overview; rest on link and arrows as specified | #1200 | merged |
| M4 | T031–T038 | full | Issue links by keyboard (focus counts for the card, Menu key returns to link) | #1207 | merged |
| M5 | T039–T050 | full | Jira base URL setting (wire 60; 58 when planned, `main` moved to 59) and Jira issue links | #1217 | merged |

Cut notes: US1 (P1) split in three (M1–M3): the whole story is ~30 tasks and well over 800 lines, and each part has its own deliverable (core + `TextLink`; the card in the showcase; the app). M1 and M2 are showcase-only (`docs-not-needed`). M2 is 10 tasks: `ControlOverlay`, the card and the grid change together (one routing decision, R6–R8) and have no smaller observable split. US3 (M4) goes before US2 (M5), both P2, because it completes the keyboard path of the link M3 puts in the app. Rule 6 (review A tasks r1 F1): the link stays out of the Tab order until M4 (`TextLink::focusable(false)` while the card has no `.on_link_keyboard_focus`), so M3 leaves the keyboard exactly as before; design fixed in contracts/card-link.md U1/U3/U6 and research R5. Polish T051–T052 is code-free: close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How is the issue link reached by keyboard, given 1121's focus-expands rule? | A Tab stop right after its card; arrows move between cards only; focus on the link counts as focus on the card | agent-resolved | spec.md#FR-008; specs/1121-overview-card-full-text/spec.md#FR-002 |
| D2 | spec | A collapsed card's link moves when the card expands (lines are added above it); how is a click aimed at it kept off the card? | A pointer resting on a collapsed card's link does not expand the card | agent-resolved | spec.md#FR-007 |
| D3 | clarify | Do the issue links follow the integration switches (FR-001)? | Yes: GitHub link only while the GitHub issues switch (1102) is on, Jira link only while the Jira integration switch (533) is on | owner (card "Follow switches", the default confirmed; nothing changed) | spec.md#FR-001; Story 1 sc. 6, Story 2 sc. 7 |
| D4 | clarify | Where does the Jira base URL come from (FR-010)? | Per project: a base URL field on each project, for projects on different Jira sites. Stored as `jira_base_urls` keyed by `jira::connection_key` beside each project's Jira connection (feature 533 route: `DaemonSettings` map, one-project `SettingsSet` change merged on disk), edited for the open project in Settings → Jira, independent of the connection. M1's `JiraBaseUrl` unchanged; only M5 (T039–T050) amended | owner (card "Per project", replacing the app-wide default) | spec.md#FR-010; research.md#R3, #R4; contracts/wire.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | 399604367e0651717c12d766941fef7e892d58e9:48d810a53d17d50a45ab3b3ffaabec63220e6048 | CHANGES: 5 MAJOR, 3 MINOR (F1–F8), all fixed in spec.md, none declined |
| Review A spec | 2 | 5b39717f:af373d3b (fix diff, sonnet) | CHANGES: 1 MAJOR (F1 FR-005/FR-006 press-off-link contradiction, ripple on press), 1 MINOR (F2 SC-007 not measurable); both fixed, prose only, no round 3 |
| Review A plan | 1 | 200e0ca569743faecd1117df47127d69ad60e7c2:29f7e6c71c906f99358fc938b2e83ab638dfb25c | CHANGES: 3 MAJOR (F1 link Menu key origin, F2 focus pairing across the menu, F3 repo not re-read on project switch), 2 MINOR (F4 R6 scrollbar cursor, F5 R11 rejected alternatives); all five fixed in research/data-model/contracts/plan, none declined |
| Review A plan | 2 | dbc05dbcce3d2007986d9a9952f95f8041e77c0f:f8b66178b945888957cd553d9d889aff3a718669 (fix diff 29f7e6c7..f8b66178, sonnet) | CLEAN, 2 MINOR (F1 card-id fallback ends the menu hold, F2 deferred loss under an open menu); both fixed in data-model + K5, prose and test list only, no round 3 |
| Review A tasks | 1 | 1922d8f94766790a9b58f3c179918ec99ab12f8a:989fdbface58399752e833df82bfadf17c1174a6 | CHANGES: 4 MAJOR (F1 M3 Tab stop half-wired, F2 adopt test in a binary-only fn, F3 link-focus hand-offs untasked, F4 [P] tasks share files), 3 MINOR (F5 literals, F6 private daemon helpers, F7 D3/D4 tags); all fixed (F1 by keeping the link out of the Tab order until M4: contract U1 `.focusable`, U3), none declined |
| Review A tasks | 2 | 0368cc294df0c6db78913928089c602e7870219f:69649f215a54be0ba4162c45fea1cf8a177d54f7 (fix diff 989fdbfa..69649f21, sonnet) | CLEAN, 1 MINOR (F1 T033/T036 link-focus hand-off not tagged D3); fixed: tagged (D3) and added to the header list, prose only, no round 3 |
| Scoped gate M1 | 1 | 79f14e6c | RED: `idle_requests_no_frames` (TextLink's own `shell.request_redraw()`); fixed in 50a09809 (click via the stack's button) |
| Full gate M1 | 1 | 50a09809 | RED: flaky `micold-daemon attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` (untouched; 5/5 green alone); rerun GREEN |
| Review A M1 | 1 | 32c67355331e1a393de36645923df016972978d9:50a09809fe5596886ff266bd8cabaa061a0b467f | CHANGES: 1 MAJOR (F1 a click on a Tab-focused `press_takes_focus(false)` link dropped the keyboard, FR-005), 2 MINOR (F2 focus layer topped up for a child that draws no hover layer, F3 touch lift off the link captured); all three fixed with tests, none declined |
| Full gate M1 | 2 | f5e371b9 | GREEN (after review A r1 fixes) |
| Review A M1 | 2 | dd7dc7da9290a6eab81856708dc2fbb7f6246401:f5e371b9385f10e414dcf1e1c49c56e29aae6209 | CLEAN (sonnet) |
| Review B M1 | 1 | dd7dc7da9290a6eab81856708dc2fbb7f6246401:f5e371b9385f10e414dcf1e1c49c56e29aae6209 | CLEAN (sonnet, conformance) |
| Visual pass M1 | 1 | dd7dc7da9290a6eab81856708dc2fbb7f6246401:f5e371b9385f10e414dcf1e1c49c56e29aae6209 | C1, C2, C4, C5 PASS (light/dark x 1.0/2.0); C3 focus ring not observable: F1 MAJOR, the showcase had no Tab traversal. Fixed: Tab/Shift+Tab wired into the showcase (`state::key_message`, `ui::focus_move_task`) with tests. Evidence `visual-pass/m1/` |
| Full gate M1 | 3 | 75e361b6 | RED twice on untouched timing flakes (`micold-client` bin `pr_list_read_a_second_ask_shares_the_read_under_way`, 3/3 green alone; `micold-daemon structured_session messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal`); third run GREEN |
| Review A M1 | 3 | 58faf2edfdeef46ce65515e5f5352043d1c8dddf:75e361b63685cb7d4008e07f327c2a6ded0abe20 | CLEAN (sonnet, showcase Tab fix diff) |
| Visual pass M1 | 2 | 58faf2edfdeef46ce65515e5f5352043d1c8dddf:75e361b63685cb7d4008e07f327c2a6ded0abe20 | FAIL (sonnet, C3 only): F1 MAJOR, Tab/Shift+Tab had no visible effect anywhere in the showcase (light/dark x 1.0/2.0; Escape worked). Cause: the gallery poses a *focused* `TerminalPane`, which writes Tab to its process and captures the event, and `iced::keyboard::listen` hears only uncaptured keys, so the showcase never saw Tab (confirmed headless: the gallery returns `Status::Captured` for Tab; `focus_next` itself walks its 211 focusables). Fixed: the showcase listens with `iced::event::listen_with(state::event_message)`, which ignores the capture status; test `tests/showcase_tab.rs` drives Tab through the real gallery and hands its status to the listener (red with a captured-drop mutant). Proven live (Xvfb + xdotool, light 1.0): Shift+Tab x22/x23 from rest rings the caption and body links (`light-1.0-3-tab-fixed-ring-{caption,body}.png`); r2 evidence `*-3-tab-x*-no-ring.png` |
| Full gate M1 | 4 | 919dee35 | GREEN (after visual r2 fix) |
| Review A M1 | 4 | d8272a25bfb51c71040f9beb512c07def3356a3b:919dee35498cb93f597039ee8c3b154cf4385615 | CLEAN (sonnet, event_message fix diff) |
| Visual pass M1 | 3 | d8272a25bfb51c71040f9beb512c07def3356a3b:919dee35498cb93f597039ee8c3b154cf4385615 | FAIL (sonnet, C3 only): Tab/Shift+Tab reach both links in all 4 combos, ring in `secondary`, gone on tabbing away; F1 MAJOR the ring (3 px, drawn inside the link's box) sat flush with the label and painted over glyph tops, bottoms and the first glyph's left edge (caption read "ssue #1122"), light/dark x 1.0/2.0; observation: under focus the ring hid the hover underline. Fixed in part 4 (unit continued by the orchestrator's call for this one narrow fix, not an escalation): the link's button padded by `FOCUS_RING_WIDTH + 1` on every side (the material layer draws rings inside the control, `keyboard_focus::ring_bounds`, so no outside-ring pattern exists), underline clamped inside the label box; test `the_focus_ring_clears_the_label_on_every_side`, and `the_focus_layer_is_the_same_under_the_pointer` now requires the underline under focus (both red on the old padding). Evidence `visual-pass/m1/*-3-r3-*.png` |
| Full gate M1 | 5 | d9b86ce5 | RED twice on untouched environment/timing flakes (`micold-daemon mcp_create_session a_cli_that_would_ask_to_trust_the_folder_gets_no_first_prompt`: the fixture's `git commit` hit the signing server's 503, 3/3 green alone; `structured_session messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal`, the known flake); third run GREEN |
| Review A M1 | 5 | 4f755bbccbc4c6ea9031f7da1a821289a582dc9a:d9b86ce519f5531d2d490c5c71e0f710e3065a31 | CHANGES (sonnet): F1 MAJOR iced strokes a quad's border inside the quad, so the ring covers 1.5 to 4.5 px in from the link's edge and overlapped the 4 px gutter's label box by 0.5 px; the clearance test modelled a centred stroke. Verified in iced_tiny_skia-0.14.1 engine.rs (stroke path offset by half the width, inside `quad.bounds`) and iced_wgpu-0.14.0 solid.wgsl (`clamp(0.5 + dist + border_width)`). Fixed in part 5 (orchestrator's call for one narrow fix): `GUTTER = 2 * FOCUS_RING_WIDTH` (6 px, label on a whole pixel 1.5 px clear of the ring), test models the inner edge as `ring_bounds` shrunk by the whole width (red on the old 4 px gutter), `keyboard_focus` comment and assertion say the ring's outer edge is `ring_bounds` itself, inside the control. F2 MINOR underline clamp could lift it into glyphs under a tight line height: fixed, clamp dropped (`underline_y`), and the clearance test now covers every `TypeRole::ALL` role and asserts the underline is clear of the ring and at or below centre + 0.4 size |
| Visual pass M1 | 4 | 4f755bbccbc4c6ea9031f7da1a821289a582dc9a:d9b86ce519f5531d2d490c5c71e0f710e3065a31 | PASS (sonnet, C3 only): ring clear of the glyphs in light/dark x 1.0/2.0; observation: the horizontal gutter looks thinner than the vertical one (the line box adds vertical room), addressed by the r5 fix's wider gutter. Evidence `visual-pass/m1/*-3-r4-*.png` (40 PNGs) |
| Full gate M1 | 6 | d8d84697 | RED once on the known untouched flake (`micold-daemon attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice`, as gate r1); rerun GREEN |
| Review A M1 | 6 | 26eeeb1fc8c8cccda3ad3c1adc715ae6fe1e7d25:d8d8469762d85b03bea08e4c1fcc97e0d0a34bf8 | CLEAN (sonnet, r5 fix diff) |
| Review A plan amend D4 | 1 | 1a021e7be429ef5efa89f02f4283924163533d8b:0164f37579efae69cade704acdcc2f6633307cab | CHANGES (session model): F1 MAJOR the *Project forgotten* behaviour (spec edge cases, data-model) had no test task or quickstart step: fixed, T041 gains the forget/re-add case (`ProjectAdd` A, set, `ProjectRemove` A, entry still in `stored(dir)` and the next `DaemonSettings`, `ProjectAdd` A again resolves it by `connection_key`). F2 MINOR T045 did not say `JiraDraft::follow` (settings.rs:510) needs the map: fixed, T045 names the `base_urls: &BTreeMap<String, JiraBaseUrl>` parameter and its callers (persist.rs:288, daemon_sync.rs:1073, main_tests.rs:3270, settings.rs tests). F3 MINOR the moved `set` helper (jira_setting.rs:45) cannot send a base URL: fixed, T041 says `set` gains a `jira_base_url` argument, existing callers pass `None` |
| Review A plan amend D4 | 2 | c627583d8ac12b7a3566014f16b20b741c372013:ae2b013038751aac9eeaf86abab1620426bb12d9 | CLEAN (sonnet, fix diff ae2b0130..8ebea0f9) |
| Full gate M2 | 1 | d05aad46 | RED: `material_boundary` counted `Operation::<..>::scrollable(` in `ui/focus.rs` as building a wrapped widget; fixed d94cc82e |
| Full gate M2 | 2 | d94cc82e | GREEN |
| Review A M2 | 1 | 8daa7797f59f03b36df7d97c5e568026b04db9f8:d94cc82e9f7bb0d4f7527efd9a0f7261cfc16089 | CHANGES (session model, code-review high): F1 MAJOR the expanded card's twin, built with no messages, lays out no link, so a linked card with no branch line (the link on a line of its own) has its row measured a line short (FR-009): fixed a5984cdd, `OverviewCard::twin_of` gives the twin the expanded card's `on_issue_link` in `CardGrid::into_grid`; test `the_twin_of_a_linked_card_with_no_branch_keeps_its_row` (red: the row below moved 24 px). F2 MAJOR `over_link` used the link's translated but unclipped bounds, so with the expanded body scrolled the link counted as under a pointer on the title (rest and press routing): fixed a5984cdd, `ScreenBounds` keeps each scrollable's on-screen view and reports `visible()`, the bounds clipped to it; test `a_link_scrolled_out_of_the_body_is_not_under_the_pointer` (red: `["link rest 0"]`). F3, F4, F5 MINOR declined (see *Declined review findings*) |
| Review B M2 | 1 | 8daa7797f59f03b36df7d97c5e568026b04db9f8:d94cc82e9f7bb0d4f7527efd9a0f7261cfc16089 | CLEAN (sonnet, conformance). Verify: `--lib control_overlay overview_card card_grid` 124 passed; `layout_snapshot` 72; `showcase_completeness` 42; `idle_requests_no_frames` 14; five deliberate deviations judged acceptable (R11, U1 amended; snapshot additions only). F1 MINOR the cycle log's T018 row said 4 nodes per card, the snapshot adds 2: fixed a5984cdd |
| Visual pass M2 | 1 | 8daa7797f59f03b36df7d97c5e568026b04db9f8:d94cc82e9f7bb0d4f7527efd9a0f7261cfc16089 | PASS (sonnet, showcase §B1, light/dark x 1.0/2.0) = T019: C1 link whole at 280/380/480 collapsed and expanded, 120-char branch ellipsized before it; C2 line heights equal to a no-link card, link centred on the branch line, lone link flush; C3 pointing hand, 1 px underline, no hover layer under the link; C4 link press/release pixel-identical to hover, body press ripples; C5 Tab never stops on a link; C6 2.0 crisp. Observation (1), no card hover layer in any frame even without a link: not a regression, main (0164f375) had none either: its card wrapped the face in `button(face).style(\|_, _\| button::Style::default())`, which paints nothing in any status, the `Ripple` paints only a running ripple, and `TakesTheKeyboard`'s layer shows only under shown keyboard focus (`keyboard_focus.rs` `Focus::indicator` is `None` unless focused and visible); `ControlOverlay` keeps exactly that chain. Observations (2)–(6) are pre-existing or by design (overlay scroll-tracking offset is the 1121 pose's too). Evidence `visual-pass/m2/` (45 PNGs) |
| Full gate M2 | 3 | a5984cdd | GREEN (after review A r1 fixes) |
| Review A M2 | 2 | d94cc82e9f7bb0d4f7527efd9a0f7261cfc16089:a5984cdd | CLEAN (sonnet, fix diff d94cc82e..a5984cdd): F1, F2 fixes hold; declines F3 (touch: `keyboard_focus.rs:522` gives up the keyboard only on a left mouse press) and F5 (the per-frame probe walks one bounded card subtree, only the card under the pointer) confirmed. F1 MINOR `CardGrid::expanded` doc comment had a ~130-column line: fixed, prose only (re-wrapped), no round 3. Visual r2 not needed: neither fix changes what is drawn (the showcase's expanded linked card has a branch line, so its twin's height is unchanged; F2 changes pointer routing only) |
| Full gate M2 | 4 | c006b626 | GREEN (after review A r2 F1, comment only) |
| Full gate M3 | 1 | 5d362874 | RED: `anatomy_call_sites::the_test_module_convention_this_relies_on_holds`: `ui/mod.rs` had a second inline `#[cfg(test)]` module (the arrow test); moved to `ui/overview_arrow_tests.rs` |
| Full gate M3 | 2 | 06c27e10 | RED: `field_focus_call_sites::the_focus_message_reaches_the_traversal_operation` read the `CardStepped` arm's `Message::FocusMoved` fallback as the `FocusMoved` arm; the arm now calls `dialog_focus_move_task` itself |
| Full gate M3 | 3 | 44bbbce2 | RED: `overlay_registry::the_keyboard_subscription_names_no_surface` read `Msg::CardStepped` in `ui::subscription` as a per-surface message; exempted beside `FocusMoved` (it names a direction) |
| Full gate M3 | 4 | 707c9ec4 | GREEN |
| Review A M3 | 1 | ab166f69b8377f4ebb15d5973aa9a7a0b283887c:707c9ec42b5d20a4adc8dbca113b235a434c317c | CHANGES: 1 MAJOR (F1 `LinkRested` cleared `pointer_escaped` on the same card, so an Escape-collapsed card re-expanded after a rest on its link), 2 MINOR (F2 `card_link_id` fell in `card_id`'s namespace: worktree `link-x` = x's link; F3 a rest held by the menu survived a rest on that card's link); all fixed: F1 clears only on another card + test, F2 one `card_part_id` helper with `/` before the dir + test, F3 drops the held rest for the same card + test |
| Review B M3 | 1 | ab166f69b8377f4ebb15d5973aa9a7a0b283887c:707c9ec42b5d20a4adc8dbca113b235a434c317c | CLEAN |
| Visual pass M3 | 1 | ab166f69b8377f4ebb15d5973aa9a7a0b283887c:707c9ec42b5d20a4adc8dbca113b235a434c317c | PASS on all 9 criteria (T030); evidence in visual-pass/m3/ (133 PNGs). Observations, non-blocking: criteria 6–8 not run in every scale/theme combo; PATH-without-xdg-open variant of 7 not run |
| Full gate M3 | 5 | 7103e7a0 | GREEN (after the review A r1 fixes, rebased onto origin/main) |
| Review A M3 | 2 | 41736a28..7103e7a0 (fix commit; r1 snapshot stale after the rebase) | CLEAN (sonnet, fix diff) |
| Full gate M4 | 1 | 1ccd6669 | RED: clippy (`unused_mut` in an overview_card test, `cloned_ref_to_slice_refs` in a ui/overview test, `type_complexity` in a main_tests table) |
| Full gate M4 | 2 | c04408a4 | GREEN |
| Review A M4 | 1 | f2307f9a1e2ce80fb17368e3f6e21f999a3b0454:c04408a4053f06b61fd945b56ae10fe80024a386 | CHANGES: 2 MAJOR (F1 an unchanged `Listed` ends the Tab pairing, F2 link hand-off fires on a project switch), 2 MINOR (F3 blur keeps a held link focused, F4 `focus_return_link` a separate bool); all fixed test-first, none declined (verdict verbatim below) |
| Review B M4 | 1 | f2307f9a1e2ce80fb17368e3f6e21f999a3b0454:c04408a4053f06b61fd945b56ae10fe80024a386 | VERDICT: CLEAN |
| Visual pass M4 | 1 | f2307f9a1e2ce80fb17368e3f6e21f999a3b0454:c04408a4053f06b61fd945b56ae10fe80024a386 | FAIL on criteria 1 (reverse half), 6 and 7 (after-Escape half); 2–5, 8, 9 PASS (verdict verbatim below; evidence visual-pass/m4/). Root cause: the link sits in a different place in the collapsed and expanded body, so a change of form rebuilds its focus state; fixed by the root handing the keyboard back to the link (`link_focus_after`) |
| Full gate M4 | 3 | 4faae67f | GREEN (after the r1 fixes) |
| Review A M4 | 2 | 69879e504c90cc162144dff73698687dd0461b02:532bd52a1e3c706506bbb096f732a55e4a2e22f1 | VERDICT: CLEAN; 1 optional MINOR (F1 `LinkFocus::of` clones on every message), declined (verdict verbatim below) |
| Visual pass M4 | 2 | 69879e504c90cc162144dff73698687dd0461b02:532bd52a1e3c706506bbb096f732a55e4a2e22f1 | PASS on all re-checked criteria (1, 6, 7; regression 2, 4); 64 r2 PNGs in visual-pass/m4/ (verdict verbatim below). T038 ticked |
| Full gate M5 | 1 | af17c7fd | RED: clippy `large_enum_variant` on `features::runs::Effect` (`ClientMsg` grew with `SettingsSet.jira_base_url`); fixed b4ba37fd (`Option<Box<JiraBaseUrlChange>>`) |
| Full gate M5 | 2 | b4ba37fd | GREEN (core changed, so the scoped gate would have been the whole workspace: the full gate ran instead) |
| Review A M5 | 1 | 3d085ccd3ea3965bbcf29c77c4afac48bce8b959:b4ba37fd9cdca22050adfb90b0188afebd7014fb | VERDICT: CLEAN; 3 MINOR: F2 (typed trailing `/` freezes the draft) fixed test-first, F3 (subtitle) fixed, F1 (double `SettingsChanged` echo) declined (verdict verbatim below) |
| Review B M5 | 1 | 3d085ccd3ea3965bbcf29c77c4afac48bce8b959:b4ba37fd9cdca22050adfb90b0188afebd7014fb | VERDICT: CLEAN; 1 MINOR F1 (wire.md W2/W3 stale) fixed, prose only (verdict verbatim below) |
| Visual pass M5 | 1 | 3d085ccd3ea3965bbcf29c77c4afac48bce8b959:b4ba37fd9cdca22050adfb90b0188afebd7014fb | PASS on all 8 criteria; 41 PNGs in visual-pass/m5/; T050 ticked (verdict verbatim below). Observation 1 (read-only window switched B → A shows no "Issue #1122") is ours: M3's repo read fires only on `Attached`, and a read-only window's attach is answered `Refused { ProjectBusy }`; fixed test-first (`repo_reread_on_a_read_only_switch`) |
| Full gate M5 | 3 | 44164707 | RED: `micold-daemon --test attention_error_notice` `without_focus_the_first_reporting_window_gets_the_notice` (`left: []`), a timing flake outside this diff (no daemon file changed since the green r2); 5 reruns of that test passed |
| Full gate M5 | 4 | 44164707 | GREEN (after the r1 fixes) |
| Review A M5 | 2 | 660cc05a9c3777cc54b0584f8a87eceffd625c2a:44164707544ad85d3bc0c6988bba72fbf08a9e3f | VERDICT: CLEAN, no findings; F1's declined reason confirmed (verdict verbatim below) |
| Visual pass M5 | 2 | 660cc05a9c3777cc54b0584f8a87eceffd625c2a:44164707544ad85d3bc0c6988bba72fbf08a9e3f | PASS on both criteria (read-only switch link, subtitle); r2 PNGs in visual-pass/m5/ (verdict verbatim below) |
| Converge close | — | — | skipped: review B CLEAN for M1–M5 (close.md step 1) |
| Full gate close | 1 | fd2a64ef | RED: feature 942's `structured_permission::a_pending_request_marks_its_session_waiting_until_the_last_is_decided` (3.0 s under load); passes 5/5 alone |
| Full gate close | 2 | fd2a64ef | GREEN (T051) |
| TDD verify close | 1 | 6f552538a137b82b74c095c00793ecaef9b68684:fd2a64ef85925313a77d395156792aed6ece370a | pending (sonnet, autopilot-worker) |


### M4 r1 verdicts (verbatim)

<details><summary>Review A M4 r1</summary>

#### 1122 M4 review A r1 (snapshot f2307f9a..c04408a4)

VERDICT: CHANGES
F1 [MAJOR] crates/micold-client/src/ui/overview.rs:365 (with crates/micold-core/src/overview_expansion.rs:154) — The forward-Tab pairing breaks whenever a card is still expanded after the card's `FocusLost`. `App::update` calls `settle_card_menu` after every outermost update (app.rs:1121). When `expansion.expanded().is_some()`, `settle_card_menu` applies `Event::Listed`. `apply` takes `just_lost` on every event, so `Listed` uses up the one-event snapshot that sits between `FocusLost(A)` and `LinkFocusGained(A)`, and the link's gain then counts as a new focus. Example: Escape collapses focused card A, and the pointer rests on card B, which expands by pointer. Tab from A to its link should keep A collapsed and B expanded (FR-008, K5). Instead A re-expands with `Cause::Focus`, `focus_escaped` is cleared, and B collapses under the pointer. The tests miss this because no card is expanded during their Tab.
   Evidence: `let watched = expansion.expanded().is_some() || expansion.focused_link().is_some(); … if watched { state.overview.expansion.apply(Event::Listed(listed.clone())); }` and `let just_lost = self.just_lost.take();` at the top of `apply`.
   Fix: make `Listed` keep `just_lost` (put it back when nothing in the snapshot is gone), or have `settle_card_menu` skip `Listed` when the listing is unchanged. Add a K5 test where B is expanded by pointer: `focus(a)`, `Escape`, `rest(b)`, `FocusLost(a)`, `Listed([a,b])`, `LinkFocusGained(a)`. It should end with B expanded and A escaped.
F2 [MAJOR] crates/micold-client/src/main.rs:737-741 — `link_focus_handoff` fires on a project switch. If a link holds the keyboard when the project changes, `held_link` is the old project's location. After the update, `focus_return_target` cannot find that card among the new project's cards and returns the new first card, and the hand-off focuses it. No `OpenFocused` precedes that focus (and `ProjectSwitched` reset the expansion), so the first card's `FocusGained` is a new focus and it expands. The 972 card path moves no focus on a switch, and the Overview's own opening expands none (FR-013).
   Evidence: `let target = focus_return_target(&app.core, &location)?; if target != location { return Some(card_id(&target)); }`. Nothing here checks that the project is still the same.
   Fix: in `update`, record the active project next to `held_link`. Skip the hand-off when the project changed, or when `held` is not a location of the current project. Add a main_tests case: link focused, then switch project, and no focus task follows.
F3 [MINOR] crates/micold-core/src/overview_expansion.rs:231-235 — `WindowBlurred` clears `link_held` without applying the deferred loss. This leaves `focused = A`, `focus_part = Link` while the link's menu still holds the keyboard (the blur does not close the menu). `focused_link()` then returns `Some(A)`, so `shown_focused_link` and `link_focus_handoff` treat the link as focused under the open menu. If a choice is then made, the card's gain turns the part into `Both` even though the link never got the keyboard back. A later press elsewhere drops only the card's part, so A stays "focused" and expanded by `Cause::Focus` with nothing holding the keyboard.
   Evidence: `Event::WindowBlurred => { …; self.link_held = None; self.collapse(); }`, which never calls `lost_plain(held)`.
   Fix: in `WindowBlurred`, call `if let Some(held) = self.link_held.take() { self.lost_plain(held) }`, or keep `link_held` across the blur. Add a K5 test: `LinkFocusGained(a)`, `MenuOpened`, `LinkFocusLost(a)`, `WindowBlurred`, then `focused_link()` is `None`.
F4 [MINOR] crates/micold-client/src/features/overview.rs:81 — `focus_return_link` is a separate bool that has to be cleared at the same moment as `focus_return` at four sites. Any future site that clears `focus_return` and forgets the bool leaves a stale `true`, and a later card-menu return then goes to the link.
   Evidence: `state.focus_return = None; state.focus_return_link = false;` is repeated in ProjectSwitched, twice in `settle_card_menu`, and in the dismiss arm.
   Fix: fold the two into `focus_return: Option<(SessionLocation, ReturnTo)>`, or a small struct.

</details>

<details><summary>Review B M4 r1</summary>

VERDICT: CLEAN

</details>

<details><summary>Visual pass M4 r1</summary>

#### 1122 M4 visual pass r1 (T038, HEAD 622b9f8e)

VISUAL PASS M4 (T038) r1: FAIL on criteria 1 (reverse half), 6 and 7 (after-Escape half); the rest PASS. 170 PNGs in /home/user/micold-ai-ide/specs/1122-overview-card-issue-link/visual-pass/m4/, named <theme>-<scale>-<phase>-<n>-<what>.png. Client and daemon were built from the worktree at HEAD 622b9f8e (source = c04408a4; later commits are docs). Run on Xvfb :77 with lavapipe and a throwaway HOME and data dir. Pointer parked at the screen corner after opening the Overview. All four theme x scale combos were run for every criterion except where noted.

1. FAIL (forward half PASS; scale 1.0 and 2.0, light and dark)
   - Forward: Tab onto card 1122 expands it; Tab again puts the ring on the link with the card still expanded; Tab again moves to card 1124, which expands, and 1122 collapses. Files: light-1.0-1-1, 1-2, 1-3. Shift+Tab from the link goes back to its own card, then to Default (light-1.0-1-6, 1-7).
   - Reverse from a card onto the previous card's link does not work. Shift+Tab from card 1124 expands card 1122 but draws no ring on its link. Return, Space and the Menu key do nothing, and focus is lost. The next Shift+Tab wraps to the last card (`feat_no-ticket`). See light-1.0-1-4 and 1-5, and the same in dark and at 2.0.
   - The same happens from card 1125 onto link 1124 (checked once, not captured). The card looks expanded but the link is not focused.

2. PASS: the ring sits in the link's gutter, clear of the label, with no underline and no pointer. Zooms: light-1.0-2-ring-zoom.png and light-2.0-2-ring-zoom.png, crisp and aligned at 2.0. Dark checked in dark-1.0-3-2 and dark-2.0-8-4.

3. PASS: on 1125 expanded with its link focused, the ring directly under the title is whole, with its top edge visible. Files: <theme>-<scale>-3-2-tab-link1125-focused-first-row.png, checked in light 1.0 and 2.0 and dark 1.0.

4. PASS: Enter on the link gives xdg-open exactly `https://github.com/o/r/issues/1125`, one call. Space gives one more identical call, and the same for 1122. The Overview stays open, no ripple (frames at +0.12 s) and nothing selected. Files: 4-1 to 4-5. Also holds in full screen (9-2).

5. PASS: from the focused link of 1122, Right and Down go to card 1124 and Left and Up go to Default; none lands on a link. Files: 5-1 to 5-4.

6. FAIL
   - Escape on the focused link collapses the card, but the ring does not stay on the link; no ring is drawn anywhere (6-1).
   - Shift+Tab then wraps to the last card (6-2), and Tab does not return to the link (6-3).
   - Escape with nothing expanded closes the Overview (6-4), which PASSES.

7. FAIL on the after-Escape half; the rest PASS.
   - Menu key and Shift+F10 on the expanded link open the card menu anchored at the link, first entry "Open" highlighted. Escape closes it and the ring is back on the link with the card still expanded (7-1, 7-2, 7-5, 7-6).
   - After an Escape-collapse, the Menu key opens nothing because focus is lost (7-3, 7-4).

8. PASS: the card 1124 body scrolls, and Tab onto its link keeps the link in view with its ring. Page Down and Page Up from the link scroll the body like from the card (8-1 to 8-7). The link was already visible before the Tab, so no scroll was needed. At 2.0 the 120-char card is wide, so check dark-2.0-8-* and light-2.0-8-* for overflow.

9. PASS: in full screen, forward Tab, Enter and Menu hold (9-0 to 9-4); the reverse failure is the same as in criterion 1.

Observations:
- The reverse and Escape failures share one pattern: whenever a card's expansion changes without the user having tabbed forward onto the link, the link loses keyboard focus. The card's layout appears to change when it expands or collapses, which resets the focus state, but I did not trace the code.
- `feat-1125_first` as a branch does not make the link the first row. The display name derives to "First", shows as its own row, and a worktree name override equal to the directory name is ignored. I used branch `feat/1125_first` with a name override of `feat/1125_first`, so title = branch = name and the link is the first row.
- At scale 2.0 the grid is a single column and scrolls.
- Default holds keyboard focus on fresh open.
- Cleanup: Xvfb, client, daemon, the run directory and temp data are removed.

</details>

### M4 r2 verdicts (verbatim)

<details><summary>Review A M4 r2</summary>

#### 1122 M4 review A r2 (snapshot 69879e50..532bd52a)

VERDICT: CLEAN

I found no BLOCKER or MAJOR in the round-2 fix diff. I read the diff against the milestone brief and contracts, and I did not invoke the `code-review` skill. I ran only the core tests (`cargo test -p micold-core overview_expansion`: 51 passed, 0 failed). I did not run the client crate's tests or the gate.

Round 1 fixes:
- F1 holds. `Listed` clones the state after `just_lost` is taken, so a listing that changes nothing compares equal and puts the snapshot back. A listing that removes a tracked card still ends the pairing.
- F2 holds. `link_focus_after` returns `None` when `workspace.active` changed, before any hand-off.
- F3 holds. `WindowBlurred` applies the deferred loss through `lost_plain` and then collapses as before.
- F4 holds. `FocusReturn { location, to_link }` is armed and cleared as one value, and `card_focus_return_id` destructures it.

New focus-restore code (`link_focus_after`), including the redundant focus you flagged:
- Under an open link menu, `focused_link()` is `None` while `link_held` is set, so nothing is restored. The `MenuClosed` update is covered by the same check. The return to the link or card then comes only from `card_focus_return_id` and the `link_held == location` branch of `gained`.
- On Overview close, `shown_focused_link` requires `overview.open`, so it returns `None`.
- Together with `card_focus_return`: the two never name different controls in one update. A card return (an entry chosen) leaves `focus_part` as Card, so no link is focused after the update.
- A forward Tab onto the link gives a redundant focus of the already-focused link. The follow-up `LinkFocusGained` hits the "already holds, same part" early return in `gained`, so core state is unchanged.
- Moving the keyboard from one link to another gives the same harmless extra focus.
- Shift+Tab from a link to its own card is not a misfire: the link is no longer focused after the update, so there is no restore.
- Pointer rest or Escape changing the form of the focused link's card restores the link. That is the intended behaviour.

F1 [MINOR] crates/micold-client/src/main.rs:~735 — `LinkFocus::of` runs on every message and clones the active project `PathBuf` and the expanded location each time, even with the Overview closed.
   Evidence: `project: app.core.workspace.active.clone()` in `LinkFocus::of`, called unconditionally at the top of `update`.
   Fix: optional. Compare a cheap value instead (for example only snapshot the project when `shown_focused_link` is `Some`), or accept the cost.

</details>

<details><summary>Visual pass M4 r2</summary>

#### 1122 M4 visual pass r2 (T038, HEAD ce44c8a9)

VISUAL PASS M4 (T038) r2: PASS on all criteria (1, 6, 7, 2, 4). Built daemon and client from HEAD ce44c8a9 (code 4faae67f) via scripts/build-lock.sh. Ran on Xvfb :77 with lavapipe, a throwaway HOME and data dir, a fake xdg-open, origin git@github.com:o/r.git, and the same four worktrees as round 1. Pointer parked at the screen corner after opening the Overview; keyboard only. 64 new PNGs are untracked in /home/user/micold-ai-ide/specs/1122-overview-card-issue-link/visual-pass/m4/, named <theme>-<scale>-r2-<criterion>-<n>-<what>.png. Source untouched, nothing committed.

Coverage: light 1.0 ran every criterion; dark 1.0 ran 1, 6 and 7; light 2.0 ran 1 and 6.

1. PASS (forward and reverse; light 1.0, dark 1.0, light 2.0)
   - Forward: Tab onto card 1122 expands it (r2-1-1). Tab puts the ring on its link with the card still expanded (1-2). Tab goes to card 1124, which expands, and 1122 collapses (1-3).
   - Reverse: Shift+Tab from 1124 expands card 1122 and the ring is on its link (1-4).
   - Enter there: xdg-open gets exactly `https://github.com/o/r/issues/1122`, once (1-5). The Overview stays open and the ring stays.
   - Shift+Tab again: card 1122 itself, still expanded, no ring on the link (1-6). Shift+Tab again: Default (1-7).
   - From card 1125 (1-8), Shift+Tab lands on the ring on link 1124 (1-9). Enter there gives `https://github.com/o/r/issues/1124`.
   - Same files exist for dark-1.0 and light-2.0 (`light-2.0-r2-1-4` and `1-6` checked: ring on the link, then card focused with no ring on the link).

6. PASS (light 1.0, dark 1.0; light 2.0 for steps 6-1 to 6-4)
   - Escape on the focused link collapses the card and the ring stays on the link (6-1).
   - Shift+Tab goes to its own card, still collapsed (6-2). Tab goes back to the link, still collapsed, ring on the link (6-3). Tab again focuses the next card (1124) and it expands (6-4).
   - With Default focused and nothing expanded (6-5), Escape closes the Overview (6-6).

7. PASS (light 1.0, dark 1.0)
   - Menu key on the focused link of the expanded card opens the menu at the link with "Open" highlighted (7-1). Escape: ring back on the link, card still expanded (7-2).
   - Shift+F10 behaves the same (7-3, 7-4).
   - After Escape-collapse (7-5, ring on the link), the Menu key opens the menu anchored at the link (7-6). Escape closes it, the ring is back on the link and the card stays collapsed (7-7).

2. PASS (light 1.0): the ring sits in the link's gutter around the label, clear of the text, with no underline and no pointer. Zoom crop of light-1.0-r2-2-1-link-ring.png checked.

4. PASS (light 1.0)
   - Enter on the link: xdg-open gets exactly the 1122 URL, once (r2-4-1, r2-4-2).
   - Space: one more identical call, so two calls in total (r2-4-3, r2-4-4).
   - The Overview stays open, no ripple at +0.12 s, nothing selected.

Observations:
- Light 2.0 grid is a single column and scrolls. The crop is full-window, so the ring and card form are visible in those shots.
- I did not capture dark 2.0 or full-screen shots, as the prompt did not ask for them in r2.
- Cleanup is done: temp fixture, data and HOME dirs, pinned binaries, `.visual-pass/run.*` removed. The Xvfb, client and daemon processes were killed; they still show as defunct entries (zombies) that the harness has not reaped.

</details>

### M5 r1 verdicts (verbatim)

<details><summary>Review A M5 r1</summary>

# 1122 M5 review A r1 (snapshot 3d085ccd:b4ba37fd)

VERDICT: CLEAN
F1 [MINOR] crates/micold-daemon/src/server.rs:1404-1419 — Each setter in the `SettingsSet` chain broadcasts its own `SettingsChanged`, so a save that has both `jira_enabled`/`jira_connection` and `jira_base_url` first echoes the base-URL map from before the save. `adopt_daemon_settings` (daemon_sync.rs:1011) then overwrites the change the client already applied, and the Overview's Jira links flicker off until the `set_jira_base_url` echo arrives. If that later write fails, they stay off. The same intermediate-echo pattern already exists for other fields, and the final state is correct.
   Evidence: `catalog.rs:577/589` — `set_jira_connection` and `set_jira_base_url` each call `persist_service_settings_with(ProjectJiraChange { <one field>, ..default() })`; no caller fills both fields.
   Fix: Add one state/catalog method that takes both Options, passes them in a single `ProjectJiraChange` and broadcasts once, which is what the struct was built for. Otherwise drop the struct.
F2 [MINOR] crates/micold-client/src/features/settings.rs:529,548 — `base_untouched` compares the raw typed text with the stored value's text, but `JiraBaseUrl::parse` normalises (it trims and strips a trailing `/`, jira.rs:182). If the draft outlives a save where the user typed `https://x.atlassian.net/`, the field counts as edited for good. It then stops following other windows' changes, and a later save can undo them.
   Evidence: `let base_untouched = self.base_url == Self::base_text(self.stored_base_url.as_ref());`
   Fix: Compare parsed values (`JiraBaseUrl::parse(self.base_url.trim()).ok() == self.stored_base_url`, with empty treated as None), or re-seed `base_url` from the stored value after a successful save.
F3 [MINOR] crates/micold-client/src/ui/settings/jira.rs:67 — The page subtitle still says "Applies to every project.", but the new base URL field shown under it is stored per project (docs/user-guide/settings.md says each project has its own value).
   Evidence: `"Start worktrees from the issues of a Jira Cloud project. Applies to every project."`
   Fix: Say that the switch applies everywhere while the connection and base URL belong to the open project.

</details>

<details><summary>Review B M5 r1</summary>

# 1122 M5 review B r1 (snapshot 3d085ccd:b4ba37fd)

VERDICT: CLEAN
F1 [MINOR] specs/1122-overview-card-issue-link/contracts/wire.md:W2, W3 — The contract was not updated for two M5 deviations. W2 still reads `Option<JiraBaseUrlChange>`, but the code has `jira_base_url: Option<Box<JiraBaseUrlChange>>` (the clippy fix in b4ba37fd). W3 still reads "57 → 58" and "daemon.md names version 58", but the code and docs say 59 → 60.
   Evidence: messages.rs `jira_base_url: Option<Box<JiraBaseUrlChange>>`; wire.md "## W3 — `PROTOCOL_VERSION` 57 → 58"
   Fix: Edit wire.md W2 to say the field is boxed and the postcard layout is unchanged. Edit W3 to 59 → 60 and note that `main` took 58 and 59 (feature 942).

Verify results (output in /tmp/claude-0/-home-user-micold-ai-ide/9f964f32-37d5-5fbc-86f4-e611005b6fc8/scratchpad/b-r1-verify.txt). All of the listed commands pass:
- core: protocol_roundtrip 41, schema_hash 22, settings_roundtrip 42
- daemon: jira_base_url_setting 6, jira_setting 5
- client: features_settings 138, layout_snapshot 74, overview 75, settings_sections 24, `--bin micold-ai-ide -- jira` 15

The five deliberate deviations are acceptable:
- The 59 → 60 bump is a single bump with the schema_hash pin moved in the same commit.
- The JiraDraft tests sit in `tests/features_settings.rs`, beside the existing Jira draft tests.
- The adopt test is split between `tests/overview.rs` and `src/main_tests.rs`.
- The `layout_snapshot` re-bless matches the cycle-log claim: `overview-issue-link` plus the five `settings-view-jira-on-*` states.
- The settings_sections check is a source scan, as for the other sections.

Scope, constitution, ownership and leftovers checks are clean:
- The diff since M4 touches only M5 files. No `todo!`, `dbg!` or similar was added.
- T050 is correctly unticked.
- The cycle log shows red before green for each behaviour.
- The user guide (settings.md, overview.md) and daemon.md are updated.

</details>

<details><summary>Visual pass M5 r1</summary>

# 1122 M5 visual pass r1 (T050, HEAD 6c0e732f)

VISUAL PASS M5 (T050) r1: PASS on all 8 criteria. Built daemon and client from HEAD 6c0e732f (built tree matches b4ba37fd per the prompt; the only later change is autopilot.md) via build-lock.sh. Both binaries were pinned together. Ran on Xvfb :77 with lavapipe, a throwaway HOME/data dir, a fake xdg-open, and fixture repos A (origin git@github.com:o/r.git, 3 worktrees) and B (feat-mic-87_x). Source untouched, nothing committed. 41 untracked PNGs are in /home/user/micold-ai-ide/specs/1122-overview-card-issue-link/visual-pass/m5/. All paths below are relative to that directory. Temp dirs, processes and the Xvfb are cleaned up.

Two-window checks ran at light 1.0 with the windows side by side at 800 px each (names `light-1.0-*`). A single window ran the 4 theme x scale combos. A and B opened in different windows, since a project can only be writable in one.

1. PASS (sc.1). A's Overview shows "Issue #1122"; Default, feat_no-ticket and feat-mic-87_add-login show no link. Files: light-1.0-1-cards-no-base-url, dark-1.0-1-cards-no-base-url, light-2.0-1-cards-no-base-url, dark-2.0-1-cards-no-base-url (.png).

2. PASS (sc.3, FR-010). The "Jira base URL" field sits after the four connection fields, is empty, has no required mark, and shows "Used to link Jira keys on this project's Overview cards." With the Jira switch off the field is not shown (light-1.0-2-switch-off-no-field.png). Field shots: {light,dark}-{1.0,2.0}-2-jira-base-url-field.png.

3. PASS (sc.2, sc.5, sc.6, FR-012). Saved `https://example.atlassian.net/` with the connection fields blank and nothing was marked.
   - "Issue MIC-87" appeared in window 1 and in window 2 (read-only on A) within about 1 s, with no restart: light-1.0-3-both-windows-after-save.png.
   - Clicking the link made the fake xdg-open receive exactly `https://example.atlassian.net/browse/MIC-87`, once. The Overview stayed open with no ripple and nothing selected: light-1.0-3-after-click-w1.png.
   - Reopened Settings shows `https://example.atlassian.net` without the trailing slash, and settings.json stores it that way: light-1.0-3-settings-reopened-stored-no-slash.png, light-2.0-3-settings-stored-no-slash.png.
   - The same click gave the same URL in dark 1.0, dark 2.0, light 2.0 and light 1.0 full-window.
   - Card shots: {light,dark}-{1.0,2.0}-3-cards-after-save.png.

4. PASS (sc.8). Window 2 on B: the feat-mic-87_x card had no link and B's field was empty (light-1.0-4-w2-projectB-card-no-link.png, light-1.0-4-w2-projectB-field-empty.png).
   - After saving `https://b.atlassian.net` the B link appeared and opened `https://b.atlassian.net/browse/MIC-87` (light-1.0-4-w2-B-link-after-save.png, light-1.0-4-w2-after-click-B.png).
   - Window 1 on A kept its link, opened the example.atlassian.net URL, and A's field kept its value.
   - settings.json held both entries under `jira_base_urls`: repoA `https://example.atlassian.net`, repoB `https://b.atlassian.net`.

5. PASS (sc.4, §B7, FR-011, SC-006). `http://x`, `https://u@x`, `https://x?q`, `https://x#f` and `https://` were each refused. The field is marked red with "Enter the https address of your Jira, e.g. https://acme.atlassian.net.", Settings stays open on Jira, and the stored value is unchanged. The card still opened the example.atlassian.net URL afterwards. Files: light-1.0-5-refused-http, -2 to -5 (the other four values), light-1.0-5-card-still-links-after-refusals. Marked-field shots of `http://x` in all four combos: {light,dark}-{1.0,2.0}-5-refused-http.png.

6. PASS (§B7). Cleared A's field and Saved: A's MIC-87 link went in window 1, and in window 2 with it switched to A read-only (light-1.0-6-both-windows-A-after-clear.png). B's link stayed (light-1.0-6-w2-back-on-B-link-stays.png), and settings.json then held only repoB. An earlier clear with window 2 on B is in light-1.0-6-both-after-clear.png.

7. PASS (sc.7, D3). Switching the Jira integration off and Saving removed both A's MIC-87 link and B's link, with `jira_enabled` false and the values kept in settings.json. No Keep prompt appeared, because there is no connection. Switching it on again brought both links back without a restart. Files: light-1.0-7-switch-off-both-no-links.png, light-1.0-7-switch-on-both-links-back.png.

8. PASS. The Jira link is the same primary colour and style as the GitHub link, and the whole label is the link. Tab onto it shows the ring in the link's gutter, clear of the text. Files: light-1.0-8-tab-github-link-ring.png and light-1.0-8-tab-jira-link-ring.png (side by side for comparison), plus dark-1.0, dark-2.0 and light-2.0 `-8-tab-jira-link-ring.png`.

Observations (non-blocking):
- A read-only window that was switched from B to A did not show "Issue #1122" on A's 1122 card, even about 5 s later. It did show the link when it first opened on A, and the writable window showed it throughout. The Jira links were correct in that window. I did not isolate the cause and did not check whether it predates 1122.
- The first ring/Tab shot for light 1.0 (card focused, not link) also landed in light-1.0-8-tab-jira-link-ring.png. The final file in the set is the Jira link ring, but the Tab count differs from the single-window combos.
- At 2.0 a single column scrolls. An expanded card overlaps the card above it (also true earlier in M4).
- The environment-include "script not found" toast appears after every Save. It is harness noise because HOME has no .bashrc.
- The "Enable Jira integration" switch was pre-seeded on in settings.json, not toggled from the UI to start. Toggling it off and on was exercised in criteria 2 and 7.

</details>

### M5 r2 verdicts (verbatim)

<details><summary>Review A M5 r2</summary>

# 1122 M5 review A r2 (snapshot 660cc05a:44164707)

VERDICT: CLEAN

No findings. I checked the r2 fix diff against the points in the brief:
- F2: `typed_base_url()` is shared by `follow` and `base_url_change`. The `base_url_change` error mapping is unchanged. The new test `a_typed_trailing_slash_that_was_saved_still_follows_the_map` passes, along with the rest of `jira_base_url_form` (7 passed).
- F3: the subtitle is accurate. The switch is global, and the connection fields and base URL are per project. The layout snapshot change touches only the five subtitle widths.
- F1 declined: the reason stands. `set_jira_enabled`, `set_jira_connection`, `set_github_issues_enabled` and `set_jira_base_url` each call `broadcast(SettingsChanged)` separately in the `SettingsSet` chain (`crates/micold-daemon/src/server.rs` ~1404–1419). Merging connection and base URL would not remove the echoes that carry the old maps.
- Read-only switch fix:
  - The `Refused { ProjectBusy }` arm calls `read_repo` only when the refused project is the active one and the Overview is open.
  - The BUG-022 own-instance early return above it is unaffected.
  - `read_repo` itself returns when the repository is already known, so a reconnect refusal with the repository known sends nothing.
  - `Displaced` needs nothing, since the project does not change.
  - `repo_reread_on_a_read_only_switch` and `repo_reread_on_attach` both pass (2 passed).

</details>

<details><summary>Visual pass M5 r2</summary>

# 1122 M5 visual pass r2 (HEAD a29e02b4)

VISUAL PASS M5 (T050) r2: PASS on both criteria. Built daemon and client from HEAD a29e02b4 (code 44164707) via scripts/build-lock.sh with the build env. Ran on Xvfb :77 with lavapipe, a throwaway HOME and data dir, a fake xdg-open, fixture repos A (origin git@github.com:o/r.git, worktrees feat-1122_overview-issue-link, feat-mic-87_add-login, feat_no-ticket) and B (feat-mic-87_x), and Jira and GitHub issues switched on. No source edited, nothing committed. Evidence PNGs are untracked in /home/user/micold-ai-ide/specs/1122-overview-card-issue-link/visual-pass/m5/. Xvfb, client, daemon, temp dirs and the .visual-pass build dir are cleaned up (one defunct Xvfb zombie entry remains in the process list).

1. PASS (light 1.0, two windows side by side at 800 px).
 - Window 1 held A (writable). Window 2 first opened on A read-only (r2-0-start.png), then was switched to B with the Overview open (r2-1-w2-on-B-overview.png).
 - Switching window 2 to A showed "Issue #1122" on the 1122 card within 4 s, with the read-only banner present (r2-1-w2-switched-to-A-4s.png).
 - Clicking the link made xdg-open receive exactly `https://github.com/o/r/issues/1122`, once (r2-1-w2-A-after-click.png). The log had one line after all the later switches too.
 - Window 2 back on B (r2-1-w2-back-on-B.png) and on A again, twice (r2-1-w2-A-again-2nd.png, r2-1-w2-A-again-3rd.png): the link was there each time.
 - Window 1 kept its link throughout (visible in every shot above).
 - The round-1 observation (no link after B to A in a read-only window) is fixed.

2. PASS. Settings, Jira, switch on. The subtitle reads "Start worktrees from the issues of a Jira Cloud project. The switch applies to every project, the fields below only to the open one." in full each time. It is not clipped and does not overlap anything.
 - Light 1.0, default window (1024x768): one line (r2-2-light-1.0-1024-jira.png).
 - Light 1.0, narrow (800): wraps to two lines (r2-2-light-1.0-800-jira.png).
 - Dark 1.0, default: one line (r2-2-dark-1.0-1024-jira.png).
 - Dark 1.0, narrow: two lines (r2-2-dark-1.0-800-jira.png).
 - Light 2.0, default (2048x1536 physical): one line (r2-2-light-2.0-default-jira.png).
 - Light 2.0, narrow (800 logical): two lines (r2-2-light-2.0-800-jira.png).

Observations (non-blocking):
- At the default width the one-line subtitle ends a few pixels short of the scrollbar, at 1.0 and 2.0.
- At the narrow width the "Once a project is connected..." paragraph below the subtitle touches the scrollbar ("linked" loses its last letter). That paragraph is not part of this change and was not examined further.
- To fit the 2.0 default size I restarted Xvfb at 2200x1600 for that combo only.

</details>

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | A r1 | F3 MINOR: a touch `FingerPressed` on a link reaches every card, and `keyboard_focus.rs:536` drops another card's keyboard focus on it (R7 for touch) | The premise does not hold: the keyboard is given up only on `Event::Mouse(ButtonPressed(Left))` (`keyboard_focus.rs`, the rule just above line 536); line 536 sets only the control's own `pressed` flag, and only when the finger is over that control. Another card's ripple and button likewise react only to a finger over them. So a touch on a link moves no other card's keyboard, as R7 asks. Confirmed by review A r2. |
| M2 | A r1 | F4 MINOR: the inline link's 6 px focus gutter is clipped by 2 px under the expanded body's 4 px top padding when the link is the body's first row | Deferred to M4, where it can first be seen: in M2 (and M3) the link is never a Tab stop (`focusable(false)`, contract U3 staging), so no ring is drawn in the gutter, and the clipped 2 px of hit strip lie in the body's top padding, under the title. M4 shows the ring and its visual pass covers it; recorded in *Follow-ups not done*. |
| M2 | A r1 | F5 MINOR: `over_link` runs an operation over the hovered card on every mouse event and frame | Kept by design: it walks only the one card under the pointer (a few dozen nodes), frames come only while something animates or a rest timer waits (G3, one timed wake), and the rest timer needs the link state on frames (a scroll moves the link under a still pointer, R8). A per-layout cache would also have to follow the expanded body's own scroll offset, state the grid does not hold. Confirmed by review A r2. |
| M4 | A r2 | F1 MINOR (optional): `LinkFocus::of` runs on every message and clones the active project `PathBuf` and the expanded location, even with the Overview closed | Declined: two small clones per message are negligible beside the update they bracket (no measurable cost; the reviewer marks the fix optional). Snapshotting the project only when a link is shown would add a second condition to the F2 project-switch guard that review A r2 and the visual pass r2 just verified, and would force another review, visual and gate round for no user-visible gain. |
| M5 | A r1 | F1 MINOR: each setter in the `SettingsSet` chain broadcasts its own `SettingsChanged`, so a save with `jira_connection` and `jira_base_url` first echoes the old base-URL map | Not small or local: a combined `ProjectJiraChange` for connection + base URL would still leave the earlier echoes of `set_jira_enabled` and `set_github_issues_enabled` (between the two in the chain, `server.rs` ~1404–1419) carrying the old maps, so the flicker remains whenever the switch changes in the same save. The real fix is one broadcast per `SettingsSet` for every field, a change to the pre-existing pattern (features 533, 1102 and others) outside this PR; the final state is correct (reviewer). |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- ~~M4 (T031–T038): the link as the expanded body's first row has 2 px of its 6 px focus gutter clipped (review A M2 F4).~~ Fixed in M4 (top padding `max(XS, GUTTER)` for that row; test `a_link_first_in_the_expanded_body_keeps_its_whole_gutter`); the visual pass checks the ring there.
