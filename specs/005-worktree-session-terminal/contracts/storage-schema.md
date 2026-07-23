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
| `archived` | bool | Closed via FR-015a (bugfix BUG-003). `#[serde(default)]` — absent/`false` for a live session. **Not authoritative**: a fast in-memory convenience for sidebar filtering when the store is intact. The durable source of truth is the provider-side marker file (`contracts/claude-cli.md` "Durable close/remove suppression marker") — reconciliation (FR-020b/FR-020c) checks *that*, not this field, precisely so suppression survives this field (and the file it lives in) being lost. |

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

~~**Decision: A** — extend `projects.json` by adding an optional `sessions` array to each stored
project. Unknown-field tolerance already exists (serde `default`), so older files load unchanged;
reuses the established forward-compatible `StoredProject` pattern and a single atomic write. No
`schema_version` bump is required (adding an optional array is forward-compatible under the
current rules; bump only if the shape of existing fields changes).~~

**Superseded (bugfix 002/BUG-001, 2026-07-21)**: embedding sessions directly in the shared catalog
file meant a single store-level fault destroyed every open project's sessions at once (reported as
BUG-001 in `specs/002-project-workspace-management/bugs/`). Sessions now live in the **per-project
state file** introduced by that bugfix's storage split — same forward-compatible shape (`id`,
`worktree_dir`, `title`, plus `mode` per 010-regular-terminal-mode), addressed by the project's
stable id instead of nested under the catalog's `projects[]` entries. A fault reading/writing one
project's state file no longer affects the catalog or any other project. See
`specs/002-project-workspace-management/contracts/storage-schema.md` "Bugfix: per-project storage
split" for the file layout and migration path.

Option B (a sibling `sessions.json` mapping project path → sessions) — previously kept only as a
historical note — is, in spirit, the direction the per-project split above takes (one file per
project rather than one shared file), now that the stakes of a shared-fate file are understood.

## Reconciliation on project open (bugfix 002/BUG-001)

Isolating the fault (above) stops one project's storage problem from affecting others, but does
not by itself recover *that* project's own lost session records. FR-020b (spec.md) adds a
discovery pass: when a project is opened, scan the AI CLI provider's transcript directory for the
project's root directory and every discovered worktree —
`<claude-config-dir>/projects/<encoded-cwd>/*.jsonl` per the provider seam (`src/provider.rs`),
once per supported location. Each transcript filename is itself the session id (the id `claude`
was launched with via `--session-id`); any id with no matching entry in the (possibly
missing/corrupted) per-project session list is reconstructed as a `Session`:

- **id** — parsed from the transcript filename.
- **location** — `SessionLocation::Default` for the root directory, `SessionLocation::Worktree(dir)`
  for a worktree (010-root-dir-session).
- **title** — `parse_title` on the transcript contents if it yields one, else `SessionLabel::Pending`.

This is additive only — it never removes or overrides an existing persisted record, and a
transcript that already has a matching record is left untouched. It runs at the same I/O boundary
as `session_has_conversation` (`src/main.rs`), reusing the provider seam rather than adding a new
one.

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
- **(002/BUG-001)** A transcript found under a project's root directory or any worktree, with no
  matching persisted session record, is reconstructed as a session on project open; a transcript
  matching an existing record is not duplicated (FR-020b, SC-010).
- **(BUG-003)** A closed/removed session's transcript is never reconstructed by reconciliation,
  even when the app's own store (this file, or the catalog) has been deleted entirely between the
  close/remove and the next project open (FR-020c, SC-011).

## Constitution mapping

Principle IV (local-first, atomic, offline), II (sessions persisted + restorable).
