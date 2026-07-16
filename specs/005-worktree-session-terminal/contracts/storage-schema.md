# Contract: Session Persistence Schema

**Feature**: 005-worktree-session-terminal | extends `src/store.rs`.

Local-first (Principle IV). Extends the existing per-user JSON store pattern
(`<data_dir>/…` via `directories`, atomic temp-file + rename, missing/corrupt → recover to
empty). **Worktrees are NOT persisted** — they are re-discovered from git on project open
(FR-018), exactly as `Availability` is recomputed rather than stored today. Only **sessions**
are persisted, so they restore across restarts (FR-020, FR-023a).

## What is persisted

Per project (keyed by the project's canonical path), a list of sessions:

| Field | Type | Notes |
|-------|------|-------|
| `id` | string (UUID) | The `claude` `--session-id`; stable `--resume` handle (FR-020). |
| `worktree_dir` | string | Hosting worktree's `dir_name` (binding). |
| `title` | string \| null | Last-known `claude` `ai-title` for the sidebar label (FR-011a). `null` = never seen yet → `Pending`. |

### Empty sessions are excluded (bugfix BUG-001)

A session for which `claude` has recorded **no conversation** (started but never used) MUST NOT
be persisted. On save, such sessions are filtered out; on load, any that slipped in are pruned.
"Has a conversation" is determined by the presence of the session's `claude` transcript at
`<claude>/projects/<encoded-cwd>/<session-id>.jsonl`, where `<encoded-cwd>` is the worktree path
with every non-alphanumeric char replaced by `-` (honoring `CLAUDE_CONFIG_DIR`). This is a
filesystem check performed at the I/O boundary (the binary), not in the pure store. Guarantees a
restart never resumes a nonexistent conversation (FR-020/FR-020a).

## What is NOT persisted

- `SessionLifecycle` and terminal buffers/scrollback (FR-021 — no replay; recomputed at runtime).
- Worktree list / status (re-discovered from git, FR-018).
- Any `claude` conversation content (owned by `claude` under `~/.claude`, not by this app).

## Placement decision

**Decision: A** — extend `projects.json` by adding an optional `sessions` array to each stored
project. Unknown-field tolerance already exists (serde `default`), so older files load unchanged;
reuses the established forward-compatible `StoredProject` pattern and a single atomic write. No
`schema_version` bump is required (adding an optional array is forward-compatible under the
current rules; bump only if the shape of existing fields changes).

Option B (a sibling `sessions.json` mapping project path → sessions) is kept only as a historical
note; it is not the chosen shape.

## Load / restore behavior

1. Load catalog (existing flow) → sessions attached to each project as `Idle`, `title` → label.
2. **Prune empty sessions on load** (bugfix BUG-001): drop any restored session with no `claude`
   conversation transcript, so leftovers from before this fix are cleaned and never resumed.
3. On project open: discover worktrees from git; drop persisted sessions whose `worktree_dir`
   no longer resolves to a `Valid` worktree (or keep them under a `Missing` worktree, flagged —
   consistent with FR-018a; decide in tasks.md).
4. Reopening a session (FR-023a) resumes it via `claude --resume <id>` (terminal-backend).

## Guarantees (test targets)

- Roundtrip: save → load yields identical persisted session set (SC-008).
- Missing/corrupt store → empty, no crash (existing `LoadStatus::Recovered` behavior reused).
- A `title` of `null` restores as `SessionLabel::Pending`.
- Empty sessions (no `claude` conversation) are excluded on save and pruned on load (BUG-001).

## Constitution mapping

Principle IV (local-first, atomic, offline), II (sessions persisted + restorable).
