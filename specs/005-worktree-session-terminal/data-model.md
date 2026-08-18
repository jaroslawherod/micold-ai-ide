# Phase 1 Data Model: Worktree & Session Navigation with Embedded Terminal

**Date**: 2026-07-15 | **Feature**: 005-worktree-session-terminal

Pure-core types (no iced, no `portable-pty`, no spawned processes). Enums make invalid states
unrepresentable (Constitution Principle V). Derived from the spec's Key Entities + Functional
Requirements. Persistence shapes are in [contracts/storage-schema.md](./contracts/storage-schema.md).

---

## Entity: Project (existing — extended by reference)

The existing `Project` (`src/project.rs`) remains the active-context identity. This feature
adds the rule that **only a git repository may be opened** (FR-001a). The existing
`Project.is_git_repo` (point-in-time from `FolderScanner`) is too loose for the open gate; the
open flow uses `Git::is_repo_root` (`git rev-parse --show-toplevel`, research R7). A project
owns a set of worktrees (discovered) and, transitively, sessions (persisted).

- Identity: canonical filesystem path (unchanged).
- Relationship: `Project 1 — * Worktree` (discovered), `Worktree 1 — * Session` (persisted).

## Entity: Worktree

An isolated workspace under `.claude/worktrees/<dir>` bound to a dedicated git branch. The
**source of truth is git** (`worktree list --porcelain`), not persistence — the model is
rebuilt on project open and after each mutation (FR-018).

| Field | Type | Notes |
|-------|------|-------|
| `dir_name` | `String` | Directory component under `.claude/worktrees/`, `${type}-${ticket}-${name}` (FR-006). Identity within a project. |
| `path` | `PathBuf` | Absolute path to the worktree directory. |
| `branch` | `String` | Bound git branch `${type}/${ticket}-${name}` (FR-006). |
| `status` | `WorktreeStatus` | Health on disk (FR-018a). |
| `sessions` | `Vec<SessionId>` | Sessions hosted by this worktree (0..N, FR-010a). |

```rust
/// Health of a discovered worktree (FR-018a). Enum, not bools, so the sidebar renders an
/// explicit state and session-start can be blocked at the type level (Principle V).
pub enum WorktreeStatus {
    /// Registered with git and its directory exists — fully usable.
    Valid,
    /// Registered with git but the directory is gone (deleted externally); git-prunable.
    Missing,
    /// A directory exists under `.claude/worktrees/` that git does not know as a worktree.
    Invalid,
}
```

**Validation / rules**:
- `dir_name` and `branch` are *derived*, never free-typed (see WorktreeNaming).
- Creation is rejected if `dir_name` collides with an existing worktree OR `branch` collides
  with an existing local branch (FR-009).
- Starting a session is disabled unless `status == Valid` (FR-018a).

**State transitions**: `(none) → Valid` on successful create; `Valid → Missing` when the dir
disappears; `Valid → Invalid` is not a transition (Invalid = orphan dir never registered),
surfaced only at discovery. Deletion/repair is deferred scope.

## Entity: WorktreeNaming (value object)

Pure mapping from form inputs to derived names (FR-006, contracts/naming.md).

| Field | Type | Notes |
|-------|------|-------|
| `type_` | `ConventionalType` | Selected from a fixed vocabulary (FR-005a). |
| `ticket` | `Option<String>` | Optional; slugified; omitted from output when absent (FR-005b). |
| `name` | `String` | Slugified; must be non-empty after slugify (FR-008). |

```rust
/// Conventional-Commits type vocabulary (FR-005a). Fixed defaults this version; the whole
/// naming ruleset is designed to become user-configurable later (FR-006a) — kept in ONE place.
pub enum ConventionalType { Feat, Fix, Chore, Docs, Refactor, Test, Build, Ci, Perf, Style }

pub struct WorktreeNaming { pub type_: ConventionalType, pub ticket: Option<String>, pub name: String }

pub struct DerivedNames { pub dir_name: String, pub branch: String }
```

**Derivation** (single source of truth, FR-006/006a):
- With ticket: `dir = "{type}-{ticket}_{name}"`, `branch = "{type}/{ticket}-{name}"`. The `_`
  boundary is in the directory only — see contracts/naming.md for why, and for the asymmetry it
  creates with feature 016's branch→directory inverse (BUG-003).
- Without ticket: `dir = "{type}-{name}"`, `branch = "{type}/{name}"`.
- `ticket`/`name` are slugified to `[a-z0-9-]` before assembly (FR-008).

