# Phase 0 Research: Forget a Project

All Technical Context items resolved from the existing codebase — no external unknowns. This
records the design decisions that shape Phase 1, each grounded in an established pattern in the
repo so the feature stays consistent with features 002 (workspace) and 008 (worktree delete).

## R1 — Where the removal logic lives (pure core vs. binary)

- **Decision**: Add a pure `Workspace::forget(&Path)` in `src/workspace.rs` that removes the
  project record and all metadata keyed by its path. Reducer arms in `src/app.rs` drive the
  confirm/cancel overlay and call it. All I/O side effects (killing live PTY processes,
  persisting the catalog) stay in `src/main.rs`.
- **Rationale**: This is exactly the split the codebase already uses. `WorktreeDeleteConfirmed`
  in `app.rs` drops records in the pure core while `main.rs` first kills processes and then calls
  `persist(&State)`. Following it keeps the removal logic unit-testable (Principle I) and confines
  the untestable process/FS glue to the binary (the Principle I GUI-wiring exception).
- **Alternatives considered**: Perform removal directly in `main.rs` — rejected: it would put
  decision logic (active-pointer clearing, metadata cleanup) outside the tested core, violating
  Principle I. A `store`-level delete — rejected: the store round-trips the whole `Workspace`;
  there is no per-entry delete API and none is needed.

## R2 — Signature and semantics of `Workspace::forget`

- **Decision**:
  ```rust
  /// Remove a known project and all application-stored metadata keyed by its path
  /// (feature 014). Non-destructive to disk. If the forgotten project was the active
  /// working space, the active pointer is cleared (FR-008). No-op if the path is unknown.
  pub fn forget(&mut self, path: &Path) {
      let path = canonicalize_best_effort(path);
      self.projects.retain(|p| p.path != path);
      self.sessions.remove(&path);
      self.worktree_names.remove(&path);
      if self.active.as_ref() == Some(&path) {
          self.active = None;
      }
  }
  ```
- **Rationale**: Canonicalizing the lookup path matches `open_or_activate`/`activate`/`rename`, so
  identity is consistent (FR of feature 002). Removing `sessions[path]` and `worktree_names[path]`
  satisfies FR-005 (discard all metadata). Clearing `active` preserves the documented invariant
  ("`active`, when `Some`, references a `path` present in `projects`") and satisfies FR-008.
  Unknown-path → no-op mirrors `rename`'s tolerant behavior. Returns `()`; the reducer decides
  `active_session` clearing (see R4), and the binary reads the running-session list *before*
  calling the reducer (see R5).
- **Alternatives considered**: Return the removed `Vec<SessionId>` so the binary need not query
  first — rejected: the binary must capture live session ids *before* the reducer mutates state
  anyway (same as `WorktreeDeleteConfirmed`, which uses `sessions_in_worktree` pre-mutation), so a
  return value adds nothing. Keep empty `worktree_names[path]`/`sessions[path]` maps — rejected:
  leaves stale keys; `clear_worktree_name` already prunes empty maps, so full removal is
  consistent.

## R3 — Confirmation flow (messages + overlay)

- **Decision**: Three new `Message` variants and one new `Overlay`, mirroring `WorktreeDelete*`:
  - `Message::ProjectForgetRequested(PathBuf)` — reducer stores `forget_target = Some(path)` and
    opens `Overlay::ConfirmForgetProject`.
  - `Message::ProjectForgetConfirmed` — reducer calls `workspace.forget`, clears
    `active_session` if the forgotten project was active, clears `forget_target`, closes overlay.
  - `Message::ProjectForgetCancelled` — reducer clears `forget_target`, closes overlay, no change.
  - `State.forget_target: Option<PathBuf>` holds the pending target (parallel to
    `worktree_delete_target: Option<String>`).
- **Rationale**: Reuses the exact request→confirm/cancel pattern already proven for destructive
  worktree deletion, including the `open_overlay` helper and the overlay-dismiss wiring (the
  Escape-to-cancel map at the bottom of `app.rs`). Confirmation is mandatory (FR-002).
