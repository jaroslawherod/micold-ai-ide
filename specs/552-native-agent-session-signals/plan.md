# Implementation Plan: Identify Agent Work From the AI CLI's Native Signals

**Branch**: `feat/552_replace-name-based-hiding-of-agent-work-with-the` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/552-native-agent-session-signals/spec.md`

## Summary

Two halves. **Removal**: delete the name-based worktree classification (feature 014's `agent-` /
`worktree-agent-` convention), the "Show agent worktrees" chip, the provenance records and their
one-time migration (029), and feature 582's worktree attaching, so every discovered worktree is
listed. **Replacement**: register Claude's `SubagentStart` / `SubagentStop` hooks in the per-session
`--settings` file, parse them in the daemon's hook receiver into an *agent run* record keyed by the
origin Micold session (the hook URL's session) and Claude's `agent_id`, persist it on the stored
session, and carry it on `SessionSummary` for #551's tree. Copilot is mapped from its own
`events.jsonl` if the capture at task T001 confirms a sub-agent event; Pi, Codex and OpenCode are
documented gaps. The hooks never feed the activity FSM (FR-007).

**First task (T001)**: capture a real `SubagentStart`/`SubagentStop` body and confirm `--settings`
accepts both events, and capture a Copilot `events.jsonl` with a sub-agent, before any registration
code (research R1, R3). A result that contradicts R1 returns to this plan.

**Finding that reshapes the spec's model** (research R1): a Claude sub-agent is not a separate
session. Its hook payloads carry the *parent's* `session_id` and `transcript_path`, plus `agent_id`
and `agent_type` (SubagentStop adds `agent_transcript_path`). A sub-agent therefore has no Micold
catalog entry and no PTY. The "agent session id" of FR-004 is Claude's `agent_id`; the "origin" is
the Micold session whose `/hook/<uuid>` URL received the payload. FR-011 holds trivially (nothing is
hidden; sub-agents are not catalog sessions). Background sessions (`claude --bg`) have no hook
event, so they are a documented gap (R2).

## Technical Context

**Language/Version**: Rust (workspace edition as in `Cargo.toml`), iced client

**Primary Dependencies**: existing only: `serde`, `serde_json`, `tokio` (daemon hook receiver). No new crate.

**Storage**: local per-project state files (`StoredProjectState`, `crates/micold-core/src/store.rs`); additive `#[serde(default)]` field, `SCHEMA_VERSION` stays 1

**Testing**: `cargo test` via `mise run test-core` / `mise run gate`; daemon integration tests in `crates/micold-daemon/tests/`; client reducer tests in `crates/micold-client/tests/`

**Target Platform**: Linux, macOS, Windows (hook receiver is loopback HTTP, platform independent)

**Project Type**: desktop app: `micold-core` (pure), `micold-daemon` (service), `micold-client` (iced)

**Performance Goals**: hook handling stays O(1) per payload; at most 256 agent runs kept per session

**Constraints**: offline; payload body never logged (existing FR-047); loopback best effort

**Scale/Scope**: touches ~45 source/test files, mostly deletions (research R6)

## Constitution Check

*GATE: pass before research; re-checked after design (still pass).*

- [x] **I. Test-First**: every code unit starts from a failing test; the test list is in the Test Strategy below. Deletions start by flipping the visibility tests (all worktrees listed) to red.
- [x] **II. Multi-Session**: the record is scoped to one origin session and one daemon (FR-014); concurrent sub-agents are separate entries keyed by `agent_id`; persisted with the session and restored on restart.
- [x] **III. Worktree Integration**: removing the toggle lists every worktree; worktree create/switch/cleanup unchanged; no new location is introduced.
- [x] **IV. Local-First**: all state in the project state file and loopback hooks; nothing leaves the device.
- [x] **V. Rust + iced**: typed `AgentRun`, `AgentSignal`; hook payloads parsed to a struct, not substring scans; `RunState` enum makes "stopped without a start" explicit.
- [x] **VI. Cross-Platform**: paths from payloads are opaque `String`s, never parsed; no platform code added.
- [x] **VII. Documentation**: `docs/user-guide/worktrees-and-sessions.md`, `agent-tools.md`, `settings.md`, `docs/README.md` updated in the same change; per-provider table (FR-012).
- [x] **VIII. UI Components**: the UI change is a removal (chip and attach dialog rows); no new widget. #551 owns the tree.

## Requirement Map

| FR | Where |
|---|---|
| FR-001, FR-002 | Removal plan R-A: `worktree.rs` classification, `features/worktree.rs` filter and tags, `features/sidebar.rs`, `ui/sidebar.rs` chip |
| FR-003 | `hooks.rs` `HooksMap` + `classify_hook`; [contracts/subagent-hooks.md](contracts/subagent-hooks.md); Copilot mapping in `activity.rs`/`event_log.rs` (R3) |
| FR-004 | [data-model.md](data-model.md) `AgentRun`; `SessionSummary.agents`; protocol 42 -> 43 (M2), 44 -> 45 (M3, after 776 took 44) |
| FR-005 | `StoredSession.agents` (additive, `serde(default)`) |
| FR-006 | no record written without a payload; `agents` defaults empty |
| FR-007 | `HookClass::Agent` never reaches `note_activity`; contract C3 |
| FR-008 | contract C2 (rejections) + `DaemonState::note_agent_signal` |
| FR-009 | Removal plan R-C: provenance, migration, `attach_worktree`, `needs_worktree_attach`, dialog rows; discovery/resume kept, offer retriggered by "stored sessions the catalog lacks" |
| FR-010 | Removal plan R-B: stored fields `worktree_provenance`, `provenance_migrated` become read-and-ignored `serde` fields; tolerant load test |
| FR-011 | R1: sub-agents are not catalog sessions; the record is on the origin's summary |
| FR-012, FR-013 | Docs plan R-D |
| FR-014 | record lives on one `StoredSession`; update path takes one session id |

