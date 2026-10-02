# Data Model: Notify When a Session Needs Attention, and Track Unread Sessions

**Feature**: 039 | **Date**: 2026-10-02 | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

Types marked *new* do not exist yet. Everything else is extended in place.

## Durable: the session's attention state

Held by the session service, stored in `projects.json`, sent to every window.

| Where | Field | Type | Default | Meaning |
|---|---|---|---|---|
| `Session` (`crates/micold-core/src/session.rs`), `StoredSession` (`store.rs`), `SessionSummary` (`protocol/messages.rs`) | `attention_seq` | `u64` | `0` | Number of attention events: changes into `AwaitingInput` while in view in no window. Never decreases while the catalog is intact. |
| same three | `unread` | `bool` | `false` | Set by an attention event, cleared when a window reports the session in view. |

`StoredSession` fields carry `#[serde(default)]`; `schema_version` is not bumped (research R1).

**Invariants**

- A1. `unread` becomes `true` only together with `attention_seq += 1`.
- A2. Both change only on a change of `ActivitySignal` from another value into `AwaitingInput`
  (FR-003): a repeated waiting signal changes neither.
- A3. Neither changes when the session is in view in some connected window at that moment
  (FR-002, FR-016).
- A4. `unread` becomes `false` only when a window reports the session in view (FR-019, FR-020). A
  change of activity never clears it.
- A5. A session that is removed leaves the catalog and takes both fields with it (FR-020).
- A6. With no window connected no session is in view, so every change into `AwaitingInput` is an
  attention event (FR-008).

**State transitions of `unread`**

```text
            change into AwaitingInput, in view nowhere
  read  ─────────────────────────────────────────────────▶  unread
   ▲                                                          │
   └──────────── some window reports the session in view ─────┘
  unread ── change into AwaitingInput again ──▶ unread (attention_seq += 1)
```

## Service memory: `attention::Views` *(new, `crates/micold-daemon/src/attention.rs`)*

Pure, no I/O. Lost when the service stops.

| Field | Type | Meaning |
|---|---|---|
| `views` | `HashMap<ClientId, WindowView>` | The last report of each connection. Removed when the connection ends. |
| `focus_order` | `Vec<ClientId>` | Connections in the order they last reported `focused = true`, most recent last. |
| `granted` | `HashMap<SessionId, u64>` | The highest `attention_seq` granted per session. |

| Operation | Result |
|---|---|
| `set_view(client, view)` | Stores the report; returns the session that came into view, if any. |
| `remove(client)` | Forgets the connection. |
| `is_in_view(session)` | Whether any stored report names the session. |
| `note_event(session, seq, enabled)` | Added with the setting (story 4), as is `grant`'s `enabled`. Called for each attention event. While `enabled` is `false` it records `seq` as granted, so the event can never be claimed (FR-027). |
| `grant(session, seq, current_seq, enabled)` | `true` when `enabled`, `seq > granted[session]` and `seq <= current_seq`; then records `seq`. |
| `reveal_target(holder, sender)` | `holder` when given, else the last entry of `focus_order`, else `sender`. |

`WindowView { focused: bool, in_view: Option<SessionId> }` *(new, wire)*: `in_view` is `Some` only
when `focused` is `true`.

## Client, per process: `micold_core::attention` *(new module, render-free)*

### `ViewFacts` → `in_view`

| Input | Source in the client |
|---|---|
| `window_focused: bool` | `App::window_focused` |
| `main_area_taken: bool` | `state.settings.settings_draft.is_some()` |
| `selected: Option<SessionId>` | the active project's selected session |

`in_view(facts) -> Option<SessionId>`: `selected` when the window is focused and the main area is
not taken, else `None`. A regular terminal tab of the selected session does not change it.

### `AttentionTracker`

| Field | Type | Meaning |
|---|---|---|
| `seen` | `HashMap<SessionId, Seen>` | `Seen { seq: u64, awaiting: bool }` as last observed by this process. |

`observe(sessions, phase, in_view) -> Vec<Claim>` with `phase` one of `Live` and `Reconnected`,
and `Claim { session, seq }`. The rules are the table in research R3. Sessions absent from a full
snapshot are dropped from `seen`.

### `NotificationText`

`notification_text(project, worktree, session) -> NotificationText { title, body }`:
title `"{session} is waiting for input"`, body `"{project} — {worktree}"` (FR-004). The three
inputs are the labels the sidebar shows; an empty input is not possible because each label has a
placeholder of its own.

### `resolve_reveal`

`resolve_reveal(&Workspace, project, session) -> Reveal` with
`Reveal::Show { project, session }` or `Reveal::Unavailable` (research R6).

### Counts on `Workspace`

| Function | Value |
|---|---|
| `unread_session_count(&project, in_view: Option<SessionId>) -> usize` | Sessions of the project with `unread`, less `in_view`. The client passes the session its window has in view, so the count falls before the service answers (FR-019). |
| `other_projects_unread(&active) -> usize` | The sum of `unread_session_count(project, None)` over every project except `active`: a session in view is always of the active project. |

`SwitcherEntry` (`crates/micold-client/src/features/project.rs:118`) gains `unread_count: usize`.

## Client feature state: `features::attention::State` *(new)*

| Field | Type | Meaning |
|---|---|---|
| `tracker` | `AttentionTracker` | Above. |
| `sent_view` | `Option<WindowView>` | The last report sent on this connection; `None` after a reconnect. |
| `failure_logged` | `bool` | A show failure was logged in this run (FR-010). |

`raise_plan` and `after_activation` (contract desktop-notification, *Raising the window*) are pure
functions of this module. The activation token is not kept in the state: it travels in
`SessionReveal` and comes back in `RevealSession`.

## Setting

`Settings::desktop_notifications: bool`, `#[serde(default = "default_desktop_notifications")]`
returning `true`; service-owned; mirrored in `DaemonSettings` and `ClientMsg::SettingsSet`
(FR-026 to FR-028).

## Spec entities → model

| Spec entity | Model |
|---|---|
| Session's unread state | `unread` |
| Attention event | one increment of `attention_seq` |
| Desktop notification | one granted `Claim`, shown with `NotificationText`, keyed by project path and session id |
| Project unread count | `unread_session_count` |
| Other-projects unread total | `other_projects_unread` |
| Desktop notifications setting | `Settings::desktop_notifications` |
