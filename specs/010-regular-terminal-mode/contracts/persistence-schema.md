# Contract: `StoredSession.mode` (persistence)

Governs FR-011 ("a session's current mode MUST be persisted and restored"). Extends the schema
documented in `specs/005-worktree-session-terminal/contracts/storage-schema.md`.

**Superseded placement (bugfix 002/BUG-001, 2026-07-21)**: `StoredSession` (and its `mode` field
below) no longer lives embedded in the shared `projects.json` catalog — it lives in the
per-project state file from that bugfix's storage split, so a fault in one project's file can't
wipe every project's sessions and modes at once. The schema delta and round-trip behavior below
are otherwise unchanged.

## Schema delta

```rust
// src/store.rs
struct StoredSession {
    id: uuid::Uuid,
    worktree_dir: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    mode: StoredTerminalMode,   // NEW
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
enum StoredTerminalMode {
    #[default]
    AiCli,
    Regular,
}
```

- **Backward compatibility**: `#[serde(default)]` means a catalog file written by any prior
  feature (no `mode` key at all) deserializes with `mode: StoredTerminalMode::AiCli` — every
  existing session silently starts this feature attached to the AI CLI, i.e. today's only
  behavior. No `schema_version` bump (matches the precedent set by `sessions` in feature 005 and
  `worktree_display_names` in feature 008 — both additive, both `#[serde(default)]`, neither
  bumped the version).
- **Forward compatibility**: not a concern here (no older-binary-reads-newer-file requirement
  has ever been established for this project).

## Round-trip

- `StoredCatalog::from_workspace`: `mode: session.mode.into()` (a `From<TerminalMode> for
  StoredTerminalMode` mapping, or an equivalent explicit match — kept as a separate type rather
  than deriving `Serialize`/`Deserialize` directly on the pure-core `TerminalMode` so the
  persistence *shape* can evolve independently of the pure-core enum, mirroring how
  `SessionLabel`/`title` are already two different shapes for the same concept).
- `StoredCatalog::into_workspace`: `Session::restored(id, worktree_dir, label, stored.mode.into())`.

## What is explicitly NOT persisted

- `Session.shell_lifecycle` — runtime-only, mirrors `Session.lifecycle`'s existing
  non-persistence (`session.rs`'s own doc comment: "Runtime state... Never persisted").
- Shell process output/scrollback — mirrors the AI CLI `Term`'s existing in-memory-only
  scrollback (feature 006).
- Whether a shell process was ever started for a session — on restart, both processes begin
  `NotStarted`/`Idle`; only the persisted `mode` says which one the terminal *reopens showing*,
  and the reopening (`SessionSelected`-equivalent) path spawns exactly that one process
  (spec Assumptions: "OS processes cannot survive an application restart... 'restoring the
  last-used mode' means the terminal reopens in that same mode with a freshly (re)started
  process of the appropriate kind").
