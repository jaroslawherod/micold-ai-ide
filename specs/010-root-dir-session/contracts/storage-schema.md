# Contract: Session Persistence Schema (extension)

**Feature**: 010-root-dir-session | extends `src/store.rs` and
`specs/005-worktree-session-terminal/contracts/storage-schema.md`.

Widens the existing per-project `sessions` array (`projects.json`, via `directories`,
atomic temp-file + rename) so a persisted session can record "no worktree" without a
schema version bump.

**Superseded placement (bugfix 002/BUG-001, 2026-07-21)**: the `sessions` array this contract
widens no longer lives embedded in the shared `projects.json` catalog — it lives in the
per-project state file from that bugfix's storage split, so a fault in one project's file can't
wipe every project's sessions. The `worktree_dir: Option<String>` field change and its
`SessionLocation::Default`/`Worktree(dir)` semantics below are unaffected. Additionally, this
feature's `SessionLocation::Default` (the project root) is one of the two locations scanned by the
session-reconciliation fix in `specs/005-worktree-session-terminal/spec.md` FR-020b — when a
project opens, the root directory's transcript directory is scanned alongside every worktree's, so
a Default session's conversation is recoverable even if its persisted record is lost.

## Field change

| Field | Old type | New type | Meaning |
|---|---|---|---|
| `worktree_dir` | `string` | `string \| null` | `string` ⇒ hosted by that worktree's `dir_name` (unchanged meaning). `null`/absent ⇒ hosted by the project root ("Default", `SessionLocation::Default`). |

`id` and `title` are unchanged from the 005 contract.

## Backward compatibility

- Every session persisted before this feature has `worktree_dir` as a JSON string.
  `serde_json` deserializes a present string value into `Option<String>` as `Some(..)`
  without any migration code — old files load with identical `SessionLocation::Worktree`
  results.
- No `schema_version` bump: per the 005 contract's own placement-decision precedent
  ("adding optionality is forward-compatible under the current rules"), widening a field
  to `Option<T>` where every existing value is a valid `Some(T)` is likewise
  forward-compatible — nothing that could load before fails to load after.
- A hand-edited or externally-corrupted file with `"worktree_dir": null` for what should
  be a worktree session simply restores as a Default session for that project — no crash,
  consistent with this store's existing "missing/corrupt → recover, never panic" policy
  (`LoadStatus::Recovered`).

## Save

`Session::location` maps directly:

```text
SessionLocation::Worktree(dir) → StoredSession { worktree_dir: Some(dir), .. }
SessionLocation::Default       → StoredSession { worktree_dir: None,     .. }
```

The BUG-001 "empty sessions are excluded" rule (005 contract) is unchanged and applies
identically to Default sessions: a Default session with no `claude` conversation
transcript MUST NOT be persisted, using the same transcript-presence check — the
transcript path is derived from the session's *actual resolved cwd* (the project root for
a Default session), which the existing encoding scheme (non-alphanumeric → `-`) already
handles for any path, worktree or not.

## Load / restore

1. `worktree_dir: Some(dir)` restores `SessionLocation::Worktree(dir)`; on project open,
   the existing rule applies unchanged — drop or flag if `dir` no longer resolves to a
   `Valid` worktree (FR-018a, 005 spec).
2. `worktree_dir: None` restores `SessionLocation::Default`. No discovery/validity check
   is needed or possible (there is no git-backed "Default validity" the way there is
   `WorktreeStatus`) — the project root's own existence is already the precondition for
   the project being open at all (spec Assumptions).
3. Reopening a Default session (`claude --resume <id>`) is otherwise identical to
   reopening a worktree session — same terminal-backend contract
   (`specs/005-worktree-session-terminal/contracts/terminal-backend-trait.md`), cwd
   argument only differs in value, not in how it's supplied.

## Guarantees (test targets)

- Roundtrip: save → load yields an identical persisted session set including Default
  sessions (extends SC-008 from 005).
- A pre-existing `projects.json` with only string `worktree_dir` values loads with zero
  Default sessions inferred (no accidental reinterpretation of old data).
- `worktree_dir: null` roundtrips to `SessionLocation::Default` and back to `null`
  (not `""` or an omitted key with inconsistent presence).

## Constitution mapping

Principle IV (local-first, atomic, offline, unchanged), Principle II (Default sessions
persisted + restorable, same as worktree sessions), Principle III as amended in v1.3.0
(the Default location is the one sanctioned non-worktree session location).
