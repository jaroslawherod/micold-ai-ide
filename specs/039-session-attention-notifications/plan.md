# Implementation Plan: Notify When a Session Needs Attention, and Track Unread Sessions

**Branch**: `feat/notify-session-needs-attention` | **Date**: 2026-10-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/039-session-attention-notifications/spec.md`

## Summary

A session that changes to awaiting input while no window has it in view raises one desktop
notification and becomes unread. The unread mark is on its sidebar row, a count on its project's
row in the switcher, and a total for the other projects on the switcher's button. Clicking the
notification brings the right window to the front and shows the session. One Settings switch
turns the notifications off.

The session service is the arbiter, because it is the only party that runs while no window is
open and that sees every window ([research R1](./research.md)):

1. Each window reports whether a session is in view (`WindowView`).
2. On a change into `AwaitingInput` with the session in view nowhere, the service adds one to the
   session's `attention_seq` and sets `unread`, both stored in the catalog.
3. A window that sees a higher `attention_seq` claims it; the service grants each sequence to one
   window, and that window shows the notification through the operating system's own facility.
4. A window that reports the session in view clears `unread`.
5. A click is sent to the service, which forwards it to the window that holds the project.

## Technical Context

**Language/Version**: Rust, edition 2021, workspace `rust-version` 1.97 (unchanged)

**Primary Dependencies**: `iced` 0.14 (`window::gain_focus`, `minimize`, `request_user_attention`,
`run`); new direct dependencies of `micold-client`, each target-specific: `zbus` 5.19 (Linux,
already in `Cargo.lock`), `wayland-client` 0.31, `wayland-backend` 0.3 (`client_system`) and
`wayland-protocols` 0.32 (`client`, `staging`) (Linux, all already in `Cargo.lock`),
`mac-usernotifications` 0.3.1 (macOS, new), `tauri-winrt-notification` 0.8.1 (Windows, new). Licences are `MIT OR Apache-2.0` throughout ([research R4](./research.md)).

**Storage**: two fields per session in the service's `projects.json` (`attention_seq`, `unread`)
and one field in `settings.json` (`desktop_notifications`). All `#[serde(default)]`, no
`schema_version` bump. Nothing is stored by the client.

**Testing**: `mise run test-core` for the render-free rules; `mise run gate` for the workspace
(daemon rules, client reducers, component gates); quickstart §B with the `visual-pass` skill for
what only a display shows; quickstart §C by hand on macOS and Windows.

**Target Platform**: Linux, macOS, Windows desktop. Three `cfg` arms in
`shell/desktop_notify/` and one Linux-only arm in `shell/window_raise.rs`; nothing else names an
operating system.

**Project Type**: desktop application (three-crate Cargo workspace).

**Performance Goals**: notification within 2 s of the indicator's change (SC-001): the path is one
local round trip plus the system call. Mark and counts within 1 s of coming into view (SC-006):
the client hides the mark of the session it has in view without waiting for the service.

**Constraints**: the container gets no new access (FR-007); nothing leaves the computer (FR-025);
no rate limit, grouping or withdrawal (spec Out of Scope); the activity indicator, the running
count and the in-app notices are not edited (FR-032).

