# Research: The start affordance answers for its own directory

Phase 0 of [plan.md](./plan.md). Every decision below names what was chosen, why, and what was
rejected. Code references are to `main` at `ac72d25f`.

## R1 — Reuse `AiCliAvailabilityRequest { req, cwd }` unchanged; correlate in the client

**Decision**: No protocol change. The client already sends
`ClientMsg::AiCliAvailabilityRequest { req, cwd }` (`crates/micold-core/src/protocol/messages.rs:531`)
and the daemon already answers it for `cwd`'s spawn environment
(`crates/micold-daemon/src/server.rs:659–676`, 029 FR-003b). The reply,
`DaemonMsg::AiCliAvailability { req, available }`, does not echo the directory, so the client keeps
a `req → directory` map for requests in flight and files each answer under the directory its request
named. `PROTOCOL_VERSION` and the schema-hash pins are untouched.

**Rationale**: The reply's `req` is already a unique correlation id (`app.next_req`), so the
directory is recoverable on the client with no wire change. The spec's Assumptions say the service
is reused as it is. Everything this feature changes is who asks and what is kept.

**Alternatives rejected**:
- *Echo `cwd` in `DaemonMsg::AiCliAvailability`.* It would make the reply self-describing, but it
  bumps `PROTOCOL_VERSION` (15 → 16), re-pins the schema hash, and forces a client/daemon pair
  upgrade (the pinned-pair check refuses mixed builds) for information the client already holds.
- *A batch request (`cwds: Vec<PathBuf>`) answered once.* Fewer messages on project open, but one
  slow environment-include script would hold up every other row's answer, which is the opposite of
  FR-005's "switch when its own answer arrives". It is also a new message and a version bump.

## R2 — The directory is the key, not the sidebar location

**Decision**: Answers are keyed by the resolved directory, `SessionLocation::cwd(project_root)`
(`crates/micold-core/src/session.rs:269`), the same function the daemon uses to spawn
(`crates/micold-daemon/src/state.rs:1215`). The home answer uses a separate key,
`AvailabilityKey::Home`.

**Rationale**: FR-006/FR-007 say rows for the same directory share one answer, and the answer must
be the one a spawn in that directory uses. Keying by the spawn's own `cwd` makes that true by
construction. A `SessionLocation` key would be ambiguous across projects: an answer for project A's
Default row, still in flight when the user switches to project B, would land on B's Default row.

**Cost at render**: one `PathBuf` join and one `HashMap` lookup per row. FR-006 allows "reading held
answers", and neither is an availability check or an environment resolution. The sidebar already
clones each row's `dir_name` per render.

**Alternatives rejected**: keying by `SessionLocation` (above); precomputing a `location → answer`
map on every change (a second cache to keep in step, to save a string join).

## R3 — Which directories are wanted: the rows that exist

**Decision**: The set of directories whose answer is held is derived from state, never listed by
hand. It contains the active project's root (the Default row) and one entry per **visible** worktree
row whose `can_start_session()` is true (`crates/micold-core/src/worktree.rs:368`). "Visible" means
`app::State::visible_worktrees()` (`crates/micold-client/src/features/worktree.rs:191`), which hides
agent worktrees unless the sidebar's reveal control is on (feature 029 FR-025). Each worktree maps to
its directory through `app::State::location_dir(&SessionLocation::Worktree(dir_name))`, the same
function the readers use. `Worktree::path` is never used as the key, because the two differ for
included worktrees. Only the active project's rows exist, since the sidebar shows only
`workspace.active` (`crates/micold-core/src/workspace.rs`).

One shell function, `sync_cli_availability`, computes that set. It drops held and in-flight answers
for directories not in the set (FR-003, FR-012) and asks for each directory that has neither an
answer nor a request in flight (FR-006, once per distinct directory).

**Rationale**: It turns the spec's list of "appear" and "disappear" events into one idempotent step
run after each of them:

| Spec event | Where the shell runs the sync |
|---|---|
| Project opened | `shell/workspace.rs` `open_verified_project` (after its `set_worktrees`) |
| Known project reopened, active project switched | `shell/workspace.rs` `on_known_project_reopened` (after its `set_worktrees`) |
| Project restored at launch | `shell/daemon_sync.rs` `on_connected`. The restore (`startup.rs:326`) runs before a connection exists, so asking then is a no-op, and the first connect is the first chance |
| Worktree added, discovered, deleted, included, excluded, or its directory gone (`WorktreeStatus::Missing`) or back | `shell/daemon_sync.rs`, the `DaemonMsg::CatalogChanged` arm, after `reconcile_catalog(.., true)` (`daemon_sync.rs:489`). The optimistic local list edits (`features::worktree::created`, `included`, `excluded`, the delete confirm) are not askers: each is followed by the daemon's catalog push, which "also arrives, and agrees" (`daemon_sync.rs:626`), and the sync runs there |
| Hidden agent worktrees revealed or hidden again | a new shell arm in `main.rs` for `SidebarMsg::ShowAgentWorktreesToggled`: apply the reducer, then sync. Revealing makes rows appear, so it is a first ask for them. Hiding removes rows, so their answers are dropped (FR-003) |
| Project forgotten | the `ProjectMsg::ForgetConfirmed` arm (`main.rs:636`) |

