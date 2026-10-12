# Contract: origin on the wire and on disk

**Catalog file** (`projects.json`, per project): each stored session may carry `"origin":
"<uuid>"`. Absent means top-level. Written only by the daemon's `Catalog`. A session may also carry
`"origin_rejected": true` (absent means false): a provider-reported origin that failed the full
check was cleared once and is not recorded again.

**Protocol 41**: `SessionSummary` gains
`origin: Option<{ id: SessionId, status: "known" | "removed" }>`. Carried by every message that
carries a `SessionSummary` (`SessionChanged`, snapshots), so every window derives one tree from the
same data (FR-012). No new message: a recorded origin is an ordinary `SessionChanged`.

**Removal**: when an origin is archived (deleted, by any of the catalog's three archive paths), the daemon emits a `SessionChanged` for each
direct child with `status: "removed"` in the same step; a client MUST NOT infer removal from an
origin id disappearing from its list.

**Settings file**: optional `"folded_origins": ["<uuid>", …]`; absent means none folded.

**Guarantees**: a validated origin is never replaced; a pending, unvalidated one may be dropped
once, and a later report cannot re-record it for that session; the daemon never emits an origin that is
the session itself or whose chain reaches it; a client MUST treat an origin id absent from its
snapshot as pending (status `known`) or removed (status `removed`) and show the session top-level.
