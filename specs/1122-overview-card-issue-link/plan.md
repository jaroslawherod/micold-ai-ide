# Implementation Plan: Overview cards, a link to the worktree's issue

**Branch**: `claude/project-thread-7041o6` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1122-overview-card-issue-link/spec.md`

## Summary

A card whose worktree name carries a ticket shows "Issue #N" or "Issue KEY" on its branch line; a
click, or Enter on the focused link, opens the issue through feature 031's opener without touching
the card. The rules (ticket form, FR-001's conditions, label, address, the Jira base URL's
validation, the link's rest and focus effects on expansion) are render-free core code:
`micold_core::issue_link` (new), `jira::JiraBaseUrl` (new) and additions to 1121's
`overview_expansion::Expansion`. The Jira base URL is one service-owned setting beside
`jira_enabled` (wire version 57 → 58), shown as a field in Settings → Jira. The project's GitHub
repository comes from the existing `RemoteList` read, now made on every Overview open. The link is a
new shared `TextLink`; the card's control chain (ripple, keyboard focus, button) moves into a new
cdk `ControlOverlay` above the face, so a press, release or key the link takes never reaches the
card. `CardGrid` keeps a link press from other cards and times rests on a link apart from rests on
the card. Arrows step between cards only. Design decisions: [research.md](research.md) R1–R12.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`advanced::Widget`, `mouse::Cursor::levitate`,
`widget::Operation`); existing `serde` and the protocol codec; feature 031's
`shell::link_opener::LinkOpener`. No new crate.

**Storage**: `jira_base_url` in the service's `settings.json`, service-owned (R3). The link and the
repository lookup are in-memory only.

**Testing**: core unit tests (`mise run test-core`): `issue_link` K1–K3, `JiraBaseUrl` K4,
`Expansion` K5, settings and protocol round trips, `schema_hash`. Client in-crate tests: `TextLink`
U1, `ControlOverlay` U2, `OverviewCard` U3, `step_card` U4, `CardGrid` U5; reducer tests U6;
`composition_contrast.rs`; `tests/layout_snapshot.rs` with a new covered state; daemon
`tests/jira_base_url_setting.rs`. Visual pass: [quickstart.md](quickstart.md) §B. Whole gate:
`scripts/gate.sh`.

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: link changes on every window within 1 s of a settings change (SC-005): the
service's existing settings broadcast; the card is derived per render. The grid's link-bounds
lookup runs only on pointer events, over one card's subtree.

**Constraints**: no network request to show a link and none but the OS opener to follow one
(FR-014); no clipped link at 280/380/480 px (FR-003); no card activation from a link (SC-003);
root `Message` vocabulary unchanged (`root_vocabulary_is_cross_cutting`); no new `motion::wake_at`
caller (`idle_requests_no_frames.rs`); no literal `from_millis(N)` (`motion_tokens`); test paths
built with the platform separator; new state writes classified in `tests/support/state_scan.rs`.

**Scale/Scope**: core: 1 new module, `jira.rs` (+`JiraBaseUrl`, shared authority helper),
`overview.rs` (`CardModel.issue_link`), `overview_expansion.rs` (3 events, focus part), settings and
protocol fields; daemon: catalog, state, server (copy of the `jira_enabled` path); client: 2 new
widgets (`TextLink`, `ControlOverlay`), `TakesTheKeyboard` builder, `OverviewCard`, `CardGrid`,
`features::overview`, `features::settings`, `ui/settings/jira.rs`, `ui/overview.rs`, `ui/mod.rs`
arrow listener, `main.rs`, `shell/issues.rs`, `shell/persist.rs`, `shell/daemon_sync.rs`,
`shell/startup.rs`, showcase, 1 covered state; docs: 3 pages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule is a red-first core test (K1–K5) or a red-first client widget/reducer test (U1–U6); the wire and settings fields red-first in the round-trip tests and the daemon test. Only `main.rs` task wiring (`IssueLinkActivated` → `perform`, `CardStepped` → `step_card`, focus return) is glue, checked by quickstart §B. |
| II. Multi-Session | PASS | No session state; the link is per location, whatever its sessions; opening it touches none. The base URL is app-wide and service-owned, the same in every window (FR-012). |
| III. Worktree Integration | PASS | Read-only use of the worktree's directory name; `RemoteList` is the existing worktree-aware remote read; no VCS write. |
| IV. Local-First | PASS | Showing a link reads only local state; following it asks the OS to open an address the user clicked (explicit action); nothing is sent to GitHub or Jira by the app; addresses are not logged (FR-014). |
| V. Rust + iced | PASS | iced only. `IssueRef` makes "a link that is both or neither" unrepresentable; `JiraBaseUrl` can only hold a valid address; `RepoLookup` separates "not read" from "no GitHub repository". |
| VI. Cross-Platform | PASS | The opener is feature 031's per-platform `LinkOpener`; core has no OS branch; layout in logical px and the bundled font. CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/overview.md`, `docs/user-guide/settings.md`, `docs/daemon.md` in the same PR; covered state `overview-issue-link` (FR-017). |
| VIII. Components | PASS | `TextLink` is a new shared component with a builder ending in `.into()`, in the showcase; `ControlOverlay` a cdk wrapper; `TakesTheKeyboard`, `OverviewCard`, `CardGrid` extended by builders; colours from `Roles` (`primary`, `secondary`), contrast tested (FR-015). |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### Core (`crates/micold-core`)

