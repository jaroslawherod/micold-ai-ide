# Implementation Plan: Show Claude Plan Usage and the Next Limit Reset

**Branch**: `claude/project-thread-u1pay5` | **Date**: 2026-10-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/490-claude-plan-usage/spec.md`

## Summary

Show the Claude plan's usage and next limit reset in the app bar, warn at a configurable
threshold, and stay silent when there is nothing to show. Readings come from the `rate_limits`
object Claude Code passes to its status-line command (research R1). The daemon adds a `statusLine`
block to the per-session `--settings` file it already writes (R2); the block runs the daemon's own
executable as a relay (`micold-daemon status-line <relay-file>`) that posts only `rate_limits` to
the existing loopback hook receiver and runs the user's own status line command so theirs keeps
working (R3, R4). The daemon keeps one in-memory reading for the account, newest wins (R6), and
broadcasts it; the client draws a new shared `UsageIndicator` and decides currency against the
reset times (R7). Two service-owned settings (switch on by default, threshold 80%, 50–100) follow
feature 613's threshold path (R9).

The source sits behind a seam: everything after `plan_usage::status_line` and the relay works on a
source-neutral `UsageReading`, and the relay is its own milestone (M2), so if the user answers the
open escalation differently (Decisions 1–2) only M2 and one constant change.

## Technical Context

**Language/Version**: Rust (workspace edition and toolchain as pinned in `rust-toolchain.toml`)

**Primary Dependencies**: iced 0.14 (client), tokio (daemon), serde / serde_json, `directories`,
`chrono` (workspace dependency, already used by the daemon; added to `micold-core` and
`micold-client` for local-time formatting, R11). No new crate.

**Storage**: two fields in the existing `settings.json` (`plan_usage_enabled`,
`plan_usage_warn_percent`). The reading is memory only (FR-014). One owner-only relay file per
running Claude session beside the existing hook settings file.

**Testing**: `cargo test` via `mise run gate`; core logic in `micold-core` unit/integration tests;
daemon route and relay in `micold-daemon/tests`; client feature module and gates in
`micold-client/tests`; quickstart Part B visual pass.

**Target Platform**: Linux, macOS, Windows; the sandboxed daemon (feature 027) unchanged — the
relay is the daemon executable, present in the image.

**Project Type**: desktop application (iced client + local daemon).

**Performance Goals**: reading shown ≤ 10 s after Claude Code passes it (FR-007, SC-005; the
expected path is the 300 ms status-line debounce plus a loopback POST). The relay adds no wait to
the user's status line output (FR-017).

**Constraints**: no network request of the feature's own (FR-012, SC-004); no credential read
(FR-013); nothing at warning level or above (FR-016); offline-safe (Principle IV).

**Scale/Scope**: one reading per account regardless of session count; ten sessions cost ten
loopback POSTs per turn at most, deduplicated before broadcast (R6).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
all PASS, no Complexity Tracking entry.

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every module below starts from a failing test named in *Test strategy*; logic lives in `micold-core::plan_usage` and the render-free `features/plan_usage.rs`, views only map. |
| **II. Multi-Session** | PASS | No per-session state is added: the reading is the account's (R6). Per-session pieces (relay file, token) reuse the 026 hook registry and die with the session. Ten sessions → one indicator (spec Edge Cases). |
| **III. Worktree Integration** | PASS | No file or VCS operation. The user status-line lookup uses the session's cwd (worktree or Default root) read-only. |
| **IV. Local-First (NON-NEGOTIABLE)** | PASS | No request of the feature's own; the relay talks to `127.0.0.1` only; reading in memory only; works offline by showing nothing. On by default is justified by FR-001: nothing leaves the device for this feature, so there is nothing to opt into. |
| **V. Rust + iced** | PASS | Rust only. `WarnPercent` makes an out-of-range threshold unrepresentable; `UsageReading` has no empty-windows value after parsing. |
| **VI. Cross-Platform** | PASS | One relay for all platforms; the only platform branch (which shell runs the user's command, R4) sits in `status_relay::user_shell` behind one function; quickstart §B4 checks Windows. CI covers all three. |
| **VII. Documentation** | PASS | `docs/user-guide/settings.md` gains *Environment → Claude plan usage* (what is read: `statusLine` of Claude Code's settings files and the status-line data; nothing sent), and `docs/user-guide/worktrees-and-sessions.md` a short *Plan usage in the app bar* note. |
| **VIII. Reusable UI** | PASS | New shared `UsageIndicator` with a builder ending in `.into()`, showcase entry and gates (contracts/usage-indicator.md); tooltip and toolbar are the existing shared ones. |

## Requirement map

| FR | Where |
|---|---|
| FR-001 | `Settings.plan_usage_enabled` default `true` (R9); Settings → Environment switch |
| FR-002 | `settings_json` omits `statusLine` and no relay file when off (contract S1.3); `set_plan_usage_enabled(false)` clears the reading (data-model) |
| FR-003 | Settings field note + `docs/user-guide/settings.md` (Principle VII row) |
| FR-004 | `CurrentUsage::headline`/`label`; `UsageIndicator` in the app bar (contract U3) |
| FR-005 | `WarnPercent` clamp/parse; client field validation (contract W3) |
| FR-006 | `CurrentUsage::details`; tooltip (U2.3) |
| FR-007 | relay per status-line run (S2); `PlanUsageChanged` broadcast (W2); switch rewrites live settings files (R9) |
| FR-008 | `plan_usage::format_reset` with local `FixedOffset` (R11) |
| FR-009 | `UsageReading::current(now)`; 30 s tick while a reading exists (R7) |
| FR-010 | `CurrentUsage::warning`; warning glyph + role (U2) |
| FR-011 | No notification path is touched; the indicator is the only output |
| FR-012 | `status_line::extract/parse`; no HTTP client is added for this feature (R1) |
| FR-013, SC-006 | Only `statusLine` deserialised from Claude settings files (R4); no credential source touched |
| FR-014 | Relay posts only `rate_limits` to loopback (S2.3); reading not persisted (R6) |
| FR-015, FR-016 | Silent relay (S2.5); `debug`-once route logging (S3.1); indicator hidden on `None` |
| FR-017 | Relay runs POST and user command concurrently, 500 ms POST bound (S2.3–S2.4) |
| FR-018 | Daemon-owned settings + broadcast (W2, W3) |
| FR-019 | `note_plan_usage` replaces, newest wins (R6) |
| FR-020 | `UserStatusLine::resolve` + relay chaining (R4, S2.4, S2.6) |

## Project Structure

### Documentation (this feature)

```text
specs/490-claude-plan-usage/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── status-line-relay.md
│   ├── plan-usage-wire.md
│   └── usage-indicator.md
└── tasks.md            # next unit
```

### Source Code

```text
crates/micold-core/src/
├── plan_usage.rs                 # NEW: UsageReading, LimitWindow, CurrentUsage, WarnPercent, format_reset
├── plan_usage/status_line.rs     # NEW: source A parse (extract, parse)
├── plan_usage/user_status_line.rs# NEW: UserStatusLine::resolve, RelayConfig
├── mcp/binding.rs                # NEW method ConfigLocations::claude_settings_dir (extend)
├── settings.rs                   # two fields, defaults, clamp (extend)
└── protocol/messages.rs          # DaemonSettings, SettingsSet, Welcome, PlanUsageChanged (extend)

