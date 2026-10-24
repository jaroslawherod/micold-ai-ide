# Quickstart: Bottom Status Bar

**Feature**: [spec.md](spec.md) | [contract](contracts/status-bar-ui.md) |
[data model](data-model.md)

## Part A: automated

1. `mise run test-core` — the new `anatomy::status_bar::HEIGHT` figure (`micold-core/tests/tokens_anatomy.rs`); `tests/anatomy_call_sites.rs` then requires `material/status_bar.rs` to reference it.
2. `mise run gate` — client unit tests (`ui::status_bar` model, `material::status_bar` layout),
   integration tests and the geometry gates below.
3. After an intended layout change only:
   `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`, then review the
   fixture diff: every state gains a 28 dp bar at the bottom and the body loses 28 dp of height;
   nothing else moves except where a bring-up row or `Disconnected` banner was removed.

### Test layers

| Requirement | Layer |
|---|---|
| FR-004, FR-005, FR-006, FR-007, FR-008, FR-011 (wording, show/hide, location, chords, usage rules) | client unit: `ui/status_bar.rs` `model()` per state (render-free) |
| FR-007 + US4 s2/s3/s5 (focused pane, body branch) | client unit on `model()` with `session.active` / `overview.open` / `changes.open` set; `src/main_tests.rs` for focus moving between two panes through `update` |
| FR-012, FR-014, B1–B5 (height, give-way order, no press) | client unit on `StatusBar` layout (`material/status_bar.rs` tests via `test_support`), NEW geometry gate `tests/gates/status_bar_fits.rs` at 640 dp with long remote name, download detail and usage present |
| FR-001, FR-002, FR-003, FR-010, SC-001, SC-002, SC-003 | layout snapshot (`tests/layout_snapshot.rs`): NEW covered states `status-bar-steady` (connected, sandbox running, session active), `status-bar-bring-up` (Acquiring 42 %), `status-bar-warnings` (ProjectBusy + Failed sandbox: both banners and warning items), `status-bar-overview-full-screen`, `status-bar-settings`, `status-bar-changes`; anchors on the bar and on the body's top edge; existing states re-recorded |
| SC-005 | `tests/layout_text_overflow.rs` over all covered states incl. the new ones and a 640 dp variant |
| FR-009, SC-004 (7 action banners still drawn) | `tests/banner_is_not_a_snackbar.rs` extended: each of the seven conditions yields a banner with its action label; `Disconnected` yields none |
| FR-013 (under overlays, no focus) | existing `tests/overlay_stacking.rs` pattern: a covered state with a menu open over the bar; `tests/terminal_focus.rs`-style check that the bar adds no focusable widget |
| FR-015 (docs) | docs check in CI (`docs/SUMMARY.md` links); user-guide text reviewed in the PR |
| Bring-up row removed | `tests/sandbox_progress.rs`: the "indicator occupies nothing" test is replaced by "the window column has no row between toolbar and body during Acquiring" |
| Idle frames | existing `tests/idle_requests_no_frames.rs`: the bar at rest (no bring-up) requests no frames |
| Light/dark contrast | `material/composition_contrast.rs` checks for warning text and dots on `surface_container`, both schemes |

## Part B: visual pass (no human; `visual-pass` skill)

Prerequisite: `mise run run` (or `cargo run -p micold-client`) with a project open; screenshots
with `mise run screenshot <file>`. Record them under `specs/1124-bottom-status-bar/visual/`.

1. Steady state, session view: nothing between app bar and body; the bar reads a green dot,
   "Daemon connected · local", "Sandbox ready" (or "Sandbox off"), "CLI: Claude Code", plan usage
   when a reading is held, and on the right "Ctrl+Shift+O Overview", "Ctrl+Shift+P Jump".
2. Open Overview, full screen, Changes, Settings, close the project: the bar stays at the bottom at
   the same height; the CLI item is gone outside the session view.
3. Split two panes with different CLIs; click each: the CLI item follows focus.
4. Restart the sandbox from Settings: the sandbox item shows the stage and busy ring, hover shows
   the runtime line; the body's top edge does not move.
5. Stop the session service: the connection item turns red "Daemon disconnected · reconnecting…",
   no banner appears; restart it: green again.
6. Open the same project in a second window: the banner with **Take over** appears and the item
   reads "Read-only · another window" with an amber dot.
7. Resize to the minimum width (640 dp): key hints vanish first, then plan usage, then texts end in
   "…"; nothing overlaps.
8. Open the ⋮ menu and the quick switcher: both draw over the bar. Repeat 1 in the dark scheme.
