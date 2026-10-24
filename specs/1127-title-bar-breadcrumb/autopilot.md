# Autopilot ledger — #1127 title-bar-breadcrumb

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1127 "Layout: single 44px title bar with breadcrumb and jump-to field" (body verbatim in spec.md *Input*); the user asked to start the app redesign issues in dependency order. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1127
- **Worktree branch**: claude/project-thread-iwrht0
- **Started**: 2026-10-24
- **Phase**: design PR
- **Next step**: orchestrator opens the design PR from the scratchpad title/body; after merge, milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T021 | full | 44 px bar: app mark, switcher, Overview button, Settings, overflow; panel at the bar's edge | — | todo |
| M2 | T022–T033 | full | `project / place` breadcrumb; switcher under the project segment | — | todo |
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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 8eb3f915c06f64566ad2e62c6f69da7d28f756f5:08ff39f5c466168d2ad5a9ff688e3b192d4f9461 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | 11ac16c31ec7892dbc9503a8efc59c62abc11341:7c59c3b9c7fa3e6b0a2d12520149d5778c88796e | CLEAN (1 MINOR: APP_ICON_RGBA is a rename of private ICON_RGBA; fixed, prose only) |
| Tasks | 1 | 84a84006f20dfcb2715de005af939bd8cc8f3def:605ab3e676a083538fdd7b001e0f5be80469cecb | CLEAN (3 MINOR: `app_bar.*` paths in T032/T044, T022 under Implementation, M1 Verify narrowed to its quickstart parts; fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- T051 (visual pass of quickstart Part B) is in no milestone: the close unit runs it (analyze G5).
- Analyze LOWs left as is: G2 (icon contrast pairs; icons use existing app-bar roles), G4 (tooltip placement test), F3, F5. (T1 fixed with tasks review F2.)