**Validation** (`NamingError`, pure): `NoType`, `EmptyNameAfterSlug`, `InvalidBranchRef`
(fails git check-ref-format), plus a collision check performed against live git state at the
orchestration layer (`DuplicateDir` / `DuplicateBranch`).

## Entity: Session

A unit of work bound to a single worktree, associated with one embedded terminal. **Persisted**
so it survives restarts (FR-020).

| Field | Type | Notes |
|-------|------|-------|
| `id` | `SessionId` (UUID v4) | App-generated up front, passed to `claude --session-id` (R6). Stable identity + `--resume` handle. |
| `worktree_dir` | `String` | The hosting worktree's `dir_name` (binding). |
| `label` | `SessionLabel` | Sidebar label: `claude` `ai-title` when known, else placeholder (FR-011a). |
| `lifecycle` | `SessionLifecycle` | Runtime state (transient — never persisted). |

```rust
pub struct SessionId(pub uuid::Uuid);

/// Label shown in the sidebar. Extracted from `claude` (its session title), not user-entered
/// (FR-011a). Placeholder until the title is available; updated when it appears.
pub enum SessionLabel {
    /// No title yet from `claude`; show a neutral placeholder (e.g. "New session").
    Pending,
    /// The `claude`-provided session title (best-effort from the session JSONL, R6).
    Named(String),
}

/// Lifecycle of a session's `claude` process. Enum keeps "running but no process" and similar
/// invalid combinations unrepresentable (Principle V). Transient; not persisted.
pub enum SessionLifecycle {
    /// Persisted but no process running (e.g. after restart or project close). Reopen resumes it.
    Idle,
    /// `claude` process is being (re)launched.
    Starting,
    /// `claude` process is running; terminal is live.
    Running,
    /// Process exited/crashed; auto-restart pending (FR-022), attempt counter for the guard.
    Restarting { attempts: u8 },
    /// Auto-restart gave up after repeated quick failures (FR-022a); user may retry manually.
    Failed,
}
```

**Validation / rules**:
- `id` is unique per project (UUID collision negligible; enforced on insert).
- A session persists `id` + `worktree_dir` + last-known `label` title (FR-020); `lifecycle` and
  terminal buffers are never persisted (FR-021 — no scrollback replay).
- Multiple sessions per worktree allowed (FR-010a).

**State transitions** (session lifecycle):

```
        start                       process ready
Idle ─────────────► Starting ───────────────────► Running
 ▲                     ▲                              │
 │ project close/switch│ resume (--resume)            │ process exits/crashes
 │ (stop, FR-023)      │                              ▼
 └───────────────── Idle ◄── (reopen) ──  Restarting{attempts++} ──auto-restart──► Starting
                                                │
                                 attempts exceed guard (FR-022a)
                                                ▼
                                             Failed ──(manual retry)──► Starting
```

- **Start** (FR-010): `Idle → Starting → Running`, launch `claude --session-id <id>` (cwd =
  worktree). New sessions generate a fresh UUID first.
- **Switch** (FR-015/015b): changing the active session does NOT change any lifecycle — every
  other session stays `Running`.
- **Close/stop** (FR-015a): terminate the process, remove the session (and its persisted
  record) → gone from the sidebar.
- **Crash** (FR-022): `Running → Restarting{attempts+1} → Starting` via `claude --resume <id>`;
  after the guard limit within a short window → `Failed` (FR-022a).
- **Project close/switch** (FR-023): all of that project's sessions `Running/Starting → Idle`
  (processes stopped, records preserved). Reopen (FR-023a) restores them `Idle` and resumes on
  reopen; the crash-loop auto-restart does NOT apply to intentional stops.

## Entity: Embedded Terminal (runtime, gui-gated — not in persisted model)

The interactive surface for an active session. Modeled behind `TerminalBackend`
(contracts/terminal-backend-trait.md); the real impl wraps `portable-pty` + `alacritty_terminal`
`Term` (the VT grid). Not persisted (FR-021). One per session; only the active session's grid is
rendered, but all run concurrently (FR-015b, R5).

## Aggregate: per-project session set (persisted)

Added to the existing local JSON store (contracts/storage-schema.md): a mapping from a project
path to its list of persisted sessions (`id`, `worktree_dir`, last-known title). Worktrees are
**not** stored — they are re-discovered from git on open (FR-018). Availability/lifecycle are
recomputed, never persisted (consistent with the existing `Availability` handling in `store.rs`).

## Relationship summary

```
Project (git repo, active context)
  └── * Worktree  (discovered from git; Valid|Missing|Invalid)
        └── * Session  (persisted: id, worktree_dir, label title)
              └── 1 Embedded Terminal (runtime, gui, one claude process)
```
