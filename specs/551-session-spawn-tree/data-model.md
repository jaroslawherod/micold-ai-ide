# Data Model: Session Tree

## Session (existing, extended)

| Field | Type | Persisted | Notes |
|---|---|---|---|
| `origin` | `Option<SessionOrigin>` | id only | write-once; `None` for every session today |

`SessionOrigin { id: SessionId, status: OriginStatus }`; `OriginStatus = Known | Removed`.
`status` is runtime: the daemon overlays it from the catalog (like `activity`). Only `id` is stored.

## StoredSession (existing, extended)

`origin: Option<SessionId>`, `#[serde(default, skip_serializing_if = "Option::is_none")]`.
`SCHEMA_VERSION` stays 1; a file without the key loads with every session top-level (FR-015).

## SessionSummary (wire, extended)

`origin: Option<SessionOrigin>` (`#[serde(default)]`); protocol version 41.

## Settings (extended)

`folded_origins: BTreeSet<SessionId>`, `#[serde(default)]`; `settings_version` does not move.

## SessionTree (derived, never stored; `micold_core::origin`)

Input: a worktree's sessions in catalog order. Output nodes: `Root` (no origin), `Child` (origin in
the input), `Orphan { Pending | Removed }` (origin not in the input; shown top-level).

Rules (each is a test):
- a session appears exactly once (FR-008);
- an origin that is itself nested yields depth 2+ (US1.2);
- an input that contains a cycle (should not occur, R4) breaks it at the first repeat and shows
  the repeated member as a root, so the build always terminates;
- a node's children keep catalog order; roots keep catalog order (the user's sessions are not
  reordered).

## Validation (`validate_origin`)

| Check | Result |
|---|---|
| candidate == child | rejected: self |
| candidate in other project/worktree | rejected: cross-worktree |
| chain from candidate reaches child | rejected: cycle |
| child already has origin | not replaced; `Ok(false)` |
| candidate unknown (provider reader only) | accepted as pending; re-checked when it appears, cleared if rejected |

## State transitions

`None` -> `Known(id)` once. A provider-reported pending origin that fails the full check when it
appears goes back to `None` (cleared once; never re-recorded; the only allowed change, and not a replacement: FR-010). `Known(id)` -> `Removed` when the origin is archived (deleted); never
back to `None`, never another id. Stop/resume of either session changes nothing.