**Scale/Scope**: one new core module (`attention`), one new daemon module (`attention`), one new
client feature (`features/attention.rs`), one new platform module (`shell/desktop_notify/`), one
new shared component (`UnreadMark`), three extended components, six wire changes, one setting.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Verdict | How |
|---|---|---|
| **I. Test-First (NON-NEGOTIABLE)** | PASS | Every rule is a pure function or reducer with a failing test first: `micold_core::attention`, `Workspace` counts, `attention::Views` in the daemon, `features/attention.rs` (which holds `raise_plan`, the decision how a window is raised), each backend's event mapping. What is left in `shell/desktop_notify/` and `shell/window_raise.rs` is one call per step into a notification system or `iced::window`, with no branch of its own. `shell/` is declared by the binary's `src/main.rs`, so it cannot be reached from `tests/`; it is validated by quickstart §B and §C under the principle's exception for the binary's glue. |
| **II. Multi-Session Support** | PASS | `attention_seq` and `unread` are per session, in the session's own record; each session is claimed and granted on its own (FR-009). They are persisted and restored with the session. |
| **III. Worktree Integration** | PASS | No file or VCS operation. A session of the Default entry is named by the sidebar's own label (FR-004) and counted like any other (FR-022). |
| **IV. Local-First Storage (NON-NEGOTIABLE)** | PASS | State is in the service's catalog and settings file on the user's computer; the only transport is the existing local connection; the notification is shown by the local operating system. Nothing depends on a network. |
| **V. Rust + iced Stack** | PASS | Rust and iced only. `WindowView.in_view` is one `Option`, so two sessions in view in one window cannot be expressed. |
| **VI. Cross-Platform Parity** | PASS | One trait, three backends behind `cfg` in one directory; core and daemon code never branch on the OS. CI builds and tests all three. One declared difference: on Wayland a click may not give keyboard focus ([research R7](./research.md)); the spec's FR-015 is worded for notification services that differ. |
| **VII. Documentation First-Class** | PASS | Each milestone updates the user guide with what it ships (FR-031): `worktrees-and-sessions.md`, `project-selection.md`, `settings.md`. `docs/development/architecture.md` and `component-library.md` in the last milestone. |
| **VIII. Reusable UI Component Foundation** | PASS | `UnreadMark` is a shared builder component; `TreeItem`, `MenuItem` and `Button` are extended, not copied; the showcase gets three entries (FR-030). |

No violation; Complexity Tracking is empty.

### Re-check after Phase 1 design

Unchanged. The design added no storage outside the two existing files, no OS branch outside
`shell/`, and no widget outside `ui/material/`.

## Requirement → design map

| Requirement | Design | Where |
|---|---|---|
| FR-001 | Attention event → claim → grant → `show` | research R1, R3; wire W1; N1 |
| FR-002 | No event while in view in any window | R2; W1.3; data-model A3 |
| FR-003 | Only a change of `ActivitySignal` counts | R3; W1.3; A2 |
| FR-004 | `notification_text` from the sidebar's three labels | R4; N2 |
| FR-005 | First sight of a session adopts, never claims; only `AwaitingInput` makes an event | R3 table row 1; A2 |
| FR-006 | The `Reconnected` rule of `AttentionTracker::observe` | R3 table row 3 |
| FR-006a | One grant per sequence | R3; W1.4 |
| FR-007 | The window shows the notification; the service uses the existing connection | R1, R4 |
| FR-008 | No window, no claim; the event still sets `unread` | R1; A6 |
| FR-008a | `unread` in `StoredSession`; default `false` | R1; data-model Durable |
| FR-009 | Per-session sequence, claim and grant | R3; W1.4 |
| FR-010 | `Err` logged once, ignored | R4; N4 |
| FR-011 | `RevealSession` → `raise_plan` → `Reopened` + `Selected` | R6, R7; N5, N6; W3.4 |
| FR-012 | The service picks the holder, else the most recently focused | R6; W3.1 |
| FR-013 | `Reveal::Unavailable` → notice, no change | R6; N5 |
| FR-014 | Only the two existing selection messages are sent | R6; N5; W3.3 |
| FR-015 | Every backend shows without needing a click report; a click nobody can resolve changes nothing | R4, R6 Known limit; N7, N9 |
| FR-015a | No launch registration, no session argument | R6; N8 |
| FR-016 | `unread` set with the event, not when in view | R1; W2.1; A1, A3 |
| FR-017 | `unread` does not depend on claim, grant or setting | W2.3; W4.2 |
| FR-018 | `UnreadMark` on `TreeItem`, by position and weight | R9; U1, U3 |
| FR-019 | `WindowView` clears; the client hides at once | R2; W2.2; U5 |
| FR-020 | Only a view report clears; removal drops the record | A4, A5 |
| FR-021 | `MenuItem::trailing_mark` with the word | R5, R9; U6 |
| FR-022 | Counts read `Workspace::sessions`, not the filtered sidebar | R5; U6 |
| FR-023 | `Button::trailing_mark` with `other_projects_unread` | R5, R9; U7 |
| FR-024 | One holder, sent to every window | R1 |
| FR-025 | Catalog file and local connection only | R1 |
| FR-026 | `Settings::desktop_notifications`, default `true` | R8; W4.1 |
| FR-027 | The grant is refused while off, and events while off are recorded as granted | R8; W4.2 |
| FR-028 | One field, no per-CLI field | R8 |
| FR-029 | Three backends behind one trait | R4; contract Backends |
| FR-030 | `UnreadMark` and three showcase entries | R9; contract Showcase |
| FR-031 | User-guide task in each milestone | Delivery order |
| FR-032 | Existing indicator, count and notices not edited | R9; U8 |