## Project Structure

### Documentation (this feature)

```text
specs/552-native-agent-session-signals/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/subagent-hooks.md
└── tasks.md             # /speckit-tasks
```

### Source code touched

```text
crates/micold-core/src/
├── worktree.rs          # remove AGENT_*_PREFIX, is_agent_id, matches_reserved_convention, WorktreeOwner,
│                        #   classify_owner, ProvenanceView, durably_known_worktrees, plan_backfill (R-A, R-B)
├── workspace.rs         # worktree_provenance, provenance_migrated, record_user_created: removed (R-B)
├── store.rs             # StoredSession.agents (new); provenance fields read-and-ignored (R-B)
├── attach.rs            # keep discovery/resume; remove attachable_worktrees, AttachTarget worktree arm,
│                        #   NeedsWorktreeAttach (R-C)
├── session.rs           # Session.agents: Vec<AgentRun> (new, AgentRun in a new agent_run.rs)
├── mcp/{tools,policy}.rs# remove attach_worktree
├── protocol/{messages,version}.rs # SessionSummary.agents; drop worktree attach shapes; 42 -> 43 (agents, M2) -> 45 (attach shapes removed, M3; 776 took 44)
│                        #   (tests/schema_hash.rs pin and docs/daemon.md follow)
└── bin/micold-demo-provenance.rs  # deleted (and its use in site/capture/demo-project.sh)
crates/micold-daemon/src/
├── hooks.rs             # register SubagentStart/Stop; HookClass::Agent; payload struct
├── state.rs, catalog.rs # note_agent_signal; persist; session_summary; drop attach_worktrees/claim provenance
├── mcp/tools.rs, server.rs, ops.rs # remove attach_worktree handler and worktree attach targets
└── activity.rs / event_log.rs # Copilot mapping, if confirmed (R3)
crates/micold-client/src/
├── features/{worktree,sidebar,attach}.rs, ui/{sidebar,mod,attach_dialog}.rs, catalog_sync.rs,
│   shell/{startup,daemon_sync}.rs, overlay/registry.rs, app.rs, main.rs  # R-A, R-C
docs/user-guide/{worktrees-and-sessions,agent-tools,settings}.md, docs/README.md
specs/{014-hide-agent-worktrees,029-worktree-provenance,582-attach-provider-worktrees-sessions}/  # superseded notes
```

**Structure Decision**: no new crate or module beyond `micold-core/src/agent_run.rs`; extend the
existing hook receiver and stored session.

## Removal plan

- **R-A (hiding)**: the client lists every discovered worktree; `worktree_tags` loses the "Agent"
  tag; `show_agent_worktrees` and `Msg::ShowAgentWorktreesToggled` go. The visibility tests in
  `worktree_visibility.rs`, `sidebar_tree.rs` (444-685, 941-1052), `app_state.rs` (617-777) flip
  to "all listed"; `reserved_convention.rs` is deleted.
- **R-B (provenance)**: stop reading it for any decision; `StoredProjectState` keeps the two keys
  as ignored `serde` fields so older files load (FR-010) and are dropped on next write. No
  migration code remains. `provenance_migration.rs`, `worktree_provenance*.rs` are deleted or cut.
- **R-C (attach)**: `attach_worktree`, `AttachTarget::Worktree`, the dialog's worktree rows,
  `needs_worktree_attach` (a session whose worktree exists is `resumable`). The start-up offer
  fires when `AttachDiscover` finds stored provider sessions the catalog lacks; dismissal stays
  in-memory per run. The attach message-shape removal bumps the protocol to 45 (M3).
- **R-D (docs)**: superseded banners on 014, 029, 582 (each names 552 and what replaced it); user
  guide pages updated; a provider table: Claude = `SubagentStart`/`SubagentStop`; Copilot per R3;
  Pi, Codex, OpenCode = no native signal (gap); background `claude --bg` sessions = gap.

## Test Strategy

| Requirement | Layer |
|---|---|
| FR-001/002, US1 | client reducer/state tests (all worktrees listed, no toggle, rename invariance); core test that no classification API remains (compile-time removal) |
| FR-003/004/008, US2 | daemon integration `hooks_receiver.rs`: post SubagentStart/Stop bodies, assert summary; unit tests for payload parsing in `hooks.rs` |
| FR-005 | core `store_roundtrip.rs` (additive field, version stays 1) + daemon restart test |
| FR-007 | daemon `attention_claims.rs` (existing SubagentStop test kept, now with registration) |
| FR-009 | core `attach_discovery.rs`, daemon `attach_apply.rs`, `mcp_tools_catalog.rs`, client `attach_offer.rs` |
| FR-010 | core `store_*` test loading a pre-feature file with both keys and a hide preference |
| FR-006 | daemon + core: a session that received no signal has an empty `agents` list |
| FR-011 | client: a summary carrying runs lists every worktree and session unchanged (nothing hidden) |
| Protocol bump | `crates/micold-core/tests/schema_hash.rs` pin (42 -> 43 in M2, 44 -> 45 in M3), `version.rs` history, `docs/daemon.md` |
| FR-014 | daemon test: two sessions, signal for one leaves the other untouched |
| FR-012/013, US3/4 | docs grep test in `scripts/tests/` or quickstart §A (docs are verified in CI) |
| Visual | quickstart §B: sidebar shows no chip; no geometry gate changes (a chip is removed) |

## Complexity Tracking

No constitution violations.
