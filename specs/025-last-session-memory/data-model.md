# Data Model: Reopen on the session I was last using

Feature 025. One value, moved to a home where it can be written by the process that owns the file
and read by the one that needs it first.

## The memory

| | |
|---|---|
| **What** | For one project, the session that was most recently in front of the user |
| **Cardinality** | At most one per project; absent for a project no session has been used in |
| **Type** | `Option<SessionId>` — the absence *is* `None`, never a sentinel id (Principle V) |
| **Lifetime** | Persisted; survives restarts. Discarded when the project is forgotten (FR-009) |
| **Written by** | The daemon, on `SetViewedSession` — the single writer of the store |
| **Read by** | The client, from its own load at `boot()`, and by the switch path already |

## Moved: `foreground_by_project`

| Before | After |
|---|---|
| `micold_client::app::State::foreground_by_project: BTreeMap<PathBuf, SessionId>` | `micold_core::workspace::Workspace::foreground_by_project: BTreeMap<PathBuf, SessionId>` |
| In-memory, client-only, dies at exit | Persisted with the workspace it describes |

It moves rather than being copied. Two homes for one fact can disagree, and the one on `State` could
not be persisted at all without the client writing a file the daemon owns (research R2).

Its meaning does not change: keys are canonicalised project paths, exactly as `Workspace::sessions`
is keyed, so the two are looked up the same way. `record_foreground` and `explain_foreground` keep
their behaviour and change only where they read from.

## Persisted shape

`StoredProjectState` — the per-project file (`projects/<hash>.json`) that already holds that
project's sessions and worktree display-name overrides:

```text
StoredProjectState {
    schema_version: u32,
    sessions: Vec<StoredSession>,
    worktree_display_names: BTreeMap<String, String>,
    last_session: Option<SessionId>,        // NEW, #[serde(default)]
}
```

**Why here and not in `projects.json`**: it is per-project data about that project's sessions, and
this is the file those sessions live in. It is also the file `remove_project_state` already deletes
when a project is forgotten, which is FR-009 with no extra code.

**Why no `schema_version` bump**: `#[serde(default)]` makes an old file load with `None` — which
means "no memory", the behaviour the application has today. A new file read by an older build
carries one unknown field, which `serde` ignores. `store.rs` records this same argument for the
BUG-001 split (research R7).

### Invariants

1. **I0 — The memory only ever moves forward.** It is set by a session becoming current in that
   project and by nothing else. No event clears it: not closing the session it names, not an
   internal loss of the pointer, not a failed restore (FR-005a). Forgetting the project is not an
   exception — it deletes the file the memory lives in, along with that project's sessions (§2.5).
2. **I1 — A memory is a hint, never a promise.** `last_session` may name a session that no longer
   exists, was closed, or belongs to a worktree that is gone. Nothing validates it at load; it is
   resolved when used, by the resolution that already exists (I3).
3. **I2 — The client never writes it to disk.** It reads at load and mutates in memory for the
   current run; persistence happens daemon-side. This is the split `Workspace::sessions` already
   has, and it exists because `store.rs` has no locking.
4. **I3 — Usability is decided by `explain_foreground`, not by the store.** A remembered session is
   restored when it is present and not `archived`; otherwise the existing fallbacks apply. There is
   one implementation of that question (feature 008 FR-003a).
5. **I4 — Restoring never starts anything.** Applying the memory sets which session is displayed and
   nothing else (FR-004).

## Reading it at launch

```text
boot()
  └─ store.load()                      → workspace, sessions, and now the memory
  └─ prune_empty_sessions(workspace)   → a memory naming a pruned session stops resolving (FR-005)
  └─ if let Some(active) = workspace.active
       └─ restore_after_activation(active)     ← the same function a project switch calls
```

Nothing launch-specific in that sequence, which is the point. `restore_after_activation` resolves
the memory, applies it, and focuses the terminal — all three wanted here (research R5) — and
`set_current_session` inside it arms feature 024's reveal, so FR-012 comes free.

## Writing it

```text
ClientMsg::SetViewedSession { project, session }   (already sent on every path that changes it)
  └─ daemon: state.set_viewed(client, project, session)      (existing, per-client, in memory)
  └─ daemon: if session is Some AND differs from what is remembered:   (NEW)
                catalog records project → session, then persists
```

Two conditions, both from clarification:

- **`Some` only.** A `None` report never clears the memory (FR-005a, §2.6). The pointer goes to
  nothing for reasons the user did not take, and the restore already declines a session that cannot
  be shown — so a stale memory costs nothing and a lost one costs the user their place.
- **Only on a change.** Attach re-sends the current id and a session start may name the session
  already in front of the user; writing on those would rewrite a file holding every session record
  with identical content (FR-001a).

`SetViewedSession` is already sent on welcome/attach, forced re-attach, selecting or starting a
session, and switching projects (research R4). Nothing new is sent, and no message changes shape —
so no schema hash moves.

## Entity mapping to the spec

| Spec entity | Here |
|---|---|
| Last-used session | `Workspace::foreground_by_project[project]`, persisted as `StoredProjectState::last_session` |
| Project | The canonicalised path that keys both that map and `Workspace::sessions` |
| Session | `micold_core::session::Session` — unchanged; `archived` is what makes a closed one unrestorable |
