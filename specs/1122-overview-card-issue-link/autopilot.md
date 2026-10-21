# Autopilot ledger — #1122 overview-card-issue-link

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1122: Overview cards: link to the worktree's issue. Blocked by: #1120 (the branch line the link sits on). "Issue #N" beside the branch opens the GitHub issue in the browser; N comes from the worktree name, as the issue-title summary already does. A Jira key (MIC-87) in the name links to `<Jira base URL>/browse/<key>`, which needs a Jira base URL setting and shows no link while it is unset. A press on a link does not activate the card. No link on Default or on a worktree with no issue. Acceptance: both links open the right URL; Settings has the Jira field; card activation is unaffected; docs updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1122
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: milestone M3
- **Next step**: M3 review A r1 fixes (F1–F3) in; rebase on origin/main, full gate, push; then review A r2 (sonnet, fix diff), then PR text.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1180 | Design (spec, plan, tasks) | merged | 9ad05048 |
| #1188 | M1: link rules in core; shared `TextLink` in the showcase | merged | 0164f375 |
| #1195 | M2: the issue link on cards in the showcase | merged | c61380e1 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Link rules proven in core; showcase shows the shared `TextLink` | #1188 | merged |
| M2 | T010–T019 | full | Showcase shows cards with "Issue #1122", click opens only the link | #1195 | merged |
| M3 | T020–T030 | full | GitHub issue links on the Overview; rest on link and arrows as specified | — | in progress |
| M4 | T031–T038 | full | Issue links by keyboard (focus counts for the card, Menu key returns to link) | — | planned |
| M5 | T039–T050 | full | Jira base URL setting (wire 58) and Jira issue links | — | planned |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | A r1 | F3 MINOR: a touch `FingerPressed` on a link reaches every card, and `keyboard_focus.rs:536` drops another card's keyboard focus on it (R7 for touch) | The premise does not hold: the keyboard is given up only on `Event::Mouse(ButtonPressed(Left))` (`keyboard_focus.rs`, the rule just above line 536); line 536 sets only the control's own `pressed` flag, and only when the finger is over that control. Another card's ripple and button likewise react only to a finger over them. So a touch on a link moves no other card's keyboard, as R7 asks. Confirmed by review A r2. |
| M2 | A r1 | F4 MINOR: the inline link's 6 px focus gutter is clipped by 2 px under the expanded body's 4 px top padding when the link is the body's first row | Deferred to M4, where it can first be seen: in M2 (and M3) the link is never a Tab stop (`focusable(false)`, contract U3 staging), so no ring is drawn in the gutter, and the clipped 2 px of hit strip lie in the body's top padding, under the title. M4 shows the ring and its visual pass covers it; recorded in *Follow-ups not done*. |
| M2 | A r1 | F5 MINOR: `over_link` runs an operation over the hovered card on every mouse event and frame | Kept by design: it walks only the one card under the pointer (a few dozen nodes), frames come only while something animates or a rest timer waits (G3, one timed wake), and the rest timer needs the link state on frames (a scroll moves the link under a still pointer, R8). A per-layout cache would also have to follow the expanded body's own scroll offset, state the grid does not hold. Confirmed by review A r2. |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M4 (T031–T038): when the link is the expanded body's first row (no display name, no branch line), the body's 4 px top padding clips 2 px of the link's 6 px focus gutter (review A M2 F4). Give that row the gutter's room when the ring arrives, and check it in M4's visual pass.
