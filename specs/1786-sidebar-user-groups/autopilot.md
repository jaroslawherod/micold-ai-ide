# Autopilot ledger — #1786 sidebar-user-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1786 "feat: sidebar: let the user create groups"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1786
- **Worktree branch**: claude/project-thread-919twg
- **Started**: 2026-11-11
- **Phase**: close
- **Next step**: close PR gate and review done; merge it, then handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1848 | Design | merged 2026-11-12 | 553474089e559f95557125168150c3a3abfdcc95 |
| #1905 | M1 | merged 2026-11-14 | b479914eb6744a3691cd90d805b7547e24464392 |
| #1913 | M2 | merged 2026-11-15 | 6006f3e02039a34205aa3afad24a23e7a8d6b511 |
| #1946 | M3 | merged 2026-11-15 | cae50ef59af4fac194e5559b72608cc0ac814a23 |
| #1968 | M4 | merged 2026-11-15 | d1712abc888fad5ab736f08d5fb3e877bad784cf |
| #1972 | M5 | merged 2026-11-15 | 719e9f3b9cd09310ee41249f26d9603941c1f69e |
| #1977 | M6 | merged 2026-11-15 | 77ee454535b69b39d0d56c4702d2acb9f09786d6 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T044 | full | Layout core, store, create a group | #1905 | merged |
| M2 | T018–T023 | full | Rename and delete a group | #1913 | merged |
| M3 | T024–T028 | full | Move to group from the menu | #1946 | merged |
| M4 | T029–T034 | full | Drag between lists and cancel | #1968 | merged |
| M5 | T035–T042 | full | Reorder, collapse, no state grouping | #1972 | merged |
| M6 | T043, T045 | full | Showcase poses and visual pass | #1977 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |
| D2 | implement M1 | Where is the layout applied? | In `State::sidebar_list` (the drawn list), not `sidebar_entries`: the Overview's cards walk `sidebar_entries` and must not lose a collapsed group's cards | agent-resolved | crates/micold-client/src/ui/overview.rs `cards` |
| D3 | implement M1 | How are ids never reused across restarts? | A stored `next_id` counter in the layout file | agent-resolved | data-model.md § UserGroup `id` |
| D4 | implement M1 | Does "New group" also get a button in a project header? | No: since feature 1125 the sidebar has no project header and its header buttons are the chip bar's more menu; contract C6a's three entry points (more menu, empty-area menu, chord) are the contract | agent-resolved | contracts/sidebar-groups.md C6a; plan.md line 80 |
| D5 | implement M1 | `GroupHeader::new(name, on_toggle)` with no toggle message before US6? | `GroupHeader::new(name).on_toggle(m)`: no twisty until M5 adds `Msg::Toggle`, so M1 draws no dead control | agent-resolved | tasks.md T040 |
| D6 | implement M2 | T022 says the group menu opens "by the 803 chord", but no row menu has a keyboard opener | The menu opens by right click; its items are keyboard-operable like any menu. The keyboard opener lands with the header's tab stop in T040 (M5), which now names it (803 chord / Menu key on the focused header, with its test); U30 is PARTIAL | agent-resolved | tasks.md T040; test-list U30 |
| D7 | implement M3 | Is a header's count the group's members or the rows shown under the active filter? | The rows shown, with the hidden members said aloud: `n` while none is hidden, `n of m` while a tag filter hides some, `empty` only for a group that holds no member. A group is not empty because a filter hides its members, and the count never claims rows the list does not draw (C3: hidden rows are not targets, their stored places stay). A member whose worktree is gone is not counted as hidden | agent-resolved | spec.md edge cases (empty group; tag filter hides worktrees); contracts/sidebar-groups.md C3, C5; tests `a_header_counts_the_rows_shown_*` |
| D8 | implement M4 | Review A of M4 still found a MAJOR in round 3 (fixed, not re-reviewed): go on to review B and the visual pass, or hold M4? (escalation category 5) | Go on (option 1) | agent-resolved, orchestrator | review B round 1 covered the post-round-3 commits (head c4df81448) |
| D9 | implement M5 | Which keys open the group menu from a focused header, and what does a click on the header do? | The Menu key and Shift+F10, the card menu's keys (feature 972); the menu opens with the keyboard on its first enabled entry, Up/Down skip disabled entries, Enter/Space run it. A click on the header toggles it, like Enter/Space and the twisty (the header's press is the toggle, as the Overview row's press is its action) | agent-resolved | crates/micold-client/src/ui/sidebar.rs `header_menu_anchor`; tests a_keyboard_opened_group_menu_* |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 2 | spec.md | 58e1734ce000a84d1fef7957a5f76047a693cc51 | 15197570fe64dd3fe6ce468637233ff476ad4e90:58e1734ce000a84d1fef7957a5f76047a693cc51 | CLEAN |
| Plan | 2 | design | 076a350f4e2bebc369741dda330ff1265ccea2cb | fe475105b6c85398bc85de75395b5c8b0f586243:8c6d9c8dfc934231e7defc72aab8e90441e8fa1b | CLEAN |
| Tasks | 1 | tasks.md, milestones | fd659fa9 | fd659fa948c3b865c9e5891240b2f31309838abb:d9fddb2b92550718f4a637e8d771d0cbf244709d | CHANGES (1 MAJOR fixed: gate wiring and snapshot moved to M1) |
| Tasks | 2 | fix diff | 1fc59a5f | 70700fb0bd62ac2308bdaa57a5598ba424ce7882:1fc59a5f9b962824a590f71b1c41069072f626c6 | CLEAN (1 MINOR fixed, prose only) |
| Plan | 1 | design | 33647c442736a049f12d6cc8752134e390fc02f0 | 9c4e947e275c19ed0709a25e96d98d7e43160c6c:c0341c2bd14d57f5c0d8119083fd10ec6f2fb7ac | CHANGES (3 MAJOR fixed) |
| Tasks | 2 | fix diff | 7ad2a0361951595eea2d898d6746b37a1ddb0036 | 70700fb0bd62ac2308bdaa57a5598ba424ce7882:1fc59a5f9b962824a590f71b1c41069072f626c6 | CLEAN |
| Plan | 3 | design | 5d3885de780da42ac43772e0841f49a11dce4d3c | e984a4491e90ea566d4e00fe11bf0e87b4597b0f:5d3885de780da42ac43772e0841f49a11dce4d3c | CHANGES (1 MAJOR fixed: rename_dialog.rs line restored in plan.md) |
| Plan | 4 | design | 5d3885de780da42ac43772e0841f49a11dce4d3c | e984a4491e90ea566d4e00fe11bf0e87b4597b0f:5d3885de780da42ac43772e0841f49a11dce4d3c | CLEAN (1 MINOR left: text.rs not listed in plan, pre-existing) |
| M1 diff (A, code-review high) | 1 | origin/main...14c552a6a | 14c552a6a | fa0e41a7:14c552a6a | CHANGES (3 fixed: hint counted headers, chord reset an open dialog, layout file kept on forget; plus the sibling_dir cleanup; 6 declined) |
| Review B M1 | 1 | conformance | 18bc3ebc25f945817224ca706f2089e258172acd | 571a6f78f42446d00816ad36b6d4fbad107069fa:18bc3ebc25f945817224ca706f2089e258172acd | CHANGES (1 MAJOR fixed: drift-M1.md; 1 MINOR fixed: Verify line) |
| Visual pass M1 | 1 | quickstart B1 | 18bc3ebc25f945817224ca706f2089e258172acd | — | PASS (screenshots /mnt/project-files/issue-1786/m1-*.png; header has no collapse glyph yet, M5) |
| M2 diff (A, code-review high) | 1 | origin/main...HEAD | 1899fc214e2463840e7778e90ac0d3a0f6ff7164 | 70213a3cb713aa6253c12e0215a8276a1fa7d027:1899fc214e2463840e7778e90ac0d3a0f6ff7164 | CHANGES (4 fixed: stale menus survive Help, more menu and switcher; rename kept the menu under an open dialog; no-op rename wrote the store; duplicate menu code; 4 declined) |
| Review B M2 | 1 | M2 | 53565ed9eb4b8c572a8a84459f0c1a0bab55badc | b479914eb6744a3691cd90d805b7547e24464392:53565ed9eb4b8c572a8a84459f0c1a0bab55badc | CHANGES (2 MAJOR fixed: drift-M2.md, T040 names keyboard opener; 2 MINOR fixed) |
| Review B M2 | 2 | M2 | 4c3288732b05a7f614473ad3c58c82a7467a151b | 53565ed9eb4b8c572a8a84459f0c1a0bab55badc:4c3288732b05a7f614473ad3c58c82a7467a151b | CLEAN (scoped re-review of da2697f8e, 4c3288732) |
| Visual pass M2 | 1 | M2 | 53565ed9eb4b8c572a8a84459f0c1a0bab55badc | b479914eb6744a3691cd90d805b7547e24464392:53565ed9eb4b8c572a8a84459f0c1a0bab55badc | CLEAN (PASS; screenshots /mnt/project-files/issue-1786/m2-*.png; later fixes docs only) |
| Review B M3 | 1 | conformance | 6dbef3fb587ed1e6880555eb2d69a90ed0b77e12 | 6006f3e02039a34205aa3afad24a23e7a8d6b511:6dbef3fb587ed1e6880555eb2d69a90ed0b77e12 | CLEAN (3 MINOR left: Cycle 9 red shown by mutation only; T027 names features/worktree.rs, covered by drift-M3; ledger rows, now fixed) |
| Visual pass M3 | 1 | M3 menu, keyboard, D7 count, themes | 6dbef3fb587ed1e6880555eb2d69a90ed0b77e12 | 6006f3e02039a34205aa3afad24a23e7a8d6b511:6dbef3fb587ed1e6880555eb2d69a90ed0b77e12 | PASS (screenshots /mnt/project-files/issue-1786/m3-*.png) |
| M3 diff (A, code-review high) | 1 | origin/main...HEAD | 23a411d07f6a30308c2923efb6e0cee3278f1fc4 | 2687a887a67c6a031fda6a8fb6897ee53fe046fd:2f9d3665853d6d3d41fa9ac8d8b5135f74db9cfd | CHANGES (5 fixed: header counted gone rows as hidden, choosing the row's own list reordered it, New group over an open dialog closed it, listed ignored filters, adopt rule copied twice; 3 declined) |
| M4 diff (A, code-review high) | 1 | M4 | 725770af9eb94bbe648b45bbf7e0a3938b723b5e | 1bf5b31241892b3fb8b91b3270b05b4d4c07bb2a:8c55c9c76f0bd0c94795784693b971bc04e08a4e | CHANGES (5 fixed: marker drawn on the next list's row, highlight striped across session rows, click fired on release after a drag, Escape reached an open surface first, tinted's doc moved; 5 declined) |
| M4 diff (A, code-review high) | 2 | M4 | 895410de40095a6edf9cf5566ffaa0f2160319e1 | fix diff 8c55c9c76..725770af9 | CHANGES (2 fixed: the Escape key press closed a surface before the drag, the dragged row lost its button and padding mid-drag; 2 declined) |
| M4 diff (A, code-review high) | 3 | M4 | c8a3831ba91c795c728a0034a1e43af3f6f0195f | fix diff 725770af9..895410de4 | CHANGES (round limit: 3 fixed, not re-reviewed: inert button kept a stale pressed state, a surface refusing Escape dropped the drag cancel, the cancel rule written twice; 2 declined). Escalated, category 5 |
| Review B M4 | 1 | M4 | c4df81448814ab77b73d9f9dd81549cc4ae24b2d | 7b3bec209a63756801475cb1b7759052e7c44e4e:c4df81448814ab77b73d9f9dd81549cc4ae24b2d | CHANGES (2 MAJOR, 1 MINOR, all fixed in 888b042f1: a release after Escape or a focus loss mid-drag selected the dragged row; run group rows and headers were never held; Moved(None) left from stale) |
| Visual pass M4 | 1 | M4 | c4df81448814ab77b73d9f9dd81549cc4ae24b2d | 7b3bec209a63756801475cb1b7759052e7c44e4e:c4df81448814ab77b73d9f9dd81549cc4ae24b2d | CLEAN (PASS, all six checks; screenshots /mnt/project-files/issue-1786/m4-*.png) |
| Review B M4 | 2 | M4 | 2f2bcd226fa68b3161cfad651b32c77c6dbe4f5d | db4fe07eeef99d8f15b6953a5d3d8bb01ae3a136:2f2bcd226fa68b3161cfad651b32c77c6dbe4f5d | CLEAN (scoped to fix diff c4df81448..2f2bcd226; 1 MINOR left: held survives a blur released outside the window until the next press) |
| M5 diff (A, inline review high) | 1 | M5 | 5cfbd81007b825218f9669e87bd41e8fac21d5b9 | 8035a9b69f57deb6769c9ecfa55d121303c1965e:5cfbd81007b825218f9669e87bd41e8fac21d5b9 | MINOR-ONLY (2 MINOR, recorded: Move up/down under a tag filter can swap with a hidden row; a reveal-opened group is saved with the next message) |
| M5 diff (A, inline review high) | 2 | M5 | 6a0da7a25b8287eb4cab652bd2468d5a70720abd | b5792d9ed192604e2343262ab4fcc5fe9ffa1b4d:6a0da7a25b8287eb4cab652bd2468d5a70720abd | CLEAN (scoped to the gate-fix diff: shared menu_key map, held pinned, snapshot regenerated; not counted) |
| Review B M5 | 1 | conformance | 1c3380ec1c2caaf13d6fce8a5a52e8812a507099 | d1712abc888fad5ab736f08d5fb3e877bad784cf:1c3380ec1c2caaf13d6fce8a5a52e8812a507099 | CHANGES (1 MAJOR fixed by the orchestrator: three UNPLANNED test paths added to drift-M5.md, plan-drift now clean; 2 MINOR: spec-dir files exempt, held follow-up already closed) |
| Visual pass M5 | 1 | quickstart B4–6, header keyboard, themes | 1c3380ec1c2caaf13d6fce8a5a52e8812a507099 | d1712abc888fad5ab736f08d5fb3e877bad784cf:1c3380ec1c2caaf13d6fce8a5a52e8812a507099 | PASS (no FAIL; card-state step not exercised without live sessions, covered by T041 test; keyboard checks light only; screenshots /mnt/project-files/issue-1786/m5-*.png) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | A stored `collapsed: true` hides rows under a header with no twisty | M1 never writes `collapsed: true` and M5 (same release) adds the toggle; no released build reads a file this build cannot open |
| M1 | A r1 | Header says "empty" while a filter hides every member | No group can hold a row before M3 (Move to group); revisit the count's meaning there (follow-up below) |
| M1 | A r1 | Empty-area menu does not displace the session menus, more menu or help menu | It displaces exactly what `WorktreeContextMenu` does, the rule the row menus follow; pinned by `popover_displacement.rs` |
| M1 | A r1 | `NamingDraft` does not record its project | The project cannot change while the modal dialog is up through the UI; M2 reworks the draft's targets |
| M2 | A r1 | Menu is right-click only; U30/T022 claim the 803 chord | No row menu has a keyboard opener in the codebase; the header tab stop is T040 (M5). U30 marked PARTIAL, decision D6 |
| M2 | A r1 | `GroupMenu` holds only the group id; a project switch with the menu open could act on another project's group | The runs and worktree menus keep ids the same way; an unknown id is refused, and a menu is closed by every dialog and popover |
| M2 | A r1 | `OnRightPress` is `'static` | The sidebar's rows are `'static`; a lifetime on `GroupHeader` buys nothing here |
| M2 | A r1 | Rename `GroupContextMenu` to `UserGroupContextMenu` | The task names it `GroupContextMenu`; every registry use is fully qualified and its id is `user_group_menu` |
| M1 | A r1 | `refusal()` restates `LayoutError`'s Display | The dialog's words are contract C2's (quoted name); the core's Display is for logs |
| M1 | A r1 | `apply_layout` clones per `sidebar_list()`; layout I/O on the UI thread | Small data (a handful of groups); the write happens only after a group operation, as `save_runs` does. Not a measured cost |
| M3 | A r1 | Menu with many groups is taller than the window and never scrolls; long names overflow; a group named "Ungrouped" reads like the Ungrouped entry | Every row menu is flat and clamped the same way; scrolling or truncating menus is not this feature's scope, and the desktop app is being sunset |
| M3 | A r1 | Moving an unheld row to Ungrouped does nothing | An unheld row is already ungrouped, in stable order; saving it would pin it to the front. Consistent with choosing the list a row is already in |
| M3 | A r1 | Header-fits gate only exercises `hidden: 0` | The gate's `group_header_row` pose is the showcase's; its `n of m` pose belongs to M6 (T043). `count_words` is unit-tested |
| M4 | A r1 | Highlight and marker re-wrap a row, so iced rebuilds that row's widget state (hover, pressed button, remembered pointer) | For the dragged row this is now intended (it drops its press, see the fix for the click-on-release finding); other rows lose only hover and ripple state, which the next pointer move restores. A fixed wrapper on every row would add a layout level above every row and re-point the layout snapshot and the row-path gates |
| M4 | A r1 | With the terminal focused, Escape cannot cancel a drag | Feature 006 FR-009: the focused terminal owns every key, Escape included. The drag still ends on any release or when the window loses focus |
| M4 | A r1 | Each pointer move rebuilds `sidebar_list` for the hit test | The view rebuilds the same list on every frame anyway; a handful of rows, not measured as a cost (as M1's `apply_layout` finding) |
| M4 | A r1 | A wheel scroll during a drag leaves `over` naming the row under the pointer before the scroll | MINOR: the next pointer move corrects it; the drop lands where the marker was drawn, which is what the user saw |
| M4 | A r1 | `entry_spans` repeats `scroll_target`'s row-top arithmetic | MINOR: `entry_spans` walks entries with each one's row count; `scroll_target` asks one row's top. Both read the shared `ROW_GAP` (made public for this) |
| M4 | A r2 | The drop highlight overwrites the card-state tint of the target list's session rows | Transient, only while the drag is over that list; each row's state glyph and tooltip word still say its state (feature 1718 gives colour as the secondary channel) |
| M4 | A r2 | `drop_highlight`'s parallel start list is derived state that is easy to get wrong | MINOR: one push per entry at the top of the loop, before any of its rows; covered by the highlight test |
| M4 | A r3 | With the terminal focused, Escape cannot cancel a drag (again, from `ui::subscription`'s early return) | As r1: feature 006 FR-009 gives the focused terminal every key; the drag ends on any release or focus loss |
| M4 | A r3 | Altitude: swallow the stray `Selected` in a reducer instead of at the widget | That reducer guard would make `sidebar_drag` name the session feature (feature_registration_cost); the widget-level `ContextArea::hold_release` keeps the button and its state and is unit-tested |

## Handover

M6 built (fea4fff), review B clean, visual pass light only. Awaiting CI and merge, then close-out.

M5 merged as #1972 (squash 719e9f3b9). Review A inline MINOR-only; orchestrator-run review B r1 found 1 MAJOR (three UNPLANNED test paths missing from drift-M5.md, fixed, plan-drift clean); visual pass PASS (/mnt/project-files/issue-1786/m5-*.png; keyboard checks light only, card-state step covered by the T041 test). CI's first run failed only on a corrupt llvm-cov profile after all 13154 tests passed; green after merging main. Next: M6 (T043, T045: showcase poses and visual pass) in a fresh thread; same cloud setup (orchestrator runs review B and visual pass; units write drift-M6.md with every UNPLANNED path, tests included, and cycle-log; no `crate::features::<other>::` calls).

## Open escalation

None.

## Follow-ups not done


- Menu keyboard focus (M4 visual pass, checked in M5): the group menu now has its own keyboard model (`GroupMenu::highlighted`, Up/Down/Enter/Space through a key listener, opened by keyboard on its first entry; tests a_keyboard_opened_group_menu_*, a_pointer_opened_group_menu_*). The worktree-row and run-group menus still have none: a pointer-opened row menu shows no highlight for Down and Return reaches the widget under it. App-wide (no sidebar row menu has a keyboard model; the Overview card menu is the keyboard path, feature 972), so not fixed here; the M5 visual pass should confirm the group menu.
- M5 review A MINOR: under a tag filter, Move up/Move down step within the stored list, so they can swap with a hidden row and show no change.
- M5 review A MINOR: a reveal that opens a collapsed group marks the layout unsaved after the shell's save step of that message, so it is written with the next message.
- Seen in the M2 visual pass, not isolated: after Ctrl+Shift+G (New group) the More menu stayed open behind the naming dialog and after Create.

- TDD audit follow-ups left open at close (tdd/verification.md): F3/T049 (empty-area menu has no recorded red), F5/T051 (review-fix tests have no red run), F8/T053 (one eager view test), F11/T055 (duplicated test fixtures), F14/T056 (dark-theme visual pass, with the M6 one below).
- Layout save failure (close review, MINOR): `after_update` empties `layouts_unsaved` before saving, so a failed save is not retried until the layout changes again; kept in memory only, as R9 says.
- M6 visual pass (T045): light theme only, the theme toggle ignored clicks under Xvfb; dark-theme visual check of the M6 poses not done.

M6 unit: T043 done (drop highlight pose; drop marker pose already shipped in M1). Review A inline, no findings. Scoped gate green; full gate not run (disk full, nextest missing; CI validates). T045 and review B left to the orchestrator.
