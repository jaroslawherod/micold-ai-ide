# Research: native agent-session signals (#552)

Sources: Claude Code hooks reference (code.claude.com/docs/en/hooks), local `~/.claude/projects/*/<session>/subagents/agent-*.jsonl`
transcripts, this repo's `crates/micold-daemon/src/{hooks,activity}.rs`, `scripts/autopilot/agent-hook.sh`.

## R1 Claude sub-agent hook payloads and identity

- **Decision**: register `SubagentStart` and `SubagentStop` as `http` hooks in the per-session
  settings. Payload (docs): common fields `session_id`, `transcript_path`, `cwd`, `hook_event_name`
  plus `agent_id`, `agent_type`; `SubagentStop` adds `agent_transcript_path`, `stop_hook_active`,
  `last_assistant_message`. The record key is `(origin Micold session, agent_id)`.
- **Evidence the sub-agent shares the parent session**: the docs example's `session_id` matches
  the parent transcript name; on disk, sub-agent transcripts sit under
  `<parent-session-id>/subagents/agent-<agent_id>.jsonl` and carry `agentId`, with the parent's
  `sessionId`. The docs do not state this outright; the on-disk layout is the local proof.
- **Consequence**: the origin is known from the hook URL (`/hook/<micold-session-uuid>`), not from a
  payload field, and a sub-agent is never a catalog session. Spec FR-004's "agent session's id" is
  `agent_id`.
- **Verification**: T001 captures a live payload and the `--settings` acceptance before any
  registration code; a contradiction reopens this decision.
- **Rejected**: keying by `session_id` (it is the origin's, so every sub-agent would collide);
  inferring sub-agents from `agent_id` on `PreToolUse` payloads (works, but `SubagentStart` is the
  explicit signal and gives the type up front); inferring from transcript directory names (a name
  heuristic, forbidden by FR-001).

## R2 Background agents

- **Decision**: documented gap. The hook list (33 events) has no background-session event;
  `claude --bg`/`claude agents` sessions are managed by Claude outside Micold's hook settings, and
  `Agent` calls with `run_in_background` still fire `SubagentStart`/`SubagentStop`, so background
  *sub-agents* are covered.
- **Rejected**: `WorktreeCreate` as a signal (fires for `--worktree` and isolation, not for agents
  in an existing worktree; would be a guess); scanning Claude's session store for sessions in a
  worktree not started by Micold (a heuristic by timing/location).

## R3 Copilot, Pi, Codex, OpenCode

- **Copilot**: the daemon already tails `events.jsonl` (`activity.rs::copilot_event`). Copilot's
  log is believed to carry sub-agent events (`subagent.started` / `subagent.completed`); this was
  **not verifiable offline here** (the local capture fixture has none). Decision: task T001
  captures a real log from the installed CLI; if the event is present the mapping lands in a
  sibling `copilot_agent_event`, otherwise Copilot is a documented gap. Either way no guessing.
- **Pi, Codex, OpenCode**: no sub-agent hook or event is known in the repo's contracts
  (`specs/029-pi-cli-provider`, provider table in `provider.rs`). Decision: documented gaps; the
  per-provider table in the user guide says so. Revisit when a provider ships one.
- **Rejected**: a generic "provider emitted a new session in the same cwd" heuristic (timing/name
  guess, out of scope per spec).

## R4 Storage

- **Decision**: `StoredSession.agents: Vec<AgentRun>` with `#[serde(default)]`, no schema bump
  (`store.rs` tests pin that additive fields keep version 1). Capped at 256, oldest stopped runs
  dropped first. Wire: `SessionSummary.agents`, protocol 40 -> 41.
- **Rejected**: a separate agents file (second consistency point, no benefit); live-only on
  `LiveSession` (fails FR-005); keying on the catalog session list (sub-agents are not sessions).

## R5 Activity isolation (FR-007)

- **Decision**: a new `HookClass::Agent(AgentSignal)` returns to the receiver; the receiver calls
  `note_agent_signal` only. `note_activity` is not called, so the FSM is unchanged and
  `SubagentStop` stays out of the turn-end path as feature 613 requires.
- **Rejected**: mapping `SubagentStop` to `Stop` (the very bug 613 fixed).

## R6 Removal inventory

Taken from a code map of the worktree: classification in `worktree.rs:51-195`; toggle in
`features/sidebar.rs` and `ui/sidebar.rs:403`; provenance in `workspace.rs:72-83`, `store.rs`
(`StoredProjectState`), `catalog.rs:614-786`; attach in `mcp/tools.rs`, `attach.rs`,
`features/attach.rs`, `shell/daemon_sync.rs`. The toggle was never persisted (no preference key),
so FR-010's hide preference clause is satisfied by there being nothing to load; the stored
provenance keys are the real compatibility case.

## R7 Hook registration risk

`--settings` accepting `http` hooks is already relied on by the five existing events. `SubagentStart`
supports `http` (docs). If a Claude version predates the event, the settings loader would
reject an unknown key: the hook name is a documented event since sub-agents shipped, and the live
check is quickstart §C.

## R8 Live captures (T001)

- **Claude Code 2.1.296**, `claude -p --settings <file>` with `SubagentStart`, `SubagentStop` (and
  `PreToolUse`) as `{"type":"http"}` hooks to a local listener, prompt "Use the Explore agent to
  list files...". `--settings` **accepted** both events; the `http` type worked (no `command`
  fallback needed). Raw bodies (home and temp paths scrubbed): `contracts/captures/claude-SubagentStart.json`,
  `contracts/captures/claude-SubagentStop.json`.
- **SubagentStart** keys: `session_id`, `transcript_path`, `cwd`, `prompt_id`, `agent_id`
  (`a15f822c8aa66a621`), `agent_type` (`Explore`), `hook_event_name`. No `permission_mode`.
- **SubagentStop** keys: the above plus `permission_mode`, `effort`, `stop_hook_active`,
  `agent_transcript_path` (`<parent-session-id>/subagents/agent-<agent_id>.jsonl`),
  `last_assistant_message`, `background_tasks`, `session_crons`.
- **Identity**: `session_id` in both events equals the parent's (same value as the parent's
  `PreToolUse` for the `Agent` tool call). Sub-agent tool calls (`PreToolUse` for `Bash`) carry the
  parent `session_id` plus `agent_id`/`agent_type`. Transport headers: `Content-Type:
  application/json`, `User-Agent: axios/...`; no auth header unless configured.
- **Copilot CLI**: not installed (`which copilot` fails); no sub-agent event captured. R3 stands:
  Copilot stays "unverified, gap unless a real log shows the event".
- **R1**: confirmed, not contradicted. Extra fields beyond R1's list (`prompt_id`, `permission_mode`,
  `effort`, `background_tasks`, `session_crons`) are ignorable; `agent_id`/`agent_type` are always
  present on both events.
