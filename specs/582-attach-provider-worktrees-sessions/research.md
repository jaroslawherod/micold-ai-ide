# Research: Attach a Provider's Existing Worktrees and Sessions

Each decision lists what was rejected.

## R1. What "attach a worktree" writes

**Decision**: the 029 provenance record, through `Catalog::claim_worktree` (`crates/micold-daemon/src/catalog.rs`). `classify_owner` (`micold-core/src/worktree.rs`) returns `User` for a recorded `dir_name`, so the row shows with "Show agent worktrees" off. Idempotent and metadata-only, which gives FR-003 and FR-004.
**Rejected**: a new `attached` flag in the project state (a second record that must stay in step with creation, claim and deletion; 029 rejected the same); re-running `git worktree add` (the issue's failure: the branch is checked out); the 026-style automatic adoption (FR-012 forbids it).

## R2. Existing automatic session adoption (feature 026)

`DaemonState::discover_external_sessions` runs at every project open and adopts every recorded session of the root and every startable worktree into the catalog, hidden worktrees included. It is pinned by `crates/micold-daemon/tests/session_discovery.rs` and FR-013 says not to break existing behaviour.
**Decision**: keep it. The new pass (`attach::discover_resumable`) is read-only and lists only what an adopted catalog row does not already give the user: (a) store sessions at locations the adoption skips (missing or prunable worktree: reported unresumable; a deleted worktree's store directory), (b) store sessions recorded since the last open at an attachable worktree (`needs_worktree_attach`). Sessions already adopted under a hidden worktree are not listed (FR-007); the attachable-worktree row shows their count and attaching the worktree reveals them. (c) Copilot and Pi equivalents. Root (Default) sessions stay auto-adopted as today; this is a deliberate exception to FR-012's "nothing without the user's action", because that adoption predates it and its removal would regress feature 026.
**Rejected**: removing the adoption (breaks 026 FR-014 and its tests); extending it to deleted-worktree locations (would adopt sessions that cannot run).

## R3. Provider stores (FR-006, FR-014)

Existing seam: `AiCliProvider::{config_dir, recorded_session_ids, read_title, read_label, is_archived, has_recorded_conversation}` in `micold-core/src/provider.rs`, with `launch_args` giving `--resume <id>` (Claude), `--resume=<id>` (Copilot), `--session-id <id>` (Pi, start and resume are one request).
- **Claude Code**: `~/.claude/projects/<encoded cwd>/<uuid>.jsonl` (`ClaudeProvider::transcript_dir`; the encoding replaces every non-alphanumeric char with `-`). The encoding is lossy, so the new `store_dirs` lists a provider's store directories that could belong to the project: the encoded root, each git-listed worktree path, and each directory starting with the encoded `<root>/.claude/worktrees/`. A directory that is not an exact encoding of a known path is accepted only when the first parsable transcript line carries a `cwd` equal to a path under this project's `.claude/worktrees/` (FR-007: no other project's sessions; sibling project `/a/proj-x` can never match). Reading is bounded to that first line.
- **Copilot**: `~/.copilot/sidebar-sessions-state/<sha256(cwd)>.json` via `recorded_session_ids` per known location only (the hash cannot be reversed, so no deleted-worktree discovery).
- **Pi**: `<PI_CODING_AGENT_DIR>/sessions/--<encoded cwd>--/` via `recorded_session_ids` per known location only.
- A provider whose `config_dir()` is `None` or whose store is absent contributes nothing and no error (FR-014); an unreadable directory adds a `DiscoveryNote` (FR-009).
`store_dirs` has a default returning only the known locations' directories; only `ClaudeProvider` overrides it, because only its directory name is derivable from the cwd and carries the path, whereas Copilot's index is keyed by a hash and Pi's directory by an encoding this plan does not rely on for unknown paths. **Rejected**: scanning every project directory in the Claude store and matching afterwards (reads other projects' data, SC-002); a new `ProviderSessions` trait (the existing trait already answers each question).

## R4. Cost and ordering

One directory listing per location, never per conversation (the rule `discover_external_sessions` documents). Known catalog ids are subtracted before any `is_archived` stat. Results sort by the transcript file's mtime, newest first; the MCP tool pages with `limit` (default 50, max 200) and `offset`. The title and label reads are only done for the returned page.

## R5. Dismissing the start-up offer (deferred from clarify)

**Decision**: the banner has "Attach all" and a dismiss (x) button. Dismissal is per project and per run, held in client memory (`features/attach.rs`); it is not persisted. The offer shows only while the project has no provenance records (026 adoption of root and startable-worktree sessions happens before the snapshot, so "no sessions" cannot be part of the test; see data-model OfferState) and something is found, so it ends by itself after one attach. Nothing is lost by dismissing: the sidebar's "Attach existing…" button opens the same list at any time.
**Rejected**: a persisted "don't offer again" flag (a store schema change and `schema_hash` churn for a convenience); an auto-timeout (hides an offer the user has not read).

## R6. Protocol

New `ClientMsg::AttachDiscover { req, project }` (read-only; answers `DaemonMsg::AttachReport`) and `ClientMsg::AttachApply { req, project, targets }` (answers `OperationOk` with a per-target result). Both go through the existing `PendingOp` request/response plumbing in `daemon_sync.rs`. `WorktreeClaim` stays for the per-row claim. Adding messages bumps the protocol schema hash test (`schema_hash.rs`); the plan task updates the pinned hash.
**Rejected**: client-side discovery (the daemon is the single writer and already holds the worktree cache); reusing `WorktreeClaim` N times for "attach all" (N persists and N broadcasts).

## R7. Concurrency (FR-016)

The daemon is the only writer of `projects.json` and holds one lock around the catalog, so two windows or an agent and the app serialize; the provenance record is a set, so the second attach finds the `dir_name` present and returns `AlreadyAttached`. A batch persists once. Resume reuses the existing guard: `start_session` refuses a session whose lifecycle is `Starting`, `Running` or `Restarting`; the new refusal text names "already running". A session already running outside Micold (deferred from clarify): not detected. Micold cannot see another process's terminal, and Claude Code itself decides what a second `--resume` of a live session does; the user guide says so. **Rejected**: lock files in the provider store (violates FR-010).

## R8. MCP tools and Default sessions

`attach_worktree` is mutating, not destructive (no confirmation, like `create_worktree`), audited like the other mutating tools (`is_mutating_tool`). `mcp/policy.rs::decide` refuses a Default caller with a message citing Principle III, in the same arm as rename and delete. `list_resumable_sessions` is read-only and allowed for every caller. `list_sessions`, `list_worktrees` and `list_branches` are unchanged (FR-011). Agent-side resume of a listed session is out of scope: FR-011 asks only that an agent can find them, and adopting a session from MCP would let an agent start provider processes. **Rejected**: an `include_resumable` option on `list_sessions` (clarify answer); allowing Default callers after confirmation (Principle III).

## R9. UI

A modal list dialog ("Attach existing worktrees and sessions") with a checkbox per row, "Attach selected" and "Attach all", built from the existing overlay dialog and shared button/row primitives with the builder API. The banner is the existing notice/snackbar style extended with action buttons if the shared notice supports them; if not, a `Banner` primitive is added to the shared library and consumed from there (Principle VIII). A sandboxed project shows the row-less note "Discovery is unavailable in the sandboxed runtime" (spec edge case): the sandbox keeps its own home, so the host store is not its store.

## R10. Test strategy

| Layer | Covers |
|---|---|
| core unit (`micold-core/tests/attach_*.rs`) | `attachable_worktrees` (FR-001, prunable edge), `discover_resumable` over a fixture store (FR-006, FR-007, SC-002, corrupt entry SC-005, deleted worktree, Default, thousands), read-only guarantee (FR-010: store tree hash unchanged), tool catalog, policy (FR-015), protocol round trip |
| daemon integration (`micold-daemon/tests/attach_*.rs`) | attach persists and survives restart (FR-002, FR-003 byte-for-byte via `git status` and file hashes, SC-003), idempotence and concurrency (FR-004, FR-016), MCP `attach_worktree` and `list_resumable_sessions` end to end (Story 3, SC-004), resume attaches first (FR-008), double resume refused (FR-016), hidden-by-default unchanged (FR-013) |
| client reducer (`micold-client/tests/attach_*.rs`) | offer only when the catalog is empty (FR-012, Story 4), dismissal, selection, "already attached" message (FR-004) |
| geometry gates | none: no new fixed geometry beyond shared primitives |
| quickstart Part B (visual pass) | banner, dialog, attached row in light and dark theme |
