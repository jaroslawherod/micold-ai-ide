# Implementation Plan: Worktree Overview

**Branch**: `claude/project-thread-2lmupk` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/813-worktree-overview/spec.md`

## Summary

A project Overview in the main pane: one Material card per sidebar location, coloured, iconed and
worded by one card state (Needs action, Needs attention, Working, Done, Idle; worst wins), with
type, session and pull request counts, a task summary and progress. A top-bar button with an
attention badge and the reserved chord Ctrl+Shift+O (Cmd+Shift+O) toggle it; a card opens its
location's main session. The rules are pure core modules over signals the window already has
(905 location state, 776 main session, 040 status reading); the gaps are filled on existing paths:
the agent request's caller (client only), pull request counts and the conflict flag (same GraphQL
reading), spec-kit files read off the UI thread, and hook-fed todos and prompt carried on
`SessionSummary` (protocol 49).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14, `notify` and `serde_json` (already in the workspace); no new
crate; glyphs from the bundled `assets/fonts/MaterialSymbolsOutlined.ttf`

**Storage**: none new; Overview open flag, progress and titles are per-window memory; hook activity
is runtime-only on the live session (Principle IV)

**Testing**: `cargo test` via `mise run test-core` (core) and `mise run gate` (daemon, client state
tests in `crates/micold-client/tests/`, in-crate geometry and contrast gates, showcase, icons)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: session / request / PR change on the card within 1 s (FR-008, SC-004); task
list change within 5 s; 30 cards within 1 s (SC-007): cards are a pure per-render derivation,
O(locations × sessions)

**Constraints**: no new network polling (FR-019); file reads never on the UI thread; prompts, todos
and titles never logged or sent (FR-020); the chord sends 0 bytes to a PTY (SC-006)

**Scale/Scope**: tens of locations and sessions per project; 3 core modules, 1 wire field, 1 query
extension, 1 client feature module + 1 watcher, 3 new shared components and 3 component extensions,
2 role families, ~10 icons, 1 user-guide page + 3 doc edits

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule is in tested code written red first: `card_state`, `location_type`, `session_counts`, `order`, `attention`, `activation` (core `overview.rs`); `task_progress` parsers; `HookActivity::apply`/`current`; `status_query`/`parse_status` counts and `conflict`; `keymap::encode` chord; daemon `note_hook_activity` + overlay + MCP exclusion; client `overview::update`, card building from app state, `pr_status` counts, `Prompt::session`; geometry and contrast gates for the components. `ui/` view wiring, `shell/overview_watch.rs` subscription glue and the showcase only call those (glue exception, quickstart §B). |
| II. Multi-Session | PASS | Counts and state take every un-archived session of a location (worst wins); hook activity is per session, keyed by the hook route's session; activation opens the main session; several sessions per location is a main test case. |
| III. Worktree Integration | PASS | Read-only over the sidebar's locations (worktrees, included, orphan / invalid, Default); no worktree operation; Default keeps its project-root type. |
| IV. Local-First | PASS | No persistence; file reads are local; the only network use is the existing PR reading and a one-shot cached issue-title read through the existing `gh` lookup (R8). |
| V. Rust + iced | PASS | iced only; closed enums with derived `Ord` for precedence; pure core, render-free client reducer. |
| VI. Cross-Platform | PASS | One chord predicate with the platform modifier (`is_primary_shift_chord`); bundled glyphs; CI on three platforms; quickstart §C. |
| VII. Documentation | PASS | New `docs/user-guide/overview.md` + `docs/SUMMARY.md`; the chord in `docs/user-guide/terminal-panes.md` §Keyboard shortcuts and `worktrees-and-sessions.md`; icon rows in `docs/user-guide/icons.md`; same PR. |
| VIII. Components | PASS | New shared `OverviewCard`, `CardGrid`, `ProgressBar` with builder APIs; `Button::selected`/`mark`, `UnreadMark::icon` extend existing components; new theme roles `warning*`/`success*`; showcase entries. |

Post-design re-check: unchanged, all PASS. Known limitation (not a deviation): no accessibility
tree in iced 0.14; the badge's and card's words live in tooltips and visible text (as in 905 R9).

## Design

### D1 — Card rules (core) — R4, R6, R7, R12

New `crates/micold-core/src/overview.rs` (in `lib.rs`): `LocationType`, `CardSignals`, `PrSignal`,
`CardState`, `CardReason`, `card_state`, `location_type`, `session_counts`, `PrCounts`,
`CardModel`, `order`, `attention`, `Activation`/`activation`, as in
[data-model.md](./data-model.md). `naming.rs` gains `pub fn ticket(dir_name)`.

### D2 — Task progress (core) — R9

New `crates/micold-core/src/task_progress.rs`: `Phase`, `Ratio`, `FileProgress`, `Progress`,
`feature_dir`, `checkbox_ratio`, `ledger_phase`, `artifact_phase`, `phase`, and the bounded `read`.

### D3 — Hook activity (core + daemon + wire) — R10

- New `crates/micold-core/src/hook_activity.rs`: `HookActivity`, `Todo`, `apply`, `current`.
- `protocol/messages.rs` `SessionSummary::hook_activity`; `version.rs` 48 → 49; schema hash.
- `crates/micold-daemon/src/hooks.rs` (`classify_hook` hands the parsed body on),
  `state.rs` (`LiveSession::hook_activity`, `note_hook_activity`, `overlay_live_summaries`),
  `catalog.rs` (`None`), `mcp/tools.rs` unchanged and pinned by a test.
- Client `catalog_sync.rs` adopts it; `Session::hook_activity` ([wire.md](./contracts/wire.md)
  W1–W3).

### D4 — Pull request counts and conflict (core + client) — R6

`pull_request.rs` `FRAGMENT`, `status_query`, `parse_status` → `StatusReading`,
`PullRequestStatus::conflict`, `read_outcome`; client `features/pr_status.rs` `State::counts`,
`Outcome`/`Msg` carry them; `shell/pr_status.rs` passes them on (W5).

### D5 — Agent request caller (client) — R5

`features/agent_confirm.rs` `Prompt::session`; `shell/daemon_sync.rs` fills it (W4).

### D6 — Overview state and card building (client, render-free) — R1, R2, R8, R12, R14

New `crates/micold-client/src/features/overview.rs`: `State`, `Msg`, `update`, and
`cards(&App facts) -> Vec<CardModel>` assembling D1–D5 inputs from the sidebar entries, sessions,
`pr_status`, `agent_confirm` queue, `progress`, `titles`. `app.rs`: `terminal_focused`,
`main_area_taken`, `on_escape`, `route_key` (`KeyRouting` for `ToggleOverview`). Sidebar
`revealed: Option<SessionLocation>` in `features/sidebar.rs` (set by reveal, cleared by the next
selection).

### D7 — Keys — R3, R13

`keymap.rs`: `pub fn is_overview_chord`, `KeyOutput::ToggleOverview`, `overview_chord_label()`;
`ui/material/terminal_pane.rs` publishes and captures it; `ui/mod.rs` `subscription`: the unfocused
chord listener and, while open, the arrow-key listener (named `fn`s) ([ui.md](./contracts/ui.md)
U5).

### D8 — Theme roles, icons, components — R11

- `micold-core/src/tokens/mod.rs` (+ palette): `warning*`, `success*` roles, both schemes.
- `icons.rs`: `Overview`, `CardNeedsAction`, `CardNeedsAttention`, `CardDone`, `TypeFeature`,
  `TypeBugfix`, `TypeChore`, `TypeOther`, `TypeIncluded`, `TypeOrphan`.
- `ui/material/`: `overview_card.rs`, `card_grid.rs`, `progress_bar.rs` (new); `button.rs`
  (`selected`, `mark`), `unread_mark.rs` (`icon`), `composition_contrast.rs` (U4).

### D9 — Wiring (glue)

- `ui/overview.rs` (new): `view(state, roles)` → `CardGrid` of `OverviewCard`s from
  `overview::cards`, empty project never blank (Default always has a card).
- `ui/mod.rs` `view_with`, `main_content_key`; `ui/toolbar.rs` button and badge (U2);
  `ui/sidebar.rs` draws `revealed` with the selected look; `main.rs` routes `Message::Overview`,
  sends `overview::Msg::Closed` on `SessionMsg::Selected`, runs the card's follow-up.
- `shell/overview_watch.rs` (new) on the generalised `changes_watch::run`; `shell/issues.rs` (or a
  new `shell/overview.rs`) issues the one-shot title reads (R8).

### D10 — Showcase and docs

- `showcase/catalogue.rs` + sections: `OverviewCard` (five states and variants), `CardGrid`,
  `ProgressBar`, toggled `Button`, tinted `UnreadMark` with icon; `tests/showcase_completeness.rs`.
- `docs/user-guide/overview.md` (new; in `docs/SUMMARY.md`): opening and closing, the card, the
  five states and what puts a location in each (table from data-model), location types, counts and
  the unknown mark, progress sources and their precedence, hook-fed activity and how to get it,
  activation, keys. The chord in `terminal-panes.md` §Keyboard shortcuts and
  `worktrees-and-sessions.md`; icon rows in `icons.md` (FR-022).

## Requirement map

| FR | Where | Research |
|---|---|---|
| FR-001 | D6 `cards` over `sidebar_entries` | R7 |
| FR-002 | D1 `location_type`, D8 icons, U3 | R7 |
| FR-003 | D1 `session_counts` | — |
| FR-004 | D4, D1 `PrDisplay` | R6 |
| FR-005 | D1 `card_state`, D8 roles + icons + words | R4, R11 |
| FR-006 | D8 Idle surface, U4 contrast | R11 |
| FR-007 | D1 `order` | R4 |
| FR-008 | per-render derivation (catalog, prompts, reading); D9 watcher | R9 |
| FR-009 | D9 toolbar, D8 `Button::selected` | R11 |
| FR-010 | D6 `Toggled`; view stack | R1 |
| FR-011 | D7 | R3 |
| FR-012 | D1 `attention`, D8 `mark` | R11 |
| FR-013 | D1 `activation`, D6 follow-ups, sidebar `revealed` | R12 |
| FR-014 | D7 keys, D6 `on_escape` | R13 |
| FR-015 | D6 summary from D9 title reads | R8 |
| FR-016 | D2 | R9 |
| FR-017 | D2 `Progress` empty state, U3 | R9 |
| FR-018 | D3, D1 `CardModel::activity` | R10 |
| FR-019 | D4 same reading; D9 one-shot titles; watcher off UI thread | R6, R8, R9 |
| FR-020 | W3 tests | R10 |
| FR-021 | D8, D10 showcase | R11 |
| FR-022 | D10 docs | — |
| FR-023 | D9 `view_with`, `main.rs` close on selection | R1 |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Core unit | `micold-core/src/overview.rs` tests | `card_state` exhaustive over 905 states × agent request × `PrSignal` variants (SC-003); unknown PR never gives rows 5–7/9; `location_type` precedence; `session_counts` skips archived; `order` stable within state; `attention` worst + count; `activation` three cases | 002, 003, 005, 007, 012, 013 |
| Core unit | `micold-core/src/task_progress.rs` tests | checkbox variants, non-checkbox lines, none → `None`; ledger vocabulary + numeric prefix + unknown word; `feature_dir` precedence and several folders; `phase` precedence; `read` with missing / huge / non-UTF-8 files | 016, 017 |
| Core unit | `micold-core/src/hook_activity.rs` tests | W2 table; bounds; `current` precedence | 018 |
| Core unit | `pull_request.rs` tests | query text has `mergeable`, `oc`/`mc`; W5 table; `conflict` mapping | 004, 005, 019 |
| Core unit | `micold-core/tests/protocol_roundtrip.rs`, `tests/schema_hash.rs` | `hook_activity` round-trips; absent decodes `None`; version 49 | 018 |
| Core unit | `naming.rs` tests | `ticket` for numeric, Jira, none | 015 |
| Daemon | `micold-daemon/src/state.rs`, `hooks.rs`, `tests/hooks_receiver.rs` | a TodoWrite body sets the live session's activity and broadcasts; another session untouched; overlay copies it; `catalog.rs` gives `None`; MCP tools omit it; no log line holds the prompt | 018, 020 |
| Client state | `micold-client/tests/overview.rs` (new) | `update` toggle / close / Settings-open / project switch; `cards` from app state: one per sidebar entry incl. included, orphan, Default; prompt → Needs action via `Prompt::session`; counts unknown when disabled / unavailable / released; summary title → branch → name; progress empty state; hook activity beside task ratio; after `on_disconnected` session states kept; sidebar selection closes | 001, 003, 004, 008, 010, 013, 015–018, 023 |
| Client state | `micold-client/tests/features_pr_status.rs` | counts carried and cleared with statuses | 004 |
| Client state | `micold-client/tests/keymap.rs` | chord → `ToggleOverview` on both modifier sets; near-misses; `route_key` routes it | 011, SC-006 |
| Geometry gate | `ui/material/overview_card.rs`, `card_grid.rs`, `progress_bar.rs` tests | U4 | 006, Edge: narrow |
| Contrast gate | `ui/material/composition_contrast.rs` | U4 roles, both schemes | 005, 006, SC-005 |
| Icons | `tests/icons.rs`, `tests/icons_font.rs` | new codepoints pinned, distinct, in font | 002, 005 |
| Showcase | `tests/showcase_completeness.rs` | entries list their variants | 021 |
| Visual pass | quickstart §B (visual-pass skill) | look, live updates, keys, activation, narrow, 30 cards, disconnect, two windows | all; SC-001, 002, 005–007 |
| Platforms | CI on Linux, macOS, Windows; quickstart §C | parity | Principle VI |
| Docs | gate docs checks; review | FR-022 content | 022 |

## Project Structure

### Documentation (this feature)

```text
specs/813-worktree-overview/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── ui.md
│   └── wire.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── overview.rs                  # new: card rules
├── task_progress.rs             # new: spec-kit file parsers + bounded read
├── hook_activity.rs             # new: todo list / prompt from hook bodies
├── lib.rs, naming.rs            # + modules; + ticket()
├── pull_request.rs              # mergeable, oc/mc counts, StatusReading
├── tokens/{mod.rs,palette.rs}   # + warning*, success* roles
└── protocol/{messages,version}.rs  # SessionSummary::hook_activity; protocol 49
crates/micold-core/tests/{protocol_roundtrip.rs,schema_hash.rs}
crates/micold-daemon/src/{hooks.rs,state.rs,catalog.rs}   # + tests/hooks_receiver.rs
crates/micold-client/src/
├── features/{overview.rs (new),pr_status.rs,agent_confirm.rs,sidebar.rs}
├── app.rs, keymap.rs, catalog_sync.rs, icons.rs, main.rs
├── shell/{overview_watch.rs (new),changes_watch.rs,pr_status.rs,daemon_sync.rs,issues.rs}
├── ui/{overview.rs (new),mod.rs,toolbar.rs,sidebar.rs}
├── ui/material/{overview_card.rs,card_grid.rs,progress_bar.rs (new),button.rs,unread_mark.rs,terminal_pane.rs,composition_contrast.rs,mod.rs}
└── showcase/{catalogue.rs,sections/*}
crates/micold-client/tests/{overview.rs (new),icons.rs,icons_font.rs,showcase_completeness.rs,...}
docs/user-guide/{overview.md (new),terminal-panes.md,worktrees-and-sessions.md,icons.md}, docs/SUMMARY.md
```

**Structure Decision**: existing workspace layout; rules and parsers in `micold-core`, hook
extraction in the daemon, render-free state and card assembly in the client's `features/`,
rendering in the shared `ui/material` library, wiring in `ui/` and `main.rs`.

## Complexity Tracking

No violations.