- **`src/issue_link.rs`** (new; `pub mod issue_link;` in `lib.rs`): `IssueRef`, `issue_ref`,
  `IssueSources`, `IssueLink`, `issue_link` — contract [issue-link.md](contracts/issue-link.md)
  K1–K3 (R1).
- **`src/jira.rs`**: `JiraBaseUrl`, `JiraField::BaseUrl`, `BASE_URL_REFUSED`; the authority/host
  checks of `SiteOrigin::parse` moved to one private helper both use — K4 (R3).
- **`src/overview.rs`**: `CardModel.issue_link: Option<IssueLink>`.
- **`src/overview_expansion.rs`**: `Event::{LinkRested, LinkFocusGained, LinkFocusLost}`, the focus
  part and the pairing rule — K5 (R8, R9); [data-model.md](data-model.md) *Expansion*.
- **`src/settings.rs`**, **`src/protocol/messages.rs`**, **`src/protocol/version.rs`**,
  **`src/kube/fake_host.rs`**: `jira_base_url` — [wire.md](contracts/wire.md) W1–W3.

### Daemon (`crates/micold-daemon`)

`src/catalog.rs`, `src/state.rs`, `src/server.rs`: store, persist and broadcast `jira_base_url`
beside `jira_enabled` (W2); `tests/jira_base_url_setting.rs` (new).

### Client (`crates/micold-client`)

- **`src/ui/material/text_link.rs`** (new) and **`keyboard_focus.rs`** (`.press_takes_focus`): U1
  (R5). Exported from `ui/material/mod.rs`; showcase catalogue entry.
- **`src/ui/cdk/control_overlay.rs`** (new): U2 (R6).
- **`src/ui/material/overview_card.rs`**: the link on the branch line (R11), the control chain in
  `ControlOverlay`, builders `on_issue_link`, `link_id`, `on_link_keyboard_focus`, `on_link_rest` —
  U3.
- **`src/ui/material/card_grid.rs`**: link press routing (R7), `(card, on_link)` rest target (R8) —
  U5.
- **`src/features/overview.rs`**: `State.repo: RepoLookup`, `Msg::{RepoRead, IssueLinkActivated,
  CardStepped, LinkMenuRequested}`, `MenuOrigin::KeyboardFromLink` (handled as `Keyboard`);
  `ProjectSwitched` clears `repo` (R2, R9, R10).
- **`src/features/settings.rs`**: `State.jira_base_url`, `JiraDraft.base_url`, validation and the
  `SettingsSet` value; **`src/features/window.rs`**: `FieldId::SettingsJiraBaseUrl` (R4).
- **`src/ui/settings/jira.rs`**: the **Jira base URL** field and note; `SETTINGS` entry (R4).
- **`src/ui/overview.rs`**: `card` sets `issue_link`; each card gets the link builders, `link_id`
  = `card_link_id(location)` (new); `step_card` (U4); `issue_link_for(state, location)`.
- **`src/ui/mod.rs`**: the Overview arrow listener sends `Msg::CardStepped` (R9).
- **`src/main.rs`**: `IssueLinkActivated` → `shell::links::perform`; `CardStepped` → `step_card` then
  `scroll_focused_into_view`; `LinkMenuRequested` → the menu anchored on the link with
  `KeyboardFromLink`; `card_focus_return` focuses the link for `KeyboardFromLink` (R9, R10).
- **`src/shell/issues.rs`**: `read_repo` (new), called by `on_overview_opened` and by
  `daemon_sync.rs`'s `Attached` arm while the Overview is open and `repo` is `Unknown`;
  `on_title_remotes` records `RepoRead` before the switch gate (R2).
- **`src/shell/persist.rs`**, **`src/shell/daemon_sync.rs`**, **`src/shell/startup.rs`**: the
  `jira_base_url` lines beside `jira_enabled` (found with `grep -n jira_enabled`).
- **`src/showcase/sections/surfaces.rs`**: a card with a link, collapsed and expanded (FR-015).
- **`tests/support/covered_states.rs`**, **`tests/fixtures/layout_snapshot.txt`**:
  `overview-issue-link` (new); `overview-open` anchors one level deeper (`ControlOverlay`), re-blessed
  once as an intended change (R12).
- **`tests/support/state_scan.rs`**: classify the writes to `overview.repo` and the new
  `Expansion` fields.

### Docs

`docs/user-guide/overview.md`: a section *Opening a worktree's issue* (what the link shows, when,
pointer and keyboard), the *Keyboard* list (Tab to the link, arrows skip it).
`docs/user-guide/settings.md`: the Jira section's **Jira base URL** field. `docs/daemon.md`:
version 58 and the new service-owned setting.

## Requirement map

