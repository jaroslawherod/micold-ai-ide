# Phase 1 Data Model: A session keeps its name when nothing is running it

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

This feature adds **no type, no field, and no storage location**. What it changes is which of three
existing representations of a session's name is authoritative, and in which direction they flow. So
this document is mostly about the relationships and the invariants that must hold between them.

## Entities

### `SessionLabel` (`micold-core/src/session.rs`) — unchanged

```rust
pub enum SessionLabel {
    Pending,           // no name yet → display() == "New session"
    Named(String),     // the AI CLI's name for the conversation
}
```

The in-memory name of a session, carried on `Session::label` and on the wire as
`SessionSummary::title`. Two states only, which is what keeps "a session with a blank name"
unrepresentable (Principle V) — the placeholder is a *rendering* of `Pending`, never a stored value.

| Field | Rule |
|---|---|
| `Named(s)` | `s` is the AI CLI's text, glyph-stripped (research R1), stored verbatim. Never user-entered (FR-011). |
| `Pending` | Means *no name is known anywhere*, not *no name is loaded*. After this feature the second case cannot arise. |

**Mutation**: only through `Session::set_title`, which replaces whatever was there — latest wins
(FR-005). There is deliberately no `clear_title`: nothing in this feature moves a label from `Named`
back to `Pending` (FR-008, research R5).

### `StoredSession::title: Option<String>` (`micold-core/src/store.rs`) — unchanged shape, newly written

The durable form, in the per-project state file. Maps to `SessionLabel` in both directions already:

| `SessionLabel` | `StoredSession::title` |
|---|---|
| `Named(s)` | `Some(s)` |
| `Pending` | `None` (`#[serde(default)]`, so an absent key loads as `Pending`) |

No `schema_version` bump: the field, its serde attributes, and both conversions
(`from_session` / `into_session`) already exist and are already exercised. What changes is that
`from_session` now regularly sees a `Named` label for a session this application started, where
before it saw `Pending` for all of them.

### `LiveSession::last_title: Option<String>` (`micold-daemon/src/state.rs`) — unchanged, demoted

The most recent OSC-0 title of the attached process. Stays runtime-only and un-persisted; what
changes is its standing. Before: the *only* accurate name, projected over a stale record. After: an
**observation** that is written through to the record, so the projection and the record agree.

`None` no longer means "show the placeholder" — it means "this process has not reported a title
since it was attached", and the catalog's own label answers instead.

### AI CLI records (`micold-core/src/provider.rs`) — unchanged, read-only

Each provider's own on-disk store of the conversation, read through `AiCliProvider::read_title`
(`claude`'s latest `{"type":"ai-title",...}` record; Copilot's `workspace.yaml` `name:` scalar).
A **secondary** source, consulted only for a session whose label is `Pending` (recovery, FR-006),
and never depended on for a name already recorded (FR-008).

## Relationships

```text
          AI CLI conversation
                  │
      ┌───────────┴────────────┐
      │ (live, per tick)       │ (recovery, per project open)
      ▼                        ▼
 OSC-0 title ──strip──► LiveSession::last_title      AiCliProvider::read_title
      │                        │                              │
      │                        └──────────┬───────────────────┘
      │                                   ▼
      │                    Session::label  (SessionLabel)          ◄── authoritative
      │                                   │
      │                          ┌────────┴────────┐
      │                          ▼                 ▼
      │              StoredSession::title    SessionSummary::title ──► client sidebar
      │                   (durable)                (wire)
      └──────────────────────────────────────────────┘
             the projection now *feeds* the record instead of masking it
```

## Invariants

1. **The record is never behind the screen by more than one change.** Whenever a name is displayed
   for a session, that same name has been written, or a write for it has been attempted and logged
   (FR-003, FR-009).
2. **A name is never un-set.** No transition from `Named` to `Pending` exists — not on a missing
   transcript, not on a failed read, not on a restart (FR-008).
3. **The newest name wins.** A later observation replaces an earlier one unconditionally; a name is
   never treated as first-write-wins (FR-005).
4. **A name write is addressed by `SessionId`.** Never by index, by position in a project's session
   list, or by location. This is what makes FR-012 (no session displays, inherits, or overwrites
   another's name) a property of the write path rather than of the caller's care — the lookup goes
   through `Workspace::find_session_mut`, which returns the owning project alongside the session.
5. **`Pending` means no name exists anywhere**, not "not yet loaded" — so a row reading
   "New session" is a truthful statement about the conversation, which is what FR-004 asks for.
6. **The catalog has one writer.** The daemon. The client holds names in memory for its own run and
   never persists them (the rule `remember_foreground` already states: `store.rs` has no locking, so
   a client-side write would clobber the daemon's).

## State transitions

```text
                      ┌──────────────────────────────────────────┐
                      │                                          │
   (session created)  ▼                                          │
        ────────► Pending ──── live OSC-0 title observed ────► Named(s) ──┐
                      │                                          ▲       │
                      │                                          │       │ re-title
                      └── recovery: provider record found ───────┘       │ (latest wins)
                                                                  ◄──────┘

        Pending ── recovery finds nothing ──► Pending      (no write, FR-004)
        Named(s) ── transcript deleted ─────► Named(s)     (no transition exists, FR-008)
        Named(s) ── daemon restart ─────────► Named(s)     (loaded from the record, FR-001)
```

Every arrow that lands on `Named` also persists, except where the persist itself fails — which
changes the in-memory state anyway and is logged, never surfaced (FR-009).

## Validation rules

| Rule | Source | Where enforced |
|---|---|---|
| An empty or whitespace-only name is not a name | FR-004 | Already: `parse_title` ignores an empty `aiTitle`; the recovery pass must not turn `Some("")` into `Named("")` — it takes `read_title`'s `Option` as-is. |
| A name equal to the one recorded writes nothing | FR-003 / SC-007 | The catalog method compares before persisting and reports whether it wrote (research R3). |
| A name is stored exactly as the CLI recorded it | Assumptions | No truncation, no trimming beyond the existing leading-status-glyph strip; row-level shortening stays a display concern. |
| Recovery only ever fills `Pending` | FR-006, FR-008 | The recovery pass filters on the label before it does any I/O — which is also what bounds its cost. |