A worktree whose directory is deleted leaves the wanted set (it can no longer start a session), so
its answer is dropped. When the directory comes back, it re-enters the set and is asked for as a
first ask. That is FR-004's "deleted or recreated".

**Alternatives rejected**:
- *Every worktree of the project, hidden or not.* With agent worktrees hidden by default, opening a
  project would run the environment-include script once for every hidden agent worktree. Those rows
  do not exist, which contradicts FR-003's "only for rows that exist", SC-004's "N rows over D
  directories" and the "Many rows" edge case.
- *Run the sync after every message in `main.rs::update`.* It is robust against a missed call site,
  but it rebuilds the wanted set on every PTY output and hover message. That is per-event work
  growing with rows, which FR-006 and SC-003 are written against.
- *Ask only for the selected row.* Rejected by the user (spec Clarifications, D5).
- *Keep answers for every project ever opened.* FR-003 says "only for rows that exist", and FR-012
  says closing drops them.

**Residual, accepted**: the daemon's environment-include cache is invalidated when the application
deletes a worktree (`server.rs:1407`) and when environment-include settings change
(`state.rs:1008`), but not when a directory is deleted and recreated outside the application. A
worktree recreated that way is re-asked by the client, and the daemon may answer from its cache.
Its script output would normally be unchanged, and changing how the service caches is out of scope
(spec: "the service's per-directory environment cache is reused as it is").

## R4 — Stale answers: the newest request per directory wins

**Decision**: For each key, the client records the `req` of the latest request. An answer is kept
only when its `req` is that latest `req` for the key its request named. An answer whose `req` is
unknown is dropped: it was asked before a reconnect, or for a directory pruned since. This replaces
the single window-wide `App::cli_availability_asked` (`main.rs:164`), which dropped every answer
older than the latest request *for any directory*. That violates FR-009's second half, because
answers for different directories must all be kept.

**Rationale**: FR-009 exactly. Resolutions run off the service loop (`spawn_blocking`), so a slow
first resolution for directory A can arrive after a cached answer for directory B asked later. The
old rule threw A's answer away, which with eager asks on project open would drop most of them.

**Alternatives rejected**: accepting the newest-arriving answer per directory (a cached re-ask
overtaken by a slow first ask would regress, the case 029 BUG-001 recorded); a sequence number per
directory on the wire (a protocol change for what `req` already gives).

## R5 — Reconnect: drop everything, re-ask home and every wanted directory

**Decision**: `on_connected` clears every held answer and every in-flight request (FR-011), then
asks for home (as today, `daemon_sync.rs:964`) and runs the sync. The sync re-asks every wanted
directory because nothing is held any more.

**Rationale**: FR-011 and the spec's edge case "reconnect to a different service": a restarted
sandbox may be a different image. Replies to requests sent on the old connection can no longer
arrive, and dropping the `req` map ensures nothing from it is misfiled.

**Consequence, accepted**: between connect and the home answer's arrival, no answer is held. The
affordance then behaves as it does before the first answer today: `start_intent` reports
`NothingAvailable`, and a primary press sends the stored default to the daemon, whose launch-time
check reports any missing CLI (026 FR-010). The same is true on `main` at first connect.

**Alternative rejected**: keep the home answer across a reconnect as the fallback. FR-011 says every
held answer is discarded, and a home answer from a container that is gone would be a leak of exactly
the kind FR-002 forbids.

## R6 — Environment-include change: refresh when the settings the answers were asked under change

**Decision**: `AvailabilityAnswers` records the environment-include settings its answers were asked
under (`asked_under`: enabled, script path, timeout). It is set on connect from the `Welcome`'s
settings and updated when a refresh is sent. The `DaemonMsg::SettingsChanged` arm
(`daemon_sync.rs:502`) compares the echoed values with `asked_under`, **not** with `App`'s
`env_include_*` fields. Only when they differ does it re-ask home and every wanted directory and
update `asked_under`. The held answers are kept until the new ones replace them (R4). A
`SettingsChanged` that changes only, say, the scrollback length asks nothing.

**Why not compare with `App`'s fields**: this window's own save writes them in `apply_save`
(`shell/persist.rs:295–297`) *before* the daemon's echo arrives. A diff against them would find
nothing changed for the window that saved, so SC-005's main case would never refresh.