crates/micold-daemon/src/
├── main.rs                       # `status-line` dispatch before the runtime (extend)
├── status_relay.rs               # NEW: the relay (S2), shell choice, bounded POST
├── hooks.rs                      # statusLine block, relay file, /status route (extend)
├── catalog.rs, server.rs         # settings plumbing like long_task_threshold (extend)
└── state.rs                      # plan_usage reading, note_plan_usage, set_plan_usage_enabled (extend)

crates/micold-client/src/
├── features/plan_usage.rs        # NEW: render-free reading state + tick decision
├── features/settings.rs          # two draft fields, threshold validation (extend)
├── shell/daemon_sync.rs          # Welcome / PlanUsageChanged routing (extend)
├── shell/subscriptions.rs        # 30 s tick only while a reading exists (extend)
├── icons.rs                      # Icon::PlanUsage U+E1AF, Icon::UsageWarning U+E002 (extend)
├── ui/material/usage_indicator.rs# NEW shared component
├── ui/toolbar.rs                 # place the indicator (extend)
├── ui/settings/environment.rs    # switch, threshold, statement (extend)
└── showcase/sections/atoms.rs, showcase/samples.rs  # entry (extend)

docs/user-guide/settings.md, docs/user-guide/worktrees-and-sessions.md
```

**Structure Decision**: the existing three-crate layout. Pure logic in `micold-core`, I/O in the
daemon, rendering in the client's `ui`, state in its render-free `features`.

## Milestones (for the tasks unit)

- **M1 — Core model, settings and wire** (US4 partly, FR-005, FR-008–FR-010, FR-019 logic):
  `plan_usage.rs`, `WarnPercent`, settings fields, `DaemonSettings`/`SettingsSet`, `Welcome` and
  `PlanUsageChanged`, `note_plan_usage` and `set_plan_usage_enabled` (reading part), server
  plumbing. Shippable alone: no reading can arrive yet, so nothing is visible.
- **M2 — Source A: relay and route** (FR-007, FR-012–FR-017, FR-020): `status_line.rs`,
  `user_status_line.rs`, `status_relay.rs`, `main.rs` dispatch, `hooks.rs` block, relay file and
  `/status` route, switch rewrites live settings files. The swappable milestone.
- **M3 — Indicator, Settings UI and docs** (US1–US4 views, FR-001, FR-003, FR-004, FR-006,
  FR-018): `features/plan_usage.rs`, subscription, icons, `UsageIndicator`, toolbar, settings
  section, showcase, gates, user guide.

## Test strategy

| Layer | Covers | Tests (new unless marked) |
|---|---|---|
| Core unit/integration | headline and tie rules, `current(now)` drops passed windows, tenths rounding, out-of-range and partial windows, `spend_limit` excluded, unknown keys named, `format_reset` within/after 24 h and offsets, `WarnPercent` clamp/parse, `UserStatusLine` precedence and `CLAUDE_CONFIG_DIR`, only `statusLine` read, settings defaults and clamp, wire round-trip | `micold-core/tests/plan_usage.rs`, `plan_usage_status_line.rs`, `plan_usage_user_status_line.rs`, `settings_plan_usage.rs`, `settings_contract_examples.rs` (extend) |
| Daemon | `/status` route table S3 (auth before body, 413, 400, 200-drop when off), debug-once logging, newest-wins and dedupe broadcast, switch off clears and rewrites settings files without `statusLine`, relay S2.1–S2.6 with a fake shell and fake receiver (output passthrough, exit status, recursion guard, timeout does not delay output, only `rate_limits` posted) | `micold-daemon/tests/plan_usage_route.rs`, `plan_usage_setting.rs`, `status_relay.rs`; `hooks_receiver.rs` (extend: settings JSON with and without `statusLine`) |
| Client (render-free) | reading from Welcome/PlanUsageChanged, cleared on disconnect, hidden when off, tick only while a reading exists, threshold field refuses 49/101 with the range message, change applies without new reading | `micold-client/tests/features_plan_usage.rs`, `features_settings.rs` (extend), `idle_subscriptions.rs` (extend) |
| Geometry gates | indicator height and bar positions, icon codepoints and roles | `tests/gates/bar_controls_hold_their_size.rs` (extend), `tests/icons.rs`, `tests/icon_roles.rs` (extend) |
| Client (render-free) | FR-003 statement present in the Settings section's field note | `features_settings.rs` (extend); FR-011 has no test of its own: no notification path is changed |
| Visual pass | look in light/dark, warning distinguishable, tooltip text, real Claude Code end to end, Windows shell chaining | quickstart Part B |

## Risks

- **Claude Code reloading a changed `--settings` file mid-session** is not documented as such;
  running sessions may keep their start-time behaviour until restarted. The spec already words
  FR-007 as "once Claude Code picks up the change"; the daemon still drops readings while off
  (S3.2), so FR-002's "no reading kept or shown" holds either way.
- **Windows shell selection** mirrors the documented Git Bash / PowerShell rule; a mismatch only
  affects the user's chained status line, checked in quickstart §B4.
- **Sandboxed daemon (027)**: the daemon, its hook receiver and the Claude sessions all run inside the container, so the relay's loopback POST stays inside it and `UserStatusLine::resolve` reads the container's view of home and the session cwd; a user status line that exists only on the host is not chained there (as for any host-only Claude configuration). Covered by the spec's container edge case; no extra test beyond the daemon tests, which do not depend on placement.
- **Decisions 1–2 await user confirmation**; see Summary for what changes.

## Complexity Tracking

None.
