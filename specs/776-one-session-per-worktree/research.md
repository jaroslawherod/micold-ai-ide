# Research: One Session per Worktree, Chosen at Start

Findings come from reading the tree at `origin/main`. Paths are real; "new" marks what the
plan adds.

## R1. Where the rule lives and what "has a session" means

**Decision**: a new pure module `micold-core/src/occupancy.rs`. Input: the project's
`&[Session]` and a `SessionLocation`. A session counts when `location` matches and `!archived`
(the app's "delete"/close is `Catalog::archive_session`, which keeps the record hidden). Any
lifecycle counts (starting, running, restarting, idle, failed, exited, resumable). Output
`Occupancy::{Free, Taken { id, name }}`; `Taken` holds the first matching session (stable order),
so a legacy multi-session location names a deterministic one. `SessionLocation::Default` is an
ordinary key, which gives FR-011 for free.

**Rationale**: `Session.location` and `.archived` already exist (`session.rs`), so no schema
change and old catalogs load as they are. One function serves daemon and client (Principle V:
one decision, typed result).

**Alternatives rejected**:
- Count only live sessions: the spec fixes "exactly one is about records" and a failed start must
  not free the worktree.
- A `Worktree.session: Option<SessionId>` field: a second source of truth beside the sessions
  list, needing a migration and a repair path for disagreement.
- Count archived sessions: a closed session is hidden; counting it would make FR-010 ("its
  session deleted, takes a new one") unreachable.

## R2. Atomicity of check-and-create (FR-005, SC-003)

**Decision**: do the check inside `Catalog::create_session` (`catalog.rs:560`), which already
takes `&mut self` and is called under `DaemonState`'s state lock via `DaemonState::create_session`
(`state.rs:2726`). It returns `Result<SessionId, CreateSessionError>` where
`CreateSessionError::{Occupied(ExistingSession), Io(io::Error)}`. Every caller of
`DaemonState::create_session` and `ops::create_session_with_prompt` goes through it: server
`SessionCreate` (`server.rs:958`), MCP `create_session` (`mcp/tools.rs:1354`), review send
(`state.rs:2306`), parallel runs (`runs.rs:486`, a brand-new worktree, so always free).

**Rationale**: the catalog lock is the single serialization point all windows and agents already
share, so no new gate is needed, and a check made earlier (for example in the MCP handler) could
be raced.

**Alternatives rejected**:
- Check in the MCP handler and the RPC handler: two checks, and a race between them.
- A per-worktree async gate: more machinery than the lock that already covers the write.

## R3. The refusal and its wording

**Amended 2026-10-13 (R12)**: the text also names the way to ask for another. **Decision**: reuse `ErrorKind::Refused` (`messages.rs`, "refused by policy" doc extended) for the
RPC; MCP uses `OpError` category `Conflict` (as `create_derived_worktree` does for a taken
branch). One wording function `occupancy::refusal_text(&ExistingSession)` in core, used by both:
`this worktree already has a session: "<name>" (<id>); start_session on <id> brings it back`.
For Default the first words read `the project root already has a session`. Legacy start refusal
(R9) uses `session "<name>" (<id>) is running in this worktree; stop it first`. Both refusals
append the way forward (R12): tool callers `, or pass allow_additional_session: true to run another
on purpose`; the app words it as the confirmation.

**Rationale**: FR-003 needs id, name and the hint in one place that tests can pin; the client
shows the daemon's text (as it does for other `OperationError`s), so wording is not duplicated.

**Alternatives rejected**:
- A new `ErrorKind::SessionExists` carrying a struct: a wire change for the client to display
  text it would only format; the id is in the text and the agent reads text.
- `AlreadyExists`: documented as "the target already exists", which is the wrong claim (the
  target session does not).

## R4. Definition at worktree start (FR-006)

**Decision**: add `session: Option<SessionDefinition>` to `ClientMsg::WorktreeCreate` and
`session: Option<SessionId>` to `OperationResult::WorktreeCreated`, serde-defaulted (additive).
`SessionDefinition { ai_cli: AiCli, prompt: Option<String> }` is one type, so a half definition
cannot be sent. New `ops::create_worktree_with_session`: (1) `cli_unavailable` for the CLI at
the worktree's future `cwd` before anything is created (spec US2 s4); (2) `ops::create_worktree`;
(3) `create_session_with_prompt` (which starts fresh and types the prompt). If step 3 fails the
worktree stays and the error says so; the worktree is valid and has no session, which FR-010
allows, and rolling a created worktree back would destroy the user's step. `PROTOCOL_VERSION`
40 to 41.

**Model**: the spec's "model where the CLI takes one" is vacuous today: no session record, spawn
path or `AiCli` provider carries a model (`grep -- '--model'` over core and daemon finds
nothing). The definition is therefore AI CLI plus prompt. A CLI that later takes a model adds a
field to `SessionDefinition` and the spawn plan; no part of this design blocks it.

**Alternatives rejected**:
- Client sends `WorktreeCreate` then `SessionCreate`: two steps, a window of an unfulfilled
  definition and a half-completed start if the client dies between (SC-004 wants one).
- Roll back the worktree on a session failure: loses user data already created.

## R5. Removing the add affordance (FR-004)

**Decision**: the reducer's `SessionState::start_intent` / `start_affordance_offers_a_choice`
(`features/session.rs:2232,2280`) gain an occupancy input and return a new
`StartIntent::Occupied` (no action); `ui/sidebar.rs` (`start_session_action`, line 552, and
`build_default_item`) renders no start action for an occupied row. The core `occupancy` answer
is computed from the same session list the sidebar already holds. The daemon check (R2) remains
the authority for a stale window (US1 s1).

**Rationale**: the sidebar renders an intent and decides nothing (its own doc comment), so the
decision stays in the render-free reducer where Principle I applies.

**Alternatives rejected**: disable the button with a tooltip: FR-004 says the row offers no start
action (an additional session is the menu entry only), and a disabled control still reads as an affordance.

## R6. `create_worktree` over MCP with a definition

**Decision**: add optional `ai_cli` and `prompt` to the `create_worktree` input schema
(`core/src/mcp/tools.rs`, parsed in `create_worktree_request`), valid with both the literal and
the derived request. Policy: `check_policy` additionally evaluates
`Operation::CreateSession { worktree: Named(dir_name), ai_cli, prompt: None }`, so an agent that
may not create sessions cannot do it through this door. The reply row gains `session`,
`lifecycle`, `prompt_delivered`, `prompt_reason` as `create_session` returns them. Without
`ai_cli`/`prompt` the call is the 550 call, unchanged (FR-006). `prompt` alone takes the default
CLI (spec edge: unnamed CLI takes the default).

**Rationale**: one door per effect; reusing `create_session`'s reply shape means agents parse
one format. The Default-session exception (Principle III) is unaffected: create only, and the
session runs in the new worktree.

**Alternatives rejected**: a separate `create_worktree_and_session` tool: duplicates the 550
schema and breaks "existing calls keep their result" for no gain.

## R7. Resume replaces (FR-007, FR-008b)

**Decision**: the only path that resumes a provider session into a worktree is `AttachApply`
with `AttachItem::Session` (`state.rs:2965`), which today adds a record. Change:
- if the target location is occupied by a different session, outcome is the new
  `RefuseReason::LocationHasSession` unless the request carries `replace: true`;
- with `replace: true` on the item, in this order, never via `delete_session` (its archive writes a
  durable provider marker with no undo, `mark_archived_durable`): (1) stop the old session's
  processes (the `remove_live_by_ids` step); (2) `Catalog::replace_session` pushes the new record
  and persists, rolling the push back on failure as `attach_session` does, so a failed write
  leaves the old session listed; (3) only after a successful persist, archive the old record,
  writing its durable marker last, and persist again (a failure here leaves both listed, is
  logged, and the user's delete resolves it); (4) revoke the old session's tool credentials
  (`revoke_tool_credentials`) as `delete_session` does; its pasted images are left to the startup
  sweep (`sweep_pasted_images`, feature 487 FR-014). The old record is archived once. A daemon test
  fails the first persist and asserts the old session is still listed and un-archived;
- the client's attach dialog shows the refusal, asks the person to confirm naming both sessions,
  and resends with `replace: true`. No MCP tool resumes (clarification), so `attach_worktree`
  is unchanged.
`AttachItem::Session.replace` is serde-defaulted `false` and per item.

**Rationale**: refuse-then-confirm keeps the daemon the authority (no client-only check) and
makes "confirm" a wire fact, so a scripted client cannot replace silently.

**Alternatives rejected**:
- Client-only confirmation before sending: a second window or an old client bypasses it.
- A separate `SessionReplace` message: duplicates attach's discovery and validation.

## R8. Adoption and "attach all" (FR-012)

**Decision**: `Catalog::adopt_discovered_sessions` (`catalog.rs:1443`, run at every project open
by `discover_external_sessions`) and `attach_apply` batches go through one core helper
`attach::cap_per_location(candidates, occupied)` that keeps, per location, the candidate with the
latest `ResumableSession.last_activity` (existing field), and none where the location is
occupied. Sessions not catalogued stay in `list_resumable_sessions` as resumable (they are
discovered by id, not by presence in the catalog). A single explicit resume (a batch of one
session item) is the user's pick and is never capped, only subject to R7. Accepted consequence:
the cap is not sticky; after the user deletes the catalogued session the next project-open
adoption finds the location free and catalogues the next most recent resumable session (today
all are catalogued at once). Skipping a location with an archived session would hide sessions the
user never saw, so it is not done.

**Rationale**: adoption is the one background writer that could create two sessions in a
worktree with no user action; capping there closes the last hole in SC-001. The occupancy check runs inside
`Catalog::attach_session` and `adopt_discovered_sessions` under the catalog lock, not in
`attach_apply`, which drops the state lock between discovery and attach, so a stale window or a
racing create cannot slip a second record in (FR-005).

**Alternatives rejected**: leave adoption alone and rely on the create check: the check guards
creates, not adoptions.

## R9. Legacy catalogs (FR-013, SC-005)

**Decision**: no migration, no startup change. `occupancy::start_block(sessions, id, live_ids)` returns
`Blocked { live }` when another un-archived session of the same location is starting, running or
restarting (`SessionLifecycle::is_live`, already used by `attach_session`). The check
and the claim happen at the one choke point every start passes, in one state-lock section: the
plan lookup of `DaemonState::start_session_gated` (`state.rs:3800`, reached by `ops::start_session`
from `spawn_session_start`, `create_session_with_prompt` and MCP `start_session`; a supervision
respawn goes through `respawn_primary` instead, and its session is already in `Inner.sessions`,
so the claim never applies to it).

*What counts as live.* A start in flight is not on the record (the record learns `Running` only
once the process exists; `state.rs:1552`), and `begin_start`'s `Inner.starting` entry is a
UI/input hold that every entry sets before the gated section, so it is not evidence. The check
therefore reads three things under the lock: other sessions of the location whose record
`is_live()`, those with a live process in `Inner.sessions`, and those holding a *start claim*.
`start_block(sessions, id, live_ids)` takes that set of ids (`live_ids: &HashSet<SessionId>`), so
the core function stays pure and the daemon supplies the process and claim facts.

*The claim* is new state `Inner.start_claims: HashMap<(PathBuf, SessionLocation), SessionId>`,
inserted in the same lock section as the check when no other session is live or claiming, and
an RAII guard (its `Drop` removes the entry) taken in the plan-lookup section and dropped right
after `register_session`, so every early `return Err` and any unwind releases it. Of two legacy
sessions started together (both already in `starting`), the first to reach the gated section
claims the location and the second is blocked, naming it; they are serialized by the lock, so
exactly one wins. The concurrent-start test starts two stopped legacy sessions through the real
entries (`SessionStart` from two connections, and MCP `start_session`) and asserts one process.

*Other starts.* A session that was itself live (`Restarting`, or holding the claim) is not
blocked by others; no live legacy session is stopped by this feature; a session that exited
while another is live stays stopped until the user starts it, subject to the same check. A cheap
pre-check (the same function, `DaemonState::start_refusal(session)`, reading records and live
processes only) is also called before `begin_start` on the four start entries so none shows
`Starting` for a refused start:
`SessionStart` and `SessionRestart` (`server.rs:896,909`; fire-and-forget, so the daemon
silently skips and the client reducer asks the same core function to disable the start and show
the text), MCP `start_session` (`mcp/tools.rs:1133`, a `Conflict` error) and review send. A
`Failed` mark is not used: it would rewrite a legacy record. A session whose location has one record is never blocked,
and restarting a live session is not "another" (FR-008a, FR-009).

**Alternatives rejected**: auto-archive extras at open: spec forbids losing records
(SC-005); the user deletes extras.

## R10. Review send into a worktree with a stopped session

**Decision**: `review_send_to_new_session` (`state.rs:2263`) creates a session when no session
of the entry runs. With the rule it would be refused for an entry whose session is stopped.
Change: when the entry has its own session that is not live, start that session (`LaunchMode::Resume`)
and type the prompt as first input, as the new-session path does; create only when the entry
is `Free`. Same wording and failure handling.

**Rationale**: the feature must not break "send review comments" for a worktree whose session
is idle, the common case after a restart.

**Alternatives rejected**: refuse the send: a regression on feature 482.

## R11. Docs

**Decision**: `docs/user-guide/worktrees-and-sessions.md` (the rule, the dialog fields, resume
replaces and its confirmation, legacy worktrees) and `docs/user-guide/agent-tools.md`
(`create_session` refusal text, `create_worktree` `ai_cli`/`prompt`, `start_session`) change in
the PR, and the tool catalogue descriptions in `core/src/mcp/tools.rs`. The user-guide check
(`scripts/check-user-guide-updated.sh`) enforces it.

## R12. The limit is soft: the explicit request (amendment 2026-10-13)

**What M1 shipped that changes** (merged in #794, hard refusal):
1. `Catalog::create_session` refuses unconditionally. Now it takes `Additional::{No, Yes}`; `Yes`
   skips the occupancy check only. Every existing caller passes `No` (review send, runs,
   `create_worktree_with_session`, `SessionCreate` without the field).
2. `ClientMsg::SessionCreate`, `SessionStart`, `SessionRestart` and `AttachItem::Session` carry no
   opt-in. All four gain `allow_additional_session: bool`, serde-default false; `PROTOCOL_VERSION`
   41 to 42, with `SCHEMA_HASH`.
3. MCP `create_session` and `start_session` schemas and descriptions say "at most one" absolutely.
   Both gain optional boolean `allow_additional_session`; the descriptions state default refusal and
   the opt-in.
4. `occupancy::refusal_text` / `start_block_text` omit the way forward. They append it (R3).
5. The in-lock start check of R9 (`start_session_gated`, `start_refusal`) was framed as "legacy
   only". It is the general rule for starting a session beside a live one and is skipped when the
   request is explicit.
6. User-guide pages `agent-tools.md` and `worktrees-and-sessions.md` state the refusal as
   absolute; reworded. Spec FR-001..FR-005, FR-013, SC-001..SC-003 amended.
Unchanged from M1: the `occupancy`/`start_block` functions, the typed `CreateSessionError`, the
wire fields added for `create_worktree`, the review-send reuse (R10), all M1 tests (they exercise
the default path, which still refuses).

**Decision**: one boolean, one name, everywhere: `allow_additional_session`. A per-call flag
rather than a setting (no stored state; each additional session is a deliberate act) and not a
second tool (a second door to keep in step with policy). Policy is unchanged: the same
`CreateSession` operation is evaluated; the flag does not bypass it. `start_session` with the
flag on a live/starting session is still the no-op of FR-009. Atomicity (FR-005) is unchanged for
requests without the flag; with it there is nothing to race.

**App**: the explicit request is the worktree row menu "Start another session..." on an occupied
location, then `confirm_additional_session` (copies `confirm_session_remove.rs`) naming the worktree
and the existing session; confirm sends `SessionCreate { allow_additional_session: true }` with
the default CLI (the start menu's choice applies as for any start: the confirmation is followed
by the existing CLI choice only when the default CLI is missing). A start blocked by FR-013
(stopped extra, another live) shows the same confirmation naming the live session and re-sends
`SessionStart` with the flag. A resume into an occupied worktree offers Replace and Keep both.

**Alternatives rejected**: a project or worktree setting "allow several" (new persisted state,
and it makes accidents silent again); a tool-only opt-in (the user asked for the app too).

## R13. Worktree rows only, and the default session

**Decision**: `occupancy::default_session(sessions, &location, last_viewed: Option<SessionId>)
-> Option<SessionId>`: `last_viewed` when it names an un-archived session of the location, else
the first un-archived session of the location in list order (stable, persisted; the same one a
refusal names), else `None`. The window holds `last_viewed` per location in reducer state
(session feature, transient, like the active session); nothing is persisted. `Session` has no
activity timestamp that survives (`attention_seq` is transient), so "most recently active" is not
available without a schema change; "last viewed here, else the original" is.

**Sidebar**: `build_items` (`ui/sidebar.rs:594`) pushes `session_tree_item` rows only when the
node is expanded; `node.expanded` and `default_expanded` default to collapsed and the expander is
drawn only when `node.sessions` has more than one entry. A press on the row (today it only
toggles) emits `Session::Selected(default_session)`; the expander toggles as before. Rename and
close of a single session move to the worktree row's existing context menu
(`WorktreeMsg::MenuToggled`, `ui/sidebar.rs`); a multi-session sub-row keeps its menu. Unread and
activity indicators already aggregate per location (feature 575, FR-003), so a collapsed row loses
nothing.

**Alternatives rejected**: removing session rows outright (extras would need a new switcher widget
in the pane, a Principle VIII addition for a rare case); selecting the most recent by creation
(the original session is the better default: the one the user chose at start).