## Test strategy by layer

| Layer | Runs with | Covers |
|---|---|---|
| Core unit (`crates/micold-core/src/attention.rs`, `workspace.rs`, `settings.rs`, `store.rs`) | `mise run test-core` | `in_view` (FR-002, FR-016, story 1 scenarios 9 and 10); `AttentionTracker::observe` (FR-001, FR-003, FR-005, FR-006, FR-009); `notification_text` (FR-004); `resolve_reveal` (FR-011, FR-013); counts with and without a session in view (FR-019, FR-021 to FR-023); defaults of the stored fields and the setting, and a store round trip that writes the two fields to the catalog file and nowhere else (FR-008a, FR-025, FR-026); `Settings` has one notification field and none per AI CLI (FR-028) |
| Core protocol (`crates/micold-core/tests/schema_hash.rs`, round-trip tests in `messages.rs`) | `mise run test-core` | W1 to W4 encode and decode; one version bump per milestone |
| Core source scan (`crates/micold-core/tests/`, in the style of `macos_registers_nothing.rs`) | `mise run test-core` | No backend or packaging file registers the application to be started by a notification, and the client's entry point reads no argument (FR-015a, N8) |
| Daemon unit (`crates/micold-daemon/src/attention.rs`) | `mise run gate` | `Views`: in view, grant once, grant refused while off, an event while off never granted later, reveal target (FR-006a, FR-012, FR-027) |
| Daemon integration (`crates/micold-daemon/tests/`) | `mise run gate` | Two connections: event with none in view, no event with one in view, clear on view, persistence across a restart of the service, no event for a repeated signal, removal (FR-002, FR-003, FR-008, FR-008a, FR-016, FR-019, FR-020, FR-024); with the setting off an event still sets `unread` and is never granted (FR-017, FR-027; all three AI CLIs share the one path, FR-028); reveal routing with the token forwarded (FR-012); Principle II: several sessions at once (FR-009) |
| Client reducer (`crates/micold-client/src/features/attention.rs`, `tests/`) | `mise run gate` | `WindowView` sent on change only; a grant calls the recording notifier once with the right text; an error is logged once (FR-010); `RevealSession` selects or notices (FR-011, FR-013, FR-014); `raise_plan` and `after_activation` for each row of the contract's table (FR-011); switcher entries carry counts |
| Component gates (`crates/micold-client/src/ui/material/`) | `mise run gate` | `UnreadMark` contrast in both schemes, row height unchanged, label truncates first (FR-018, FR-023, FR-032; U3, U8) |
| Backend mapping (`shell/desktop_notify/*.rs`, pure functions) | `mise run gate`, on each OS in CI | Signal or callback → `NotifierEvent`; id table; an id the table does not hold yields no event (N9); the Linux request is built the same whether or not the service lists the `actions` capability (FR-015) |
| Quickstart §B, `visual-pass` | recorded | The mark, the counts and the switch in both schemes; a real notification on a Linux notification service; a click on X11 (FR-018, FR-021, FR-023, SC-001 to SC-006, SC-008 to SC-010) |
| Quickstart §C, by hand | recorded | macOS and Windows: shown, clicked, refused by the system (FR-029, SC-001, SC-004); the sandbox on Linux (FR-007, SC-007) |

## Project Structure

### Documentation (this feature)

