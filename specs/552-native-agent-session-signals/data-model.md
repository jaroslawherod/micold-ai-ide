# Data Model: native agent-session signals (#552)

## AgentRun (new, `micold-core/src/agent_run.rs`)

| Field | Type | Notes |
|---|---|---|
| `agent_id` | `String` | provider's id (Claude `agent_id`); the key within one origin |
| `agent_type` | `Option<String>` | e.g. `Explore`; display only |
| `transcript` | `Option<String>` | opaque platform path from `SubagentStop.agent_transcript_path` |
| `state` | `AgentRunState { Running, Stopped }` | `SubagentStop` before any start creates `Stopped` |
| `provider` | `AiCli` | which signal reported it |

Origin = the owning `StoredSession`/`Session` (id of the Micold session). Rules:

- Keyed `(origin, agent_id)`; a repeat start for the same key is idempotent; a stop sets `Stopped`
  and fills `transcript`; a stop before a start inserts a `Stopped` entry (Edge: out-of-order).
- A session exit, a restart of its session, or a daemon start marks that origin's `Running` runs
  `Stopped` (the Stop hook can never arrive); tested with the FR-005 restart test.
- Cap 256 per origin; evict oldest `Stopped` first.
- Never created for an unknown origin or a payload without `agent_id` (FR-008).
- Origin deletion removes its runs with it; the runs are not catalog sessions, so nothing is
  "promoted" (Edge: deleted origin).

## Stored form

`StoredSession.agents: Vec<AgentRun>` (`#[serde(default)]`, skipped when empty). Schema version
unchanged. `StoredProjectState.worktree_provenance` and `provenance_migrated` are read into ignored
fields and not written back (FR-010).

## Wire

`SessionSummary.agents: Vec<AgentRunSummary>` (same fields). `PROTOCOL_VERSION` 40 -> 41 (M2); the removal of the worktree attach messages/status (R-C, M3) bumps it to 42.

## Removed

`WorktreeOwner`, `ProvenanceView`, `classify_owner`, `matches_reserved_convention`,
`show_agent_worktrees`, `ResumableStatus::NeedsWorktreeAttach`, `AttachTarget` worktree arm,
`Operation::AttachWorktree`.