- **Alternatives considered**: A generic reusable "confirm" overlay carrying a callback —
  rejected: iced messages are plain data (no boxed callbacks in this codebase's reducer), and the
  worktree-delete precedent is a dedicated overlay+target pair. Consistency wins.

## R4 — Clearing the active session when the active project is forgotten

- **Decision**: In the `ProjectForgetConfirmed` reducer arm, capture
  `was_active = self.workspace.active.as_deref() == Some(&canonical_target)` *before* calling
  `forget`; after `forget`, if `was_active` then set `self.active_session = None`.
- **Rationale**: `active_session: Option<SessionId>` only ever references a session of the active
  project. Forgetting the active project makes `workspace.active` `None` (R2), so the dangling
  `active_session` must be cleared too — otherwise the shell would try to render a session for a
  project that no longer exists. This mirrors `WorktreeDeleteConfirmed`, which clears
  `active_session` when the removed session was the active one. Forgetting a *non-active* project
  leaves `active`/`active_session` untouched (that session belongs to the still-active project).
- **Alternatives considered**: Always clear `active_session` — rejected: wrong when forgetting a
  background (non-active) project, which must not disturb the foreground session.

## R5 — Stopping running sessions (binary side effect)

- **Decision**: In `main.rs`, handle `ProjectForgetConfirmed` by, *before* delegating to the
  reducer, collecting the target project's live session ids and killing their processes:
  ```rust
  Message::ProjectForgetConfirmed => {
      if let Some(path) = app.core.forget_target.clone() {
          for id in app.core.session_ids_of_project(&path) {   // new pure helper on State
              if let Some(mut st) = app.terminals.remove(&id) {
                  st.kill_all();                               // AI CLI + shell processes
              }
          }
      }
      app.core.update(Message::ProjectForgetConfirmed);        // pure record drop
      persist(&app.core);                                      // FR-007 immediate persist
      Task::none()
  }
  ```
  Add a small pure helper `State::session_ids_of_project(&Path) -> Vec<SessionId>` (reads
  `workspace.sessions[canonical(path)]`), analogous to `sessions_in_worktree`.
- **Rationale**: `app.terminals` is keyed by `SessionId`; `kill_all()` terminates both the AI CLI
  and shell processes of a session (same call used by `WorktreeDeleteConfirmed` and
  `SessionCloseRequested`). Killing *all* recorded sessions of the project (not only "running"
  ones) is safe — `terminals.remove` is a no-op for idle/absent sessions — and guarantees no
  orphaned process survives (FR-010). Worktree directories/files are never removed here, so
  forget stays non-destructive (FR-006). Persisting after the reducer satisfies FR-007.
- **Alternatives considered**: Kill only sessions where `is_active()` — rejected: iterating all
  recorded ids and letting `terminals.remove` filter is simpler and equally correct. Persist
  inside the reducer — rejected: persistence is I/O and must stay in the binary (Principle IV
  boundary already established by `persist`).

## R6 — Confirmation dialog content and running-session count (FR-002a)

- **Decision**: New view `src/ui/confirm_forget.rs` built on the shared `ui::material::Modal`
  (as `confirm_delete.rs` is). It receives the project's display name, its availability, and its
  running-session count, and renders:
  - Title: `Forget "<display name>"?`
  - Body line 1: states that only the remembered entry is removed and **nothing on disk** (the
    folder, its files, or any git worktrees) is deleted (FR-002).
  - Body line 2 (conditional): when running-session count `n > 0`, `This will stop n running
    session(s).` (FR-002a); omitted entirely when `n == 0`.
  - Actions: **Forget** (`ProjectForgetConfirmed`, filled/danger style) and **Cancel**
    (`ProjectForgetCancelled`, outlined) — same button styling split as `confirm_delete.rs`.
  The count is computed at render time via `Workspace::running_session_count(path)` — no count is
  stored in `State`.
- **Rationale**: `running_session_count` already exists and counts `is_active()` sessions — the
  precise set that will be "stopped" and shown to the user. Computing at render keeps `State`
  minimal and always accurate. Reusing `Modal` satisfies Principle VIII. The danger-styled
  confirm button matches the existing destructive-action affordance.
- **Consistency guarantee (SC-005a — count shown == count stopped)**: "running sessions" is
  defined as active (`is_active()`) sessions throughout. The confirmation shows
  `running_session_count(path)`. The binary stop-loop (R5) iterates *all* recorded
  `session_ids_of_project(path)` and calls `terminals.remove(id).kill_all()`, but a session has a
  live PTY in `terminals` **iff** it is active — an idle/absent session's `terminals.remove` is a
  no-op that stops nothing. So the number of processes actually stopped equals the active count
  shown: the displayed metric and the acted-upon set coincide by construction, satisfying SC-005a
  with 0 mismatches.
- **Alternatives considered**: Precompute and store the count in `forget_target` — rejected:
  redundant state that could drift; render-time computation is a pure read.

## R7 — Placement of the Forget control in the known-projects list

- **Decision**: Add a **Forget** button to each entry row in `src/ui/shell.rs`, after the existing
  **Rename** button, using `Icon::Delete` (the existing trash glyph) with an outlined/danger
  style and dispatching `Message::ProjectForgetRequested(project.path.clone())`. The button is
  present for **every** entry, including `Unavailable` ones (FR-011) — unlike **Open**, which is
  disabled when unavailable.
- **Rationale**: `Icon::Delete` already exists and is documented as the trash action; no new icon
  is needed (Principle VIII reuse). The known-projects loop already builds per-entry `Open` and
  `Rename` buttons, so adding a third is a local, consistent change. Rename is already allowed for
  unavailable entries; Forget is the primary way to clear a stale unavailable entry, so it must be
  enabled there too.
- **Alternatives considered**: An overflow/kebab menu per row — rejected: the list uses flat
  inline buttons today; introducing a menu here would fork a new interaction pattern for one
  action. A swipe/hover-reveal affordance — rejected: not used anywhere in this app.

## R8 — Persistence and restart behavior (FR-007, FR-012)

- **Decision**: Reuse the existing `persist(&State)` path after the reducer drops the record; no
  store API change. Because `forget` removes the record from the in-memory `Workspace` and
  `persist` saves the whole catalog, the forgotten project is absent from `projects.json` and does
  not reappear on restart. Re-opening the same folder later runs `open_or_activate`, which finds
  no matching path and creates a fresh record with the default display name (FR-012) — no special
  handling required.
- **Rationale**: `persist` already prunes empty sessions and writes atomically; the removed
  project simply is not in the saved set. `open_or_activate`'s existing "create if absent" branch
  gives the fresh-entry behavior for free.
- **Alternatives considered**: A tombstone list to suppress re-adds — rejected: contradicts FR-012
  (re-opening SHOULD create a fresh entry) and adds needless state.

## R9 — Test strategy (Principle I)

- **Decision**:
  - **Unit (`tests/workspace.rs`)**: `forget` removes the record, `sessions[path]`, and
    `worktree_names[path]`; clears `active` + returns list to empty state when the forgotten
    project was active; leaves `active` and other projects/sessions intact when a non-active
    project is forgotten; is a no-op for an unknown path; forgets an `Unavailable` project the
    same as an available one.
  - **Integration (`tests/forget_project.rs`)**: drive the reducer — `ProjectForgetRequested`
    opens the overlay and sets `forget_target`; `ProjectForgetCancelled` restores prior state
    (no removal); `ProjectForgetConfirmed` removes the entry and, when it was active, clears
    `active_session` and leaves no active project; forgetting the last project yields the empty
    state (`projects.is_empty()`).
  - **Persistence (`tests/store_roundtrip.rs`)**: save after forget, reload, assert the forgotten
    project is absent and the survivors + active pointer are intact.
  - **Quickstart (manual)**: the `main.rs` process-kill + persist glue and the modal rendering,
    per the Principle I GUI-wiring exception.
- **Rationale**: Puts every decision branch (active vs. non-active, available vs. unavailable,
  confirm vs. cancel, persistence) under an automated test written first, leaving only untestable
  glue to the quickstart — the exact division the constitution's Principle I exception allows.
- **Alternatives considered**: Only integration tests — rejected: the pure `forget` invariants are
  cheapest and clearest as direct unit tests on `Workspace`.

## R10 — Per-project state file deletion & reconciliation interplay (added after rebase onto `main`)

- **Context**: `main`'s `fix/state-lost` work split persistence so each project's sessions/overrides
  live in their own file `JsonFileStore::project_state_path(project_path)` (a `projects/<id>.json`
  next to the catalog), and added session **archiving** + **reconciliation** with durable
  suppression markers so closed sessions are not resurrected. `store.rs::save` rewrites a state
  file only for projects **currently in the catalog** (`for project in &workspace.projects`), and
  `store.rs::load` reads a state file only for catalog projects.
- **Decision**: On forget, in addition to pruning the in-memory `Workspace` and persisting the
  catalog, **delete the forgotten project's per-project state file**. Add
  `JsonFileStore::remove_project_state(&self, project_path: &Path) -> io::Result<()>` (remove the
  file at `project_state_path`; treat "not found" as success). The `main.rs`
  `ProjectForgetConfirmed` handler calls it after `persist`.
- **Rationale**:
  - **FR-005 (discard persisted records)**: because `save` skips non-catalog projects, a plain
    forget would leave the project's state file orphaned on disk with its old sessions — the
    persisted session records would *not* be discarded. Explicit deletion satisfies FR-005 to the
    letter.
  - **FR-012 (fresh re-open, clean slate)**: if the folder is re-opened later, `load` would
    otherwise find and restore the stale state file's *remembered session metadata* (ids, titles,
    modes, archived flags) for the "fresh" entry. Deleting the file guarantees the re-opened entry
    retains none of the app's prior stored metadata for that path.
  - The per-project state file lives in the **app data directory**, not the project folder, so
    deleting it is discarding app metadata (FR-005), never a modification of the project on disk
    (FR-006).