| FR | Where | Test layer |
|---|---|---|
| FR-001 when a link shows; switches | `issue_link` K2; `ui::overview::card` sources; `RepoLookup`; re-read on `Attached` (R2) | core unit; client reducer (repo read, project switch); client switch-with-Overview-open test (U6); covered state |
| FR-002 label | `issue_link` K2 | core unit |
| FR-003 placement, never clipped | `OverviewCard` R11 | client U3 at 280/380/480 both forms; layout snapshot gates |
| FR-004 address | `issue_link` K2–K3, `JiraBaseUrl` | core unit; quickstart §B3 |
| FR-005 click and keys open the link, not the card | `TextLink` U1, `ControlOverlay` U2, `CardGrid` U5 | client U1–U3, U5 |
| FR-006 rest of the card unchanged; secondary click | `ControlOverlay` routing, `ContextArea` outermost | client U3 (existing card tests stay green) |
| FR-007 rest on the link | `CardGrid` U5, `Expansion` K5 | core unit; client U5 |
| FR-008 Tab stop, arrows, focus counts for the card | traversal order U2, `step_card` U4, K5 | core unit; client U3, U4 |
| FR-009 menu key, Escape on the link | `Msg::LinkMenuRequested`, `MenuOrigin::KeyboardFromLink`, `card_focus_return`; `Expansion` hold across the menu (K5); `on_escape` unchanged | core unit (K5); client `main_tests.rs` (both origins) and reducer test; quickstart §B6 |
| FR-010 settings field, default, stored | `Settings`, `JiraDraft`, `ui/settings/jira.rs` | `settings_roundtrip.rs`; `settings_sections.rs`; §B2 |
| FR-011 validation | `JiraBaseUrl::parse` K4 | core unit; client draft test (field marked, nothing sent) |
| FR-012 every window, no restart | service-owned field W2; per-render cards | daemon `jira_base_url_setting.rs`; client `adopt_daemon_settings` test |
| FR-013 opener failure | `shell::links::perform` → `LinkOpenFinished` | existing 031 tests; §B9 |
| FR-014 no network, not logged | `RemoteList` only; `IssueLink` `Debug` | core unit (Debug); quickstart §A grep |
| FR-015 components, theme, contrast, showcase | `TextLink`, `composition_contrast.rs` | `showcase_completeness`, `material_builder_api`, contrast test; §B1 |
| FR-016 rules render-free | `issue_link`, `jira`, `overview_expansion` | `mise run test-core` |
| FR-017 guide, snapshot | docs; `overview-issue-link` | docs gate; `layout_snapshot` |

## Delivery order

Cut into milestones by the tasks unit; the intended order, each step observable:

1. Core rules: `issue_link`, `JiraBaseUrl` (K1–K4).
2. Setting end to end: settings, wire 58, daemon, client mirror, Settings field (W1–W3, R4).
3. `TextLink`, `ControlOverlay`, the card with its link, the grid's press routing; showcase (U1–U3,
   U5 press); links shown on the Overview with click to open, repository read (R2, R10).
4. Rest and keyboard: `Expansion` K5, grid rest target, `step_card`, link focus, menu return
   (U4, U5 rest, R9); covered state; user guide.

## Project Structure

### Documentation (this feature)

```text
specs/1122-overview-card-issue-link/
├── spec.md
├── plan.md              # this file
├── research.md          # R1–R12
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── issue-link.md    # core: K1–K5
│   ├── card-link.md     # client: U1–U6
│   └── wire.md          # W1–W3, settings file
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── lib.rs                     # + pub mod issue_link;
├── issue_link.rs              # new
├── jira.rs                    # JiraBaseUrl, shared authority helper
├── overview.rs                # CardModel.issue_link
├── overview_expansion.rs      # link events, focus part
├── settings.rs                # jira_base_url
├── kube/fake_host.rs          # DaemonSettings literal
└── protocol/{messages.rs,version.rs}
crates/micold-daemon/src/{catalog.rs,state.rs,server.rs}
crates/micold-daemon/tests/jira_base_url_setting.rs   # new
crates/micold-client/src/
├── main.rs
├── features/{overview.rs,settings.rs,window.rs}
├── shell/{issues.rs,persist.rs,daemon_sync.rs,startup.rs}
├── ui/{mod.rs,overview.rs}
├── ui/settings/jira.rs
├── ui/cdk/control_overlay.rs  # new
├── ui/material/
│   ├── text_link.rs           # new
│   ├── keyboard_focus.rs      # press_takes_focus
│   ├── overview_card.rs
│   ├── card_grid.rs
│   └── composition_contrast.rs
└── showcase/sections/surfaces.rs
crates/micold-client/tests/
├── support/{covered_states.rs,state_scan.rs}
└── fixtures/layout_snapshot.txt
docs/user-guide/{overview.md,settings.md}
docs/daemon.md
```

**Structure Decision**: existing workspace layout: rules in `micold-core`, the setting on the
service path every switch uses, rendering in `micold-client`'s `ui/material` and `ui/cdk`, wiring in
`features/overview.rs`, `ui/overview.rs`, `main.rs`, `shell/`.

## Complexity Tracking

None.
