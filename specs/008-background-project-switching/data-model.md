# Phase 1 Data Model: Background Project Switching

This feature adds **no persisted schema** (`projects.json` is unchanged) and **no new `SessionLifecycle` variant**. All additions are in-memory fields on the render-free core `State` plus two total lookup/derivation helpers on `Workspace`. "Backgrounded" is a *view* relationship (a `Running` session that is not its project's foreground), not a stored state.

## Existing entities (reused, unchanged)

- **Project** (`src/project.rs`): identity = canonical filesystem `path`; metadata `display_name`, `is_git_repo`, `availability: Availability`. The switcher lists these and reflects `availability`.
- **Session** (`src/session.rs`): `id: SessionId` (UUID), `worktree_dir`, `label`, `lifecycle: SessionLifecycle` (`Idle → Starting → Running → Restarting{attempts} → Failed`). `is_active()` = running/starting/restarting. `on_unexpected_exit()` applies the crash-loop guard (`MAX_RESTART_ATTEMPTS = 3`). **No changes** — but `stop_for_project_change()` is no longer called on a mere switch.
- **Workspace** (`src/workspace.rs`): `projects: Vec<Project>`, `active: Option<PathBuf>`, `sessions: BTreeMap<PathBuf, Vec<Session>>`. Invariant retained: `active`, when `Some`, references a `path` present in `projects`.
- **State** (`src/app.rs`): app core; holds `workspace`, `active_session: Option<SessionId>` (the single foreground session, rendered by `view()`), overlays, etc.
- **RuntimeTerminal / `App.terminals: HashMap<SessionId, RuntimeTerminal>`** (`src/main.rs`, gui-only): live PTY handles keyed by session id. Now **retained across switches**; still keyed globally by id, so no structural change.

## New in-memory state (core `State`)

| Field | Type | Purpose | Lifetime |
|-------|------|---------|----------|
| `foreground_by_project` | `BTreeMap<PathBuf, SessionId>` | Remembers which session was in the foreground for each project, to restore on return (FR-003). | In-memory; per app run. Not persisted (R2). |
| `restarted_while_inactive` | `BTreeSet<SessionId>` | Marks sessions auto-restarted while their project was inactive, pending a return notification (FR-011, SC-007). | In-memory; entries cleared when the user returns to the owning project. |
| `notice` | `Option<String>` | Transient banner text shown on return when a background restart occurred. | In-memory; set on switch-in, cleared on dismiss/next switch. |

### Validation / invariants

- `foreground_by_project[p]` MUST reference a session id present in `workspace.sessions[p]`; stale entries are ignored on restore and overwritten on the next switch-away. A referenced session that has been **closed** (`archived`) is treated as "no stored foreground" (fall back to first running session, else `None`). A session that merely stopped is still restored — see [BUG-001](./bugs/BUG-001.md) and FR-003a. The original rule treated any non-running session as no memory at all, which was reasonable while this feature's premise (sessions keep running in the background) held; it does not hold across a restart, since lifecycle is not persisted, and the rule then discarded the memory in exactly the case the user most notices.
- An id is inserted into `restarted_while_inactive` ONLY when its owning project ≠ `workspace.active` at restart time. It is removed when that project becomes active (its removal is what arms `notice`).
- `notice` is presentation-only; it never gates behavior and is safe to drop. It is cleared by `Message::NoticeDismissed` (user dismiss) or overwritten/cleared on the next `switch_active`.

## New core helpers (pure, unit-tested)

On `Workspace`:

- `find_session(&self, id: SessionId) -> Option<(&Path, &Session)>` — total lookup across **all** projects (owning project path + session). Backs project-aware crash handling (R3) and cwd derivation.
- `find_session_mut(&mut self, id: SessionId) -> Option<(PathBuf, &mut Session)>` — mutable variant for applying `on_unexpected_exit`.
- `running_session_count(&self, path: &Path) -> usize` — number of `is_active()` sessions for a project; backs the switcher's running-background indicator (FR-007, R6).

On `State`:

- `switch_active(&mut self, path: &Path) -> bool` — the pure heart of a switch, in strict order: (1) record the current (outgoing) `active_session` into `foreground_by_project[outgoing]` **before** any activation, so the outgoing project is captured rather than the incoming one; (2) `activate(path)` (existing `Workspace::activate` rules; returns `false` and leaves state unchanged if unavailable — FR-008); (3) restore `active_session` for the incoming project (stored foreground → first running → `None`); (4) if any incoming session is in `restarted_while_inactive`, set `notice` and clear those ids. Does **not** stop or mutate any session's lifecycle (FR-001/FR-002). **Ordering caveat**: callers that mutate `workspace.active` themselves before switching (e.g. the `FolderChosen` handler calls `Workspace::open_or_activate` first) MUST capture the previous active path and hand the outgoing foreground in explicitly, or `foreground_by_project` records the wrong project.
- `note_background_restart(&mut self, id: SessionId)` — insert into `restarted_while_inactive` when the restarted session's project ≠ active.

## State transitions

### Project switch (was: destructive; now: non-destructive)

```
Before:  switch(P) => kill all terminals; outgoing sessions -> Idle; active_session = None; activate(P)
After:   switch(P) => record foreground_by_project[current] = active_session   # STEP 1, before activation
                      activate(P)                              # STEP 2: unavailable -> reject, no change (FR-008)
                      active_session = restore_foreground(P)   # STEP 3: stored | first-running | None
                      if incoming sessions ∩ restarted_while_inactive: set notice; clear those ids  # STEP 4
         (no session lifecycle change; terminals map untouched)
         # Order is load-bearing (I1): step 1 must read `current` BEFORE step 2 changes it. Any caller
         # that pre-activates (FolderChosen -> open_or_activate) must supply the outgoing path explicitly.
```

### Background session unexpected exit (poll loop, now project-aware)

```
detect exit(id) in terminals (any project)
  owner = workspace.find_session_mut(id)            # across ALL projects (R3)
  if owner.lifecycle == Idle: skip                  # intentional stop, not a crash
  decision = owner.on_unexpected_exit()             # crash-loop guard, MAX_RESTART_ATTEMPTS
  if decision == Resume:
      respawn PTY (LaunchMode::Resume) in owner cwd; reinsert into terminals
      if owner_project != active: restarted_while_inactive.insert(id)   # FR-011
  else (Failed): leave Failed; visible in sidebar on return
```

### Foreground vs background (per project, no new enum)

```
A session is "foreground"  iff  its id == State.active_session (and its project is active).
A session is "background"  iff  it is is_active() but not the foreground of its (active or inactive) project.
Switching projects changes which sessions are foreground/background; it never changes lifecycle.
```

## Data flow into the switcher (view)

The `ProjectSwitcher` renders purely from `State`:

- rows ← `workspace.projects` (name, `availability`)
- active marker ← `workspace.active`
- running count per row ← `workspace.running_session_count(path)` (R6; not from the gui `terminals` map)
- "Add project…" row ← emits `Message::ProjectSelectorOpened`
- row select ← emits `Message::KnownProjectReopened(path)` (reused; drives `switch_active`)
