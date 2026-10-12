# Research: Session Tree for Agent-Spawned Sessions

## R1. Today's code has no origin and no parent signal in any reader

Decision: add the field and seam; nothing consumes a signal today.
Evidence: `Session` (session.rs), `StoredSession` (store.rs), `SessionSummary` (protocol/messages.rs)
carry no parent; a grep of `provider.rs` and `hooks.rs` for parent/agent fields finds none. Claude
hooks `SessionStart` and `SubagentStop` are classified `Ignored` in `hooks.rs`; the payload parser
reads only `hook_event_name`. Provider-started sessions enter the catalog only by
`DaemonState::discover_external_sessions` (state.rs), a provider-store scan that adopts them as
`Session::restored` with their own conversation id.
Alternatives rejected: infer from adoption timing (FR-002 forbids it).

## R2. Per-provider native signals

Decision: add `AiCliProvider::reported_origin(&self, config_dir, cwd, session_id) -> Option<Uuid>`
defaulting to `None`. A provider gets a reader only after its signal is confirmed against a
recorded fixture in the first implementation task (spike). An unconfirmed provider ships as a gap.

| Provider | Candidate native signal | Status |
|---|---|---|
| Claude Code | None found. In 26.5k transcript lines of Claude Code 2.1.296 across local stores, `parentUuid` and `isSidechain` are message-level links; in-session subagents live in `<session>/subagents/agent-*.jsonl`, not as top-level sessions; `origin.kind` is a per-turn origin (`human`, `task-notification`), and `claude agents --json` lists sessions with `pid, cwd, kind, startedAt, sessionId, name, status` and no parent. | Gap unless the spike finds a field for `claude --bg` sessions started from a session. |
| OpenCode | Session info has a `parentID` for sessions created by the task tool. | Confirm against a fixture; reader if present. |
| Codex | Rollout `session_meta.source` can name a spawning thread (sub-agent thread spawn with a parent thread id). | Confirm against a fixture; reader if present. |
| Copilot CLI | Sub-agent events are inside the same session's `events.jsonl`, not separate sessions. | Gap (no separate session to nest). |
| Pi | Header `parentSession` marks a fork of a session, not a spawn by an agent. | Gap: nesting forks would be a different feature. |

Every provider also gains an origin through the tool server (R5) because that signal is the
daemon's own.
Alternatives rejected: parse transcript `parentUuid`/`isSidechain` (message links within one
session, not session origins); match titles or cwd (FR-002).

## R3. Where a reported origin is read

Decision: during `discover_external_sessions` for each newly adopted session, and for adopted
sessions that have no origin on each later discovery pass (a parent adopted later makes the child
resolve; a reader that now sees a signal records it once). Not on the hook path: hooks route by the
daemon-minted session id and carry no origin today, and registering `SubagentStart` would add
sessions the app does not model.
Alternatives rejected: a file watcher (new moving part; discovery already runs at project open).

## R4. Validation and immutability

Decision: one pure `origin::validate_origin(child, candidate, lookup) -> Result<(), OriginRejected>`
used by every write path. Rejects: self, a candidate not in the same project and worktree
(`SessionLocation` equal), a chain that reaches the child (cycle, bounded walk), a child that
already has an origin (FR-010; the first stands, `record_origin` returns `Ok(false)`).
A candidate not yet in the catalog is accepted only if reported by a provider reader (R3) and kept
as pending; the full check (same worktree, no cycle) then runs when the origin appears, on the
next discovery pass. If it fails the stored origin is cleared once (the only mutation of an origin,
allowed because the fact was never valid) and the session is plain top-level without a marker
(FR-009). This makes event order irrelevant (FR-016).
Alternatives rejected: validate in the client (several writers and windows).

## R5. `create_session` through the tool server

Decision: `Catalog::create_session` takes `origin: Option<SessionId>`; the tool passes
`Some(caller)` when the target worktree equals the caller's (FR-019), else `None` (FR-009). The
origin is recorded in the same `persist()` as the new session, so no child exists without it.
The caller's lifecycle is not required (a caller that exits after spawning still records).
Alternatives rejected: a second call after creation (a crash between leaves a top-level child).

## R6. Fold state persistence

Decision: `Settings.folded_origins: BTreeSet<SessionId>`, additive `serde(default)`; the sidebar
state keeps the set of folded origins and a `Msg::ToggleOrigin(SessionId)`. Stored as *folded*,
not expanded, so a never-folded origin is expanded with no extra rule (FR-017). No pruning: the
set is a few UUIDs, and a prune at boot (empty or partial list before the first daemon snapshot,
per project and per daemon) would erase a fold the user left (FR-017).
Note: spec FR-017 says "as the sidebar's expanded nodes already do", but worktree `expanded`
(`features/sidebar.rs::State.expanded`) is in-memory only today (no persisted field found in
`settings.rs`, `workspace.rs` or `shell/persist.rs`); the persistence here is new. FR-017 is
unchanged in behaviour; only its justification is corrected.
Alternatives rejected: persist in `projects.json` (client must not write it, persist.rs); persist
`expanded` for worktrees too (out of scope).

## R7. Wire and compatibility

Decision: `SessionSummary` gains `origin: Option<SessionOrigin { id, status }>`;
`PROTOCOL_VERSION` 40 to 41 (the build script's schema hash changes with messages.rs). `status` is
`Known | Removed`, computed by the daemon from the catalog: `Removed` when the origin's record is
archived; `Pending` is not sent, it is the client's reading of `Known` whose id is not in the
snapshot. Limit: if an origin's record were purged outright the daemon cannot say it was removed;
archived records are retained today, so the case does not arise.
Archiving: `Catalog::archive_session` (single delete), `archive_session_ids` and
`archive_worktree_sessions` (all three call `Session::archive`) share one helper that emits a
`SessionChanged` for every direct child of an archived session, carrying `status: removed`, in
the same step as the archive; a client never has to infer removal from an id leaving the snapshot.
A snapshot taken after the archive carries the same status, so a reconnect agrees.
Alternatives rejected: send a pre-built tree (the tree is a view, FR spec Key Entities); a
parallel `origin_removed: bool` (unrepresentable-state friendly enum instead).

## R8. Sidebar rendering

Decision: `worktree_tree`/the Default group build a `SessionTree` and emit rows depth-first;
`session_tree_item(depth, …)` already takes a depth. Children appear at depth+1 in the origin's
child order (catalog order). Orphans (pending/removed) render top-level at depth 1 with the child
marker and a muted "origin pending"/"origin removed" annotation. A folded origin shows
`child count` and an attention dot if any descendant `needs attention` (reuses the existing
attention derivation in `features/sidebar.rs`/`attention.rs`). New `TreeItem` builder methods land
in the shared `tree_view.rs`, with a showcase entry (Principle VIII).
Alternatives rejected: a sidebar-local nested widget (forks the primitive); sorting children
before the user's sessions (would reorder the user's own sessions, Edge Cases).

## R9. MCP rows

Decision: `session_row` gains `"origin": "<ref>"|null` and `"origin_state": "known"|"removed"`
(omitted when no origin). Additive; existing consumers ignore unknown keys.
Alternatives rejected: nested objects (differs from the flat row style).
