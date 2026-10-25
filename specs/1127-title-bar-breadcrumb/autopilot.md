# Autopilot ledger — #1127 title-bar-breadcrumb

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1127 "Layout: single 44px title bar with breadcrumb and jump-to field" (body verbatim in spec.md *Input*); the user asked to start the app redesign issues in dependency order. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1127
- **Worktree branch**: claude/project-thread-iwrht0
- **Started**: 2026-10-24
- **Phase**: milestone M2
- **Next step**: M2 implemented (T022–T033 ticked) on local branch wip/1127-m2 over M1's d9c7278b; reviews, visual pass and gate next

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1357 | design (spec, plan, tasks) | merged | 1076eede8a911518582fcfcbeb95fc08c4949d58 |
| #1376 | M1 | open | pending |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T021 | full | 44 px bar: app mark, switcher, Overview button, Settings, overflow; panel at the bar's edge | #1376 | PR open, CI running |
| M2 | T022–T033 | full | `project / place` breadcrumb; switcher under the project segment | — | implementing (wip/1127-m2) |
| M3 | T034–T045 | full | Jump-to field opens the quick switcher, gives way first | — | todo |
| M4 | T046–T050 | light | Overview button leaves the bar (only after #1125 M3 merges) | — | todo |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Plan usage in the title bar (issue body) or the status bar (#1124)? | Status bar only; the title bar draws none | agent-resolved | specs/1124-bottom-status-bar/spec.md#Clarifications, FR-011; mockup canvas.json note 4 |
| D2 | spec | Can the bar land before #1125's Overview row? | Yes: Stories 1–2 first; Story 3 (Overview button leaves) waits for #1125 M3 to merge; button stays until then | agent-resolved | issue #1127 "Blocked by"; specs/1125-sidebar-state-groups/autopilot.md#Milestones (M3 todo) |
| D3 | spec | What does the breadcrumb's second segment show? | Short branch (else display name); `Overview` / `Settings` when those are shown | agent-resolved | mockup Main.dc.html / Overview.dc.html title bar (`micold-ai-ide / fix/1117`, `/ Overview`) |
| D4 | clarify | Where is the jump-to field centred? | In the free space between leading and trailing groups (FR-008) | agent-resolved | mockup Main.dc.html flex spacers |
| D5 | clarify | Jump-to field while Settings / switcher open? | Disabled and inert in Settings; press closes an open switcher (FR-009) | agent-resolved | specs/968-quick-switcher/spec.md FR-012, US1 #6; ui/toolbar.rs overview_press |
| D6 | clarify | Is the app mark a control? | No: tooltip only, not pressable, no Tab stop (FR-003, FR-013) | agent-resolved | mockup Main.dc.html (plain block) |
| D7 | clarify | Bar elevation on sidebar scroll? | Kept unchanged (FR-019) | agent-resolved | app.rs State::app_bar_elevated |
| D8 | clarify | Contrast rule for the disabled field? | FR-017 covers enabled controls; disabled uses 38% treatment | agent-resolved | specs/003-material-design-layout/contracts/design-tokens.md |
| D9 | plan | Bar figures: 43 + 1 divider, 40 dp targets, keep `anatomy::app_bar` name; 3 new shared components; US3 waits for #1125 M3 (not on origin/main 2026-10-24) | as research R1, R2, R12 | agent-resolved | research.md R1–R12; git log origin/main |
| D10 | tasks | Plan paths after #1262 moved the generic components into `crates/micold-material`? | New and reshaped generic components go in `crates/micold-material/src/`; the app icon bytes stay in the client (`shell/startup.rs` `pub(crate) APP_ICON_RGBA`), `AppMark` takes a handle and a name; plan.md and research.md (header, R9, R11) fixed | agent-resolved | git show a4f2edd2 (#1262); crates/micold-material/tests/no_domain_types.rs |
| D11 | tasks | Split US1 (24 tasks with Setup/Foundational)? | Yes, along its scenarios: M1 the bar's shape (s1 part, s7, s8), M2 the breadcrumb (s1–s6) | agent-resolved | milestones.md rule 3; tasks.md *Milestones* note |
| D12 | tasks | M1 has 21 tasks: split further (analyze C2)? | No: Setup/Foundational alone has nothing to observe; sizing reason recorded under M1 in tasks.md | agent-resolved | milestones.md rule 1; constitution Task-size gate |
| D13 | tasks | FR-013 needs a tooltip on the project segment; the contract gave it one only when shortened (analyze C1) | `Switch project`, full name on a second line when shortened; contract C2, T024, T025; FR-013 (C2) moved to M2 | agent-resolved | spec FR-013; contracts/title-bar-ui.md C2 |
| D14 | M1 | T016 makes `shell/startup.rs` own `pub(crate) APP_ICON_RGBA`, but `shell` is binary-only (`mod shell` in `main.rs`) and the bar is in the lib | The bytes move to the lib: `ui/toolbar.rs` `pub const APP_ICON_RGBA` / `APP_ICON_SIDE`, re-exported from `ui`; startup reads `micold_client::ui::APP_ICON_RGBA` (one embedding) | agent-resolved | crates/micold-client/src/main.rs `mod shell;`; src/lib.rs has no `shell` |
| D15 | M1 | A 48 dp `IconButton` target cannot stand in the 43 dp bar; how do the bar's icon actions reach `ICON_TARGET` 40? | NEW `IconButton::target(side)`; `MenuTrigger` and the Settings button pass `app_bar::ICON_TARGET` (now bound, so its "carried elsewhere" waiver in `anatomy_call_sites` is gone); an inert button keeps a named target so Settings stays 40 dp while selected | agent-resolved | research R1; tests/anatomy_call_sites.rs; icon_button.rs tests |
| D16 | M1 | The switcher must shorten (B4), but a `Button`'s glyph/label/mark `row!` gives the label the whole room and drops the unread mark (T013 failed at 640 dp) | The shared planner's `Lane` (give_way.rs) gains a hugging mode; `Button` lays its glyph, label and mark on it (label `Shorten`), `StatusBar` and `Toolbar` use the bar mode; the switcher label is an `Ellipsized`. Fixture with and without the Button change is byte-identical at default widths | agent-resolved | gates/title_bar_fits.rs; fixture diff check in the M1 unit |
| D17 | M2 | How does the project segment draw `on_surface` (contract T3) when a text `Button` draws `primary`? | The breadcrumb stands its button on `breadcrumb::host(roles)` = `Host::accent(surface, on_surface)`, so label, glyph, mark number, state layer and ripple take `on_surface` together; the contrast gate reads the same function | agent-resolved | contract T3; `composition_contrast::the_breadcrumbs_words_are_legible_on_the_title_bar` |
| D18 | M2 | How does a segment know it was shortened, for its tooltip (FR-007, C2)? | At layout: a private `Fit` widget lays the segment out unbounded, compares with its room and draws one of two renderings (full-width tooltip, or the tooltip with the full text) | agent-resolved | research R4; the view does not know the window width |
| D19 | M2 | The segment's glyph and the switcher's old leading folder icon | Drop-down `Icon::SelectChevron` trailing (`Button::trailing`); the leading `OpenProject` glyph goes (`micold-ai-ide ▾`, quickstart §B1) | agent-resolved | spec US1 s1; quickstart §B1 |
| D20 | M2 | `sibling_parity` assumed every bar action leads with its glyph and every bar panel ends at one trailing inset | The breadcrumb's action is its project segment (glyph trailing the label); the switcher panel may instead start at the segment's edge (R8) | agent-resolved | research R8; FR-005 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 8eb3f915c06f64566ad2e62c6f69da7d28f756f5:08ff39f5c466168d2ad5a9ff688e3b192d4f9461 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | 11ac16c31ec7892dbc9503a8efc59c62abc11341:7c59c3b9c7fa3e6b0a2d12520149d5778c88796e | CLEAN (1 MINOR: APP_ICON_RGBA is a rename of private ICON_RGBA; fixed, prose only) |
| Tasks | 1 | 84a84006f20dfcb2715de005af939bd8cc8f3def:605ab3e676a083538fdd7b001e0f5be80469cecb | CLEAN (3 MINOR: `app_bar.*` paths in T032/T044, T022 under Implementation, M1 Verify narrowed to its quickstart parts; fixed, prose only) |
| M1 review A (code-review, high) | 1 | 0f05699fb840f8b46965b4b2bbd40170d47bdf41:bb8a87948c85d1b1a7644e73d13adab2b78c7c74 | 10 findings (no BLOCKER): fixed 5 — Lane lays each child out once when all fit; status bar measures floors only for `Shorten` items; one shared app-mark handle and a size assert on the icon bytes; exact-match asserts in `title_bar_fits`; stale 64dp comments. Declined 5 (below) |
| M1 review B (conformance, sonnet) | 1 | 254ff9de728dd865e67ccf71634150d9b430d1ef:9fe196e8363b6caaf6e70da6fb77ddc9b8ec8227 | CLEAN (1 MINOR: duplicate decision ID D13; fixed, prose only) |
| M1 visual pass | — | — | pass: 44 px bar, order, tooltips, Settings selected, menu and switcher at y 44, dark, 640 px (specs/1127-title-bar-breadcrumb/visual-pass/m1/) |
| M1 gate (`scripts/gate.sh`) | — | d483f4ac49cd33cdf002a50560be52f66772b7b4 | pass after rebase on origin/main c4ca0754 and snapshot re-record; on the way: clippy `manual_contains` in `title_bar_fits`, AppMark showcase entry marked interactive (its tooltip is live), `session_area_stability` press-in-flight helper ignores `TerminalResized` |
| M2 review A (code-review, high) | 1 | e8c78225c8584beabccb86e975eff6a0147783ca:a485c8a6050390406b0ce134367f06fa9503f935 | CLEAN (3 MINOR, fixed: `Fit`'s separate trees noted in its doc; fixture branch starts `b-` so the two floors differ; the narrowed breadcrumb test asserts the place's exact floor) |
| M2 review A round 2 (scoped, sonnet; gate fix 81c5abab..b5961ecd) | 2 | 1f2d73363e7a0308fc840aa9c032b99f6f2d2062:7746f8ee1bbd09a531749a3737f941acee43cc95 | CHANGES: 1 MAJOR (every `hung_from` caller got the bottom inset, squeezing the sidebar ⋯ menu) — fixed, opt-in `MenuOverlay::short_of_bottom()`; 1 MINOR (duplicated placement arm) — fixed |
| M2 review A round 3 (scoped, sonnet; b5961ecd..45df7948) | 3 | 45df7948 | CLEAN, citing both fixes and both `hung_from` callers |
| M2 review B (conformance, sonnet) | 1 | e8c78225c8584beabccb86e975eff6a0147783ca:a485c8a6050390406b0ce134367f06fa9503f935 (+ A's minor fixes, uncommitted) | CLEAN (3 MINOR: Verify logs pending the scoped gate — rerun; stale `ui::switcher_hung_from` comment — fixed, prose only; `Button::tooltip` named in T028 — fixed, prose only) |
| M2 visual pass | — | b5961ecd | pass: `micold-ai-ide ▾ ● 3 / fix/1117`; panel at y 44, x 48 by pointer; Overview, full screen, Review and Settings places; 640 px place shortens first; tooltips (specs/1127-title-bar-breadcrumb/visual-pass/m2/). Keyboard open and no-project not driven (test-covered) |
| M2 scoped gate | — | 81c5abab | red, fixed: model test called `changes::update` outside the root (SC-002 gate); `known_projects_overflow` read the switcher at the trailing edge, and a hung popover lost FR-011a's bottom inset (new overlay `Anchor::Below`); `session_start_press` counted the bar's chevron |

## Declined review findings

- M1 A: `rising_above(app_bar::HEIGHT)` for the shell-instance menu (`ui/mod.rs`) — pre-existing coupling to a bar that no longer sits at the window's bottom; the panel's bottom is now 43 dp above the window edge and still clears the 28 dp status bar. Out of M1's scope; follow-up below.
- M1 A: selected `IconButton` drops the hover layer — the same rule as `Button::selected` (the selected layer except while pressed); the bar's Settings button is inert while selected.
- M1 A: `MenuTrigger` hard-codes the 40 dp target and keyboard reach — T005 asks for keyboard reach on `MenuTrigger`; it is the toolbar's trigger (its doc) and the bar is its only application caller.
- M1 A: 1.5 dp / 9.5 dp fractional offsets — research R1 chose 43 dp with 40 dp controls (1.5 dp slack) on purpose.
- M1 A: 64 px icon drawn at 24 dp may alias — spec Assumptions use the existing icon; the visual pass judges it.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- The session header's chips row is 15.6 dp with text chips and 16 dp once the pull request chip (an icon chip) appears, so the terminal below re-measures by 0.4 dp; at the 1127 body height that crosses a row boundary (`session_area_stability` now ignores the resize). Give the chips row one height.
- `ui/mod.rs` shell-instance menu `.rising_above(anatomy::app_bar::HEIGHT)` (and the showcase's twin) reads the title bar's height for a bottom bar; it should read the bar it rises from (M1 review A).
- T051 (visual pass of quickstart Part B) is in no milestone: the close unit runs it (analyze G5).
- Analyze LOWs left as is: G2 (icon contrast pairs; icons use existing app-bar roles), G4 (tooltip placement test), F3, F5. (T1 fixed with tasks review F2.)
