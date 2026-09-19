# Data model: A session the AI CLI never titled still gets a label

**Feature**: [spec.md](./spec.md) · **Contract**: [first-turn-label.md](./contracts/first-turn-label.md)

## SessionLabel (`crates/micold-core/src/session.rs`) — changed

| Variant | Meaning | Rendered as | Source |
|---|---|---|---|
| `Pending` | No title and no usable typed turn known | "New session" | — |
| `Derived(String)` **new** | The first typed turn, shaped (FR-002, FR-003) | the text | `AiCliProvider::read_label` |
| `Named(String)` | The AI CLI's title | the text | `read_title` (`ai-title`; Copilot `name:` else `summary:`), or the live terminal title |

`SessionLabel::display()` renders `Derived` as it renders `Named` (D7). The string is never empty
(C6.4, C8.2).

### Transitions

```
Pending ──label──▶ Derived ──title──▶ Named ──newer title──▶ Named
   └──────────────title──────────────────▲
```

- `Pending → Derived`: `Catalog::record_session_label` (only from `Pending`; C6.4).
- `Pending → Named`, `Derived → Named`, `Named → Named'`: `Catalog::record_session_name`
  (`Session::set_title`).
- **Forbidden**: any transition into `Pending`; `Named → Derived`; `Derived → Derived'` (a label is
  derived once; FR-007 — the first turn does not change).

### Invariants

1. A session carries at most one of title, label (spec *Key Entities*).
2. A label never replaces a title, in memory, on disk or on the wire (FR-005).
3. Every write is addressed by `SessionId` through `Workspace::find_session_mut` (029 invariant 4,
   Principle II): a label derived from session A's records can only reach session A.
4. A `Derived` session is never pruned by `prunable_session_cwds` (it has, or had, a conversation).

## Session (`crates/micold-core/src/session.rs`) — gains one method

`fn set_derived_label(&mut self, label: impl Into<String>) -> bool` — sets `Derived` only when the
label is `Pending` and the text is non-empty; returns whether it changed. The precedence lives in
the type's own method, not only in the catalog.

## StoredSession (`crates/micold-core/src/store.rs`) — gains one field

| Field | Type | Serde | Meaning |
|---|---|---|---|
| `title` | `Option<String>` | `default` (unchanged) | A title |
| `label` **new** | `Option<String>` | `default`, `skip_serializing_if = "Option::is_none"` | A derived label (FR-008) |

Load: `title` ⇒ `Named`; else non-empty `label` ⇒ `Derived`; else `Pending` (C8.2). No
`schema_version` bump (research R2).

## SessionSummary (`crates/micold-core/src/protocol/messages.rs`) — wire

`title: SessionLabel` is unchanged in shape and now can hold `Derived`. `PROTOCOL_VERSION` 13 → 14
(research R3).

## Client session model (`crates/micold-client/src/catalog_sync.rs`)

Adopts `summary.title` when it is `Named` **or `Derived`** (was: `Named` only); never adopts
`Pending` over an existing label.

## Provider seam (`crates/micold-core/src/provider.rs`)

| Item | Change |
|---|---|
| `AiCliProvider::read_label` | **new**, required (C1) |
| `ClaudeProvider::read_label` | first `claude` turn in the first 1 MiB of the transcript (C2, C3, C5) |
| `CopilotProvider::read_label` | first Copilot turn in the first 1 MiB of `events.jsonl` (C2, C4, C5) |
| `CopilotProvider::read_title` | `name:`, else `summary:` (C7) |
| `CopilotProvider::read_yaml_scalar` | rejects block scalars (C7.2) |
| `PiProvider::read_label` | `None` (C1.2) |
| `FakeAiCliProvider` | `with_label`, `read_label` (C1.3) |

The record parsing and shaping are pure functions over a byte prefix in a new module,
`crates/micold-core/src/first_turn.rs` (`claude_first_turn(&[u8])`, `copilot_first_turn(&[u8])`,
`shape_label(&str)`, `read_prefix(&Path)`), so they are unit-tested without a provider or a
filesystem layout.

## Catalog (`crates/micold-daemon/src/catalog.rs`) — gains one method

`fn record_session_label(&mut self, id: SessionId, label: &str) -> io::Result<bool>` (C6.4). The
existing `record_session_name` already replaces `Derived` (C6.5).
