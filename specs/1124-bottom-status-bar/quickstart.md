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
| FR-001, FR-002, FR-003, FR-010, SC-001, SC-002, SC-003 | layout snapshot (`tests/layout_snapshot.rs`): NEW covered states `status-bar-steady` (connected, sandbox running, session active), `status-bar-bring-up` (Acquiring 42 %), `status-bar-warnings` (ProjectBusy + Failed sandbox: both banners and warning items), `status-bar-overview-full-screen`, `status-bar-settings`, `status-bar-changes` and the others tasks.md T008, T017, T022, T034 list (12 in all); anchors on the bar and on the body's top edge; existing states re-recorded |
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

## Results (close unit, 2026-10-24)

**Part A.** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test -p micold-client --all-targets` and `cargo test -p micold-core --all-targets` were run
in the worktree. The orchestrator runs `mise run gate` before pushing. The first client run found
one red, `feature_registration_cost::only_the_root_drives_a_feature` (an M4 test called
`changes::update` directly). It was fixed in the close (tasks T040), and the client suite was
re-run green. Core: 4 uid-0 permission tests fail when run as root, as in 943's audit; they are not
this feature. The TDD audit is in [tdd/verification.md](tdd/verification.md).

**Part B (T039).** The real client (`micold-ai-ide` and `micold-daemon`, pinned from one build of
`m4-local`) ran on a private Xvfb display with `ICED_BACKEND=tiny-skia`, a private
`XDG_DATA_HOME`/`HOME` and a seeded scratch git project. It was driven with `xdotool`, with ~12 s
waits per frame. This is not a real display: it shows placement, words and colour, but not GPU
rendering. Evidence is in `visual/t039-*.png`; earlier M1 evidence is in `visual/m1-*.png`.

| § | Check | Result |
|---|---|---|
| 1 | Steady state, session view: nothing between app bar and body; green dot, "Daemon connected · local", "Sandbox off", "CLI: Claude Code", hints on the right | pass (`t039-1-session-light`, row 2 of `t039-2-bars-light`). Plan usage could not be shown live: a reading needs a signed-in Claude session. It is covered by `status-bar-usage` in the snapshot and overflow gate, and by `toolbar::the_app_bar_draws_no_usage_indicator` |
| 2 | Project surface, Overview, full screen, Settings, Changes: the bar stays at the same height; CLI item only in the session view | pass (`t039-2-bars-light` rows 1, 3–5; `t039-3-bars-dark` row 2 = Changes) |
| 3 | Two panes with different CLIs, focus follows | not run live: only Claude Code is installed here. Covered by `main_tests::panes::moving_pane_focus_changes_the_status_bars_cli` (MU4 caught) |
| 4 | Sandbox bring-up: stage and busy ring, the body does not move | not reachable live: with no container runtime the bring-up failed within 1.5 s (every frame from launch already read "Sandbox failed"). The busy ring is in `m1-1-dark` (showcase), and the unmoved body in `sandbox_progress` (MU2 caught). The failed state was shown live: the banner with **Run without it for now**, the amber "Sandbox failed" item (`t039-5-sandbox-failed-dark`), and the failure reason in the item's tooltip (`t039-5-failed-tooltip-dark`, US2 s4). After **Run without it for now**: an amber "No sandbox" item and the **Try the sandbox again** banner (`t039-3-bars-dark` rows 4–5) |
| 5 | Lost connection: red "Daemon disconnected · reconnecting…", no banner, green again after | pass: the pass's own daemon was paused with SIGSTOP. The bar turned red, no banner appeared and the body did not move (`t039-4-disconnected-dark`, `t039-3-bars-dark` row 3). After SIGCONT the item read connected again |
| 6 | Second window: **Take over** banner and "Read-only · another window" | not run live: one window per pass here. Covered by the `status-bar-warnings` snapshot state and the model tests |
| 7 | 640 dp: hints first, then usage, then "…" | pass at 640×480 (`t039-6-narrow-640-dark`): the client's own texts fit whole there, so nothing drops or shortens. That is the give-way pass working: it drops only when needed. The forced give-way order is held by `status_bar_fits` with long texts (MU6, MU7 caught), and the shortened look by `m1-5-overflow-dark` |
| 8 | ⋮ menu and quick switcher over the bar; dark scheme | pass: the switcher's scrim covers the bar (`t039-7-switcher-over-bar-light`). The menu opens at the top and does not reach the bar. Dark scheme in `t039-3-bars-dark`, `t039-4`, `t039-5`, `t039-6` |

SC-006 (connected and sandbox state readable from the bar alone): pass in every frame above.
macOS flush corners and Cmd notation: not checked (Linux host). Under tiny-skia, the quick
switcher's surface rendered black behind its field (`t039-7`). This was seen only in this backend
and is not part of this feature, so it is left as a follow-up in the ledger.