**Rationale**: FR-004 names "a saved change to the environment-include settings", not every settings
save. `SettingsChanged` is the one place where both this client's own save (echoed back) and
another window's save (029 FR-011) arrive, and it arrives after the daemon has applied the change.
The daemon clears its whole environment-include cache in `set_env_include` *before* it broadcasts
(`crates/micold-daemon/src/state.rs:1008–1009`), so the re-asks resolve afresh. Keeping held answers
until they are replaced avoids a flash back to the home answer on every row.

**Alternatives rejected**:
- *Refresh in the local save path* (`persist.rs:305–346`). It misses another window's change, and it
  fires before the daemon has applied the change.
- *A flag set in `apply_save` and consumed by the echo.* It is still blind to other windows, and a
  save the daemon rejects would leave it set.
- *Drop all answers first.* Each row would briefly fall back to home, a visible flicker for no
  correctness gain.

## R7 — The start list asks for its own directory, as today

**Decision**: `SessionMsg::StartMenuOpened` keeps asking for the list's directory (`main.rs:666`).
The answer now lands under that directory's key instead of replacing the window-wide set. The list
items (`ui/mod.rs` `session_start_menu_items`) and the missing-default notice
(`features/session.rs` `start_menu_toggled`) read that directory's held answer. This covers FR-004's
"the user opening that row's start list" and FR-008's second half.

## R8 — Settings keeps the home answer

**Decision**: `Settings(Opened)` keeps asking with `cwd: None` (`persist.rs:196`). Its answer is
stored under `AvailabilityKey::Home`. The Settings view (`ui/mod.rs:294`) and the "not installed"
sentence read only that key. Home is also every row's fallback while its own answer is pending
(FR-005).

## R9 — Where the state lives, and the guards it must satisfy

**Decision**: A render-free type, `AvailabilityAnswers`, in `crates/micold-client/src/features/session.rs`,
replaces `session::State::available_providers: Option<CliAvailability>`. The shell mutates it only
through its methods. The `req` map moves into it from `App`. Consumers call directory-taking
readers on `session::State`.

**Rationale**: `features/` is where render-free decisions live and where state-level tests reach
them (`crates/micold-client/tests/features_session.rs`). The shell cannot be unit-tested without an
`App`, and the correlation rule (R4) is the part most worth testing in isolation. These guards scan
the source and must stay green:
- `tests/support/state_scan.rs`, which classifies every method called on a state path
  (`feature_write_isolation.rs` `every_method_called_on_state_is_classified`):
  - add `asked` and `answered` to `MUTATORS` (`clear` and `retain` are already there)
  - add `for_dir`, `home`, `unasked` and `known_clis` to `READERS`
  - remove `known_available`, which the rename leaves stale
- `tests/feature_write_isolation.rs` and `tests/root_state_is_shared.rs`
- `tests/cli_availability_comes_from_the_service.rs` (its vacuity check greps `shell/` for
  `available_providers = Some(`; update it to the new write)
- `tests/features_are_render_free.rs`

**Alternative rejected**: keeping the map on `App` beside `cli_availability_asked`. The readers in
`ui/` would then need the shell's state, which `ui/` must not reach.

## R10 — Proving "only on named events" and "no per-render check" (SC-003, FR-004, FR-006)

**Decision**: A source tripwire, `crates/micold-client/tests/availability_is_asked_only_on_named_events.rs`,
modelled on `tests/refresh_is_only_on_demand.rs`. It allowlists every non-comment line under
`crates/micold-client/src/` that names `ask_cli_availability`, `sync_cli_availability`,
`refresh_cli_availability` or `ClientMsg::AiCliAvailabilityRequest`, with one reason per line tied
to an FR-004 event. A stale allowlist entry fails the test too. The scan skips
`src/main_tests.rs` and every `#[cfg(test)]` module, because driving the request is how tests test
it. `refresh_is_only_on_demand.rs` instead allowlists its one inline test line. Here the test sites
are many and grow with every event test, so an allowlist of them would guard nothing. No such line may sit under `ui/`
(render) or name `Subscription` or `time::every`. The existing `tests/idle_subscriptions.rs`
continues to pin that idle schedules nothing.

**Rationale**: 029 SC-006 is verified structurally and SC-003 asks for the same. A behavioural test
can show that an event asks. Only a scan can show that nothing else does.

## R11 — Eager asks and SC-004's bound

**Decision**: No client-side throttling. Opening a project with D distinct startable directories
sends D requests, and the daemon resolves each directory's environment at most once into the cache
a spawn there reads (`state.rs` `env_include_vars_for`, keyed by `cwd`). Later redraws send nothing.

**Rationale**: SC-004 bounds this feature at D resolutions shared with spawns, which is met. The
pre-existing missing single-flight in that cache (029 BUG-001 follow-ups) can make a spawn that
races an in-flight eager ask resolve the same directory a second time. That second resolution is
the spawn's (feature 011's path), not this feature's, so SC-004 does not require fixing it, and the
spec leaves it out of scope.

**Alternative rejected**: serialising the eager asks client-side (one in flight at a time). One slow
script would delay every other row, against FR-005.