- **On-disk conversations are NOT resurrection (FR-012 boundary)**: forget does not delete the
  worktrees' on-disk `claude` conversations (FR-006). On re-open, normal reconciliation MAY
  rediscover those conversations — exactly as opening any folder that already has conversations on
  disk would. That is the current on-disk reality, not retention of the forgotten entry's
  remembered metadata, and is consistent with FR-012 (see the FR-012 note in spec.md). Deleting the
  state file removes the *metadata-based* restore path; it does not (and must not) suppress
  first-open rediscovery of surviving on-disk conversations.
- **Reconciliation runs against the active project only**: a forgotten project is neither active
  nor in the catalog, so within the running session it is never a reconciliation target. It
  therefore needs **no** per-session `archive()` step — unlike worktree-delete, which archives
  sessions because their project stays cataloged (R5-adjacent). Whole-project removal makes the
  simpler no-archive path safe.
- **`persist` signature**: `main`'s refactor changed `persist` to `fn persist(core: &mut State)`
  (was `&State`). The handler uses `persist(&mut app.core)` accordingly; no behavior change to the
  forget logic.
- **Alternatives considered**: Rely on the next `save` to overwrite the file after re-open —
  rejected: it leaves an orphaned file until (and unless) the folder is re-opened, and does not
  satisfy FR-005's "discard persisted session records" at forget time. Have the core `Workspace`
  own file deletion — rejected: `Workspace` is pure/no-I/O; file deletion belongs at the
  `store`/binary boundary (Principle IV).

## R9-supplement — Test for state-file deletion

- Extend `tests/store_roundtrip.rs`: after `save` with a project that has sessions (so its state
  file exists), call `remove_project_state(path)` (or forget-then-save flow), then assert the
  `project_state_path` no longer exists and a fresh `load` yields no sessions for that path. This
  is pure store I/O against a `tempdir`, so it stays an automated test (not quickstart).