```text
specs/039-session-attention-notifications/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── wire.md
│   ├── desktop-notification.md
│   └── unread-mark.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── attention.rs                    # new: in_view, AttentionTracker, notification_text, resolve_reveal
├── session.rs                      # Session: attention_seq, unread
├── store.rs                        # StoredSession: attention_seq, unread
├── workspace.rs                    # unread_session_count, other_projects_unread
├── settings.rs                     # desktop_notifications
└── protocol/
    ├── messages.rs                 # W1–W4
    └── version.rs                  # 21 to 26, one per milestone that changes the wire

crates/micold-daemon/src/
├── attention.rs                    # new: Views
├── state.rs                        # note_activity: attention event; set_window_view; deregister
├── catalog.rs                      # mark_attention, mark_read, session_summary, service settings
└── server.rs                       # WindowView, AttentionClaim, SessionReveal, SettingsSet

crates/micold-client/src/
├── features/attention.rs           # new: State, reducer, the DesktopNotifier seam, raise_plan
├── features/project.rs             # SwitcherEntry.unread_count
├── features/settings.rs            # the switch's draft field and message
├── app.rs                          # switcher_entries
├── catalog_sync.rs                 # copy attention_seq, unread
├── main.rs                         # derive and send WindowView after each update
├── shell/
│   ├── daemon_sync.rs              # observe, claim, AttentionGranted, RevealSession
│   ├── desktop_notify/             # new: mod.rs, linux.rs, macos.rs, windows.rs
│   └── window_raise.rs             # new: carries out raise_plan's steps
├── ui/
│   ├── material/unread_mark.rs     # new
│   ├── material/{tree_view,menu,button}.rs
│   ├── sidebar.rs, toolbar.rs, mod.rs
│   └── settings/environment.rs
└── showcase/{catalogue.rs,sections/atoms.rs}

packaging/windows/micold-ai-ide.iss # AppUserModelID on the shortcut
docs/user-guide/{worktrees-and-sessions,project-selection,settings}.md
docs/development/{architecture,component-library}.md
```

**Structure Decision**: the existing three-crate workspace. The rules live in `micold-core` and
the daemon; the client adds one feature module, one platform directory and one component.

## Delivery order

| Milestone | Ships | Wire |
|---|---|---|
| M1 — story 1, slice A | View report and attention sequence in the service; nothing new on screen | 21 |
| M2 — story 1, slice B | Claim and grant, the tracker with the reconnect rule, the notification text, the seam and the Linux backend; user guide: the notification | 22 |
| M3 — story 1, slice C | The macOS and Windows backends, the installer's application identity; user guide: the three systems and the system's permission | — |
| M4 — story 2, slice A | `unread` in the service, `UnreadMark`, the row mark, its showcase entry; user guide: the mark, unread after reopening | 23 |
| M5 — story 2, slice B | The switcher counts and the button total, their showcase entries; user guide: the counts | — |
| M6 — story 3, slice A | Click reporting in the three backends, reveal routing, raising the window, the unavailable notice; user guide: clicking, and that a notification of a closed window does not open the session | 24 |
| M7 — story 3, slice B | Wayland: the probe first, then the token on the reveal pair and surface activation; user guide: the result | 25 |
| M8 — story 4 | The setting and the switch; user guide: Settings | 26 |
| M9 — polish | Architecture and component-library docs, quickstart §B and §C recorded | — |

Stories 1 to 3 are cut in slices because each is well past the size one milestone carries
([tasks.md](./tasks.md), Milestones). Each story stands alone as the spec says: story 1 notifies
with no mark, story 2 marks with no dependence on the notification (M4 needs M1 only), story 3
adds the click to story 1's notification, story 4 adds the switch (on until then).

## Risks

| Risk | Handling |
|---|---|
| Wayland focus from a click (M7) needs `xdg_activation_v1` through foreign handles (the binding is the one `smithay-clipboard` uses in this application today); whether a compositor honours the notification's token is unverified | M7 starts with a probe; the fallback (`request_user_attention`) ships if it fails, with the limit in the user guide and a follow-up in the ledger ([R7](./research.md)) |
| Windows shows nothing without a registered `AppUserModelID`, silently | The installer's shortcut carries it; quickstart §C checks an installed build; the failure is silent by FR-010 |
| macOS shows nothing for an unbundled or unsigned binary | Development builds log once and continue; the shipped bundle is ad-hoc signed |
| `mac-usernotifications` is a young crate | It is small, pure `objc2`, by the author of `notify-rust`; it sits behind the trait, so `objc2-user-notifications` can replace it without touching callers |
| A stalled connection counts as viewing for up to 9 s | Stated limit ([R3](./research.md)) |
| macOS and Windows passes cannot be run on the Linux host | Automated tests run on all three in CI; §C is run by hand and its result recorded, or listed as a follow-up in the ledger |

## Complexity Tracking

No violations.
