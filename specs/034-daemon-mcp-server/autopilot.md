# Autopilot ledger — 034-daemon-mcp-server

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: "the daemon server should expose mcp server that will be automaticly binded into AI sessions. Should allow to manage the sessions and worktries."
- **Kind**: feature
- **Worktree branch**: feat/daemon-should-expose-mcp-server-for-agent
- **Started**: 2026-09-29
- **Phase**: 4-milestone M5
- **Next step**: M5: gate, then a sonnet re-review of review A's fixes, then review B, then T103 (real-CLI §B1 step 7/§B2, §B4 visual pass), then tick tasks.md, push and open the PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #455 | Spec | merged | 8ff6e0561386ba21219411bc16f828f08b177961 |
| #465 | Design (PR 2) | merged | a5fcc795294b6dcd2972ab5e3d31a814b41d8fb7 |
| #469 | M1: bound sessions with the read tools | merged | 3c8d332ddc4f4497d0caa1764ae2fbf2c301c6f3 |
| #474 | M2: Settings toggle for the tool server | merged | 7964273e |
| #498 | M3: create_worktree / create_session | merged | 2486d096bfb6ddbba8f6b6cad1f2489f3a2e481c |
| #509 | M4: start_session / rename_worktree | merged | f19454dcccf06d54c8da403a7a05e3931a0152b8 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T022, T078–T082, T100 | Bound sessions (Claude, Copilot) with the read tools | #469 | merged |
| M2 | T023–T028, T083, T101 | Settings toggle for the tool server (protocol 17) | #474 | merged |
| M3 | T029–T042, T072–T073, T084–T088, T102, T105 | create_worktree / create_session with first prompt, audit line | #498 | merged |
| M4 | T043–T048, T089 | start_session / rename_worktree; policy rows for the destructive tools | #509 | merged |
| M5 | T049–T059, T090–T094, T103 | Confirmations in app windows; destructive tools (protocol 18) | — | in progress |
| M6 | T060–T071, T095–T099, T104 | Cross-session read/send under the FR-016 setting (protocol 19) | — | planned |
| M7 | T074–T077 | Read latency, sandbox placement, final real-CLI pass, user guide | — | planned |

Size note: M1 (28), M3 (22), M5 (17) and M6 (18) exceed the ~15 guideline because each carries its
acceptance-gate and quickstart tasks and no split along an acceptance scenario leaves an observable
deliverable (tasks.md Notes).

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | May a Default session's agent create/rename/delete worktrees through the tool server? | No: refused by policy (FR-015a); lifting it needs a constitution amendment | agent-resolved | .specify/memory/constitution.md#III (review round 1, F1) |
| D2 | clarify 1 | May an agent interrupt its own calling session? | No: refused as invalid input (FR-015); the interrupt would abort the turn awaiting the result | agent-resolved | spec.md#FR-013, crates/micold-core/src/protocol/messages.rs#SessionInterrupt |
| D3 | clarify 1 | Cap or rate-limit agent-created worktrees/sessions? | No: same (absent) limits as the user's action (FR-009) | agent-resolved | spec.md#Assumptions; no limit in crates/micold-daemon/src/server.rs |
| D4 | clarify 1 | FR-010: other projects in scope? | Own project only; other projects' targets read as "not found" | decided by user | AskUserQuestion, clarify round 1 |
| D5 | clarify 1 | FR-014: destructive-op policy? | Confirm each in an app window; "needs confirmation" after 60 s or with no window attached | decided by user | AskUserQuestion, clarify round 1 |
| D6 | clarify 1 | FR-016: cross-session read/type? | Three-value Settings option: Auto (default; read and send, no confirmation) / Confirm each send (read allowed, each send confirmed as in FR-014) / Off (both refused) | decided by user | User's words: "use auto mode means send all no confirm, confirm each send and off by default auto"; confirmed: "default auto" |
| D7 | clarify 2 | Does a change to the FR-016 option reach running sessions? | Yes, from the next request (checked per request) | agent-resolved | spec.md#FR-004, #FR-006 |
| D8 | clarify 2 | Which windows show a confirmation; which answer counts? | All connected windows; the first answer wins | agent-resolved | spec.md#Edge Cases, FR-011 |
| D9 | clarify 2 | Reason categories for decline / timeout / vanished target | refused by policy / needs confirmation / not found; "cancelled" removed | agent-resolved | spec.md#FR-013 |
| D10 | plan | FR-007/SC-009 "cannot connect": what can a loopback TCP transport guarantee? | Another account cannot read any credential and gets only a refusal; FR-007/SC-009 sharpened, intent unchanged | agent-resolved | research.md#R7 |
| D11 | plan | Pre-approve the server's tools in the CLI (`--allowedTools mcp__micold`, `--allow-tool micold`)? | Yes: the spec's guards are FR-014/015/015a/016, enforced by the service; a CLI prompt would double-confirm and break SC-001 | agent-resolved | research.md#R2 |
| D12 | plan | Bind Pi via a TS bridge extension? | No: Pi has no MCP; FR-002's condition does not hold, FR-005 applies | agent-resolved | research.md#R4 |
| D13 | plan | rmcp SDK or hand-rolled server? | Hand-rolled stateless Streamable HTTP on the hook receiver's HTTP code | agent-resolved | research.md#R5 |
| D14 | plan | FR-017 waited for a first "awaiting input" that a fresh session never reports | Deliver on a per-CLI ready-for-input signal (Claude SessionStart hook, Pi component event, Copilot output settled); bound counts from the request; spec reworded | agent-resolved | crates/micold-daemon/src/activity.rs; research.md#R12 |
| D15 | plan | "Gracefully" stop, and stop/interrupt have no sidebar action | Stop ends processes, marks Idle, broadcasts, stays resumable; Assumption covers protocol operations | agent-resolved | crates/micold-daemon/src/server.rs SessionStop arm; research.md#R9 |
| D16 | plan | Does Auto-default cross-session read/type conflict with Principle II? | Yes, a conflict (speckit-analyze C1, tie-break review). Keep `Auto`; record a justified violation in plan.md Complexity Tracking; Principle II check marked "justified violation"; keep FR-010/016/018 guards and `mcp_cross_session.rs`; no amendment | decided by user | plan.md#Complexity Tracking; .specify/memory/constitution.md#II, #Governance |
| D17 | design | Resolve the Principle II conflict how: justified violation, MINOR amendment, or default `Off`? | Option 1, justified violation; no constitution amendment | decided by user | Coordinator relay of the user's decision on the escalation |
| D18 | design | Milestones M1–M7 as cut? | Ship as cut, M1 through M7 in order, no changes | decided by user | Coordinator relay of the user's decision |
| D19 | 4-milestone M3 | Claude Code never posts `SessionStart` over the HTTP hook; how does a Claude session show it is ready for its first prompt? | The output-settled rule, as FR-017 prescribes for a CLI that reports no signal; `HookSessionStart` removed | agent-resolved | evidence/m3-real-cli.md finding 1; cycle 23 |
| D20 | 4-milestone M3 | With a first prompt, a CLI's folder-trust question gets answered by the prompt's Enter; what should `create_session` do? | Read the CLI's own trust record read-only (Claude `.claude.json` `hasTrustDialogAccepted`, Copilot `config.json` `trustedFolders`, ancestors count); when it would ask, type nothing and return `prompt_delivered: false` with a reason; document "trust the project in that CLI first" | _(decided by user)_ | evidence/m3-real-cli.md finding 4; cycle 24 |

## Review rounds

| Milestone | Review | Round | Snapshot | Result |
|---|---|---|---|---|
| M2 | A (code-review high) | 1 | 3aed1d03b23b2894bec54e460f20359a00bd9751:bc68ca30a914ace0e6cd86002796e4462ce945ff | 2 fixed (unbound start revokes old credential; crash respawn keeps its start's binding), 1 doc fix, 5 declined |
| M2 | B (conformance) | 1 | 3aed1d03b23b2894bec54e460f20359a00bd9751:bc68ca30a914ace0e6cd86002796e4462ce945ff | CLEAN, 2 MINOR (ledger row fixed; persistence visual check declined) |
| M2 | A (sonnet re-review) | 2 | 6ad850082b956db79b824cda29517a5172665dba:4c3b01ae36397dc436b511995f71ef7ced33322d | CLEAN; fixes hold, declines stand |
| M4 | A (code-review high) | 1 | d60651b2f4fd809dee2612cd097776c8547cdd48:ca68d3837395b780eb1ccda1f1cf3b538d44db9f | CHANGES: F1 BLOCKER fmt (fixed), F2 MAJOR failed start answered success (fixed: service_error with the recorded reason), F3 MINOR fixed, F4/F5 MINOR declined |
| M4 | A (sonnet re-review) | 2 | 1882e06044be9f66f4b5bc1ad8b239d5237956bd:29e38622b60e5a1c9ff48c67d972cca5661cf3d1 | CLEAN |
| M4 | B (conformance) | 1 | 0e3d56ded2112ab8955dac9cdbb45e6215c2396e:a18955332115b3bc228c76e32100a90b6f61e72c | CLEAN; Verify green (mcp_policy 12, mcp_tools_catalog 19, mcp_lifecycle_tools 8); 1 MINOR declined |
| M5 | A (code-review high) | 1 | 9e8856e41323ac71ea4b9f4d660319f6e88e6832:ac6aaebe56cdac342e724f809336270975084e90 | CHANGES, 10 findings. Fixed: stale prompts dropped on disconnect (ids reused after a restart); another dialog opening declined every queued prompt (release now only at the outermost `update`); peer half-close no longer loses the reply; `stop_session` off the runtime thread; stopping an archived session is unknown and `Failed` keeps its reason; `delete_session` re-resolves its target after the answer; interrupt types Ctrl-C only into the process the user was asked about; 14-space typo in the conflict text. Declined: 3 (below) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | rounds 1–3 | none declined | Round 1: 6 MAJOR + 7 MINOR, all fixed. Round 2 (sonnet): 6 MINOR, all fixed. Round 3 (sonnet): CLEAN, 2 MINOR fixed. |
| plan | analyze | C1 CRITICAL (Principle II) | Not declined: resolved by the user as a justified violation (D16, D17); plan.md Complexity Tracking records it |
| tasks | round 1 | none declined | 2 MAJOR (M3 Default refusal untested through `POST /mcp`; M1 gate pointed at M7's T076), 7 MINOR, all fixed. |
| tasks | round 2 (sonnet) | none declined | CLEAN; 104/104 tasks in exactly one milestone. 2 MINOR fixed: protocol numbers shifted to main's `PROTOCOL_VERSION = 16` (M2 17, M5 18, M6 19); `mcp_audit_log` added to M5/M6 Verify. |
| checklists | sign-off | none declined | `checklists/requirements.md`: all 16 items ticked and confirmed satisfied by the round 2 reviewer. |
| plan | rounds 1–2 | none declined | Round 1: 1 BLOCKER (FR-017 readiness), 2 MAJOR (stop path, missing timeout probes), 8 MINOR, all fixed. Round 2 (sonnet): no BLOCKER/MAJOR, 3 MINOR fixed. |
| M1 | A round 1 | No read timeout on the head/body reads (slowloris) | Same pattern as the hook receiver's listener, which shares `http.rs`; loopback only and credential-gated after the head. Follow-up below |
| M1 | A round 1 | No chunked transfer encoding / 411 for a missing `Content-Length` | Both real CLIs send `Content-Length` (evidence/m1-real-cli.md); a chunked body reads as empty and gets a JSON-RPC parse error, not a hang |
| M1 | A round 1 | Windows `.claude.json` project-key form in the collision check | Unverified on Windows; no Windows Claude Code to test against. Follow-up below |
| M1 | A round 1 | Each read tool clones the catalog snapshot | Read latency is M7's (T074); the snapshot is small and cloned outside the lock |
| M1 | A round 1 | Duplicate registry type | Cosmetic; no behaviour difference |
| M1 | A round 1 | hooks.rs token file permissions | Pre-existing, outside feature 034; already a follow-up |
| M1 | A round 1 | Fixed temp name `.{file}.tmp` in `write_owner_only` can race | The file name is per session and one session's spawns are serialized; no concurrent writer of the same file exists |
| M1 | B round 1 | Sweep stale binding files at bind | Optional per the reviewer; two service instances (a dev run and the user's) can share the data directory, and a sweep would delete the other's live files. Stale files hold only credentials a restart already invalidated |
| M1 | B round 1 | Spawn tests are unix-only | Stand-ins are `#!/bin/sh`; porting needs Windows stand-ins. Follow-up below; the Windows cross-check covers compilation |
| M2 | A round 1 | `SettingsSet` chain short-circuits on the first failing setter | Pre-existing pattern shared by every service-owned field (scrollback, env-include, default CLI, Pi switch); M2 adds one link to it. Follow-up below |
| M2 | A round 1 | Catalog setter mutates memory before a failing persist | Same pre-existing pattern as `set_pi_activity_component` / `set_default_ai_cli`; not introduced here. Follow-up below |
| M2 | A round 1 | A save while disconnected is reverted on reconnect by `Welcome` | Pre-existing for every service-owned field (feature 026's design: the daemon's value is the one in force). Follow-up below |
| M2 | A round 1 | One more locked settings write + `SettingsChanged` broadcast per save | Follows the existing per-field pattern; batching `SettingsSet` into one update is a refactor of all fields. Follow-up below |
| M2 | A round 1 | `set_tool_server_enabled` duplicates the lock/persist/broadcast setter shape | Cosmetic; same as the four sibling setters. Covered by the batching follow-up |
| M2 | B round 1 | Visual pass did not check the toggle persists across Save | MINOR; persistence is pinned by `turning_the_binding_toggle_off_and_saving_tells_the_service`, `turning_the_binding_toggle_off_reaches_what_save_writes` and `turning_the_tool_server_binding_off_survives_a_save_and_load` |
| M4 | A round 1 | F4: only agent starts broadcast `Starting`; the sidebar's `SessionStart` does not | Tried (broadcast in `ops::start_session`): feature 026's `resume_failure_reported` pins that the first lifecycle announced after a sidebar `SessionStart` is its outcome, and went red. The sidebar's announcements stay as they were; the overlay still shows `Starting` in any broadcast made during a sidebar start |
| M4 | B round 1 | F1: `rename_worktree`'s own `default` check is unreachable after `parse_call` | Kept as a defensive `invalid_input` rather than a panic on the request path; same wording as the parser's |
| M4 | A round 1 | F5: the `starting` marker is not reference-counted, so of two concurrent starts the second runs without the overlay/held input | Pre-existing semantics of `begin_start`/`finish_start` shared with the sidebar's `SessionStart` (feature 026 T125); the tool already skips a session the snapshot shows `Starting`, and the per-session gate still serializes the spawns |
| M1 | B round 2 | No endpoint-level test for `discard_body` before 401/404/405 | Optional per the reviewer; the unit test pins the bound, and loopback socket buffers absorb any test-sized body, so an endpoint test would pass with or without the drain |
| M5 | A round 1 | `SessionStop` racing a start/respawn in flight: the spawn registers after the stop reported `Idle` | Pre-existing for the sidebar's stop (the old kill-only arm raced the same spawns); the start gate serialization is feature 026's. Follow-up below |
| M5 | A round 1 | `headline` repeats `operation_phrase`'s delete-worktree suffixes | Cosmetic; two short phrases in one file |
| M5 | A round 1 | `wire_operation` mirrors `policy::ConfirmedOp` onto the protocol's `ConfirmOperation` | Layering: the policy type is daemon-internal and the wire type is the versioned protocol; a `From` in core would tie the policy to protocol 18. `SendInput` arrives with M6 |

## Open escalation

None.

Resolved 2026-10-01, category 4 (environment): the disk was too full for the M5 gate (3.2 GB free). The user approved `SWEEP_ARGS='--maxsize 50GB' mise run sweep`; `/` then had 21 GB free.

## Follow-ups not done

- `create_session` builds several full catalog snapshots per call (`caller_project` / `get_session`); reuse the one `resolve_caller` returns (M3 review A).
- An unbracketed first prompt with inner newlines submits line by line; only reachable when the CLI has not enabled bracketed paste by the time it is ready (M3 review A).
- Hook receiver's `--settings` token file is written with the default umask, not owner-only (`crates/micold-daemon/src/hooks.rs` `prepare_settings`); outside this feature (research R7).
- Pi tool-server binding via a `-e` bridge extension (research R4).
- U127 (`list_worktrees` reports `status: locked`): the daemon cannot produce it. `micold_core::worktree::WorktreeStatus` has no locked state and `wire_worktree_status` never yields `WorktreeStatus::Locked`, so T015/T021 stay unticked on that one behavior. Needs worktree discovery to parse porcelain `locked` (core + sidebar), outside feature 034's files.
- T002 deviation: the MCP test fixture is included by `#[path = "support/mcp.rs"]` instead of re-exported from `tests/support/mod.rs`, so the framer helpers do not become dead code (a clippy `-D warnings` failure) in the MCP test binaries.
- Tool-server listener read timeouts (slowloris), shared with the hook receiver through `http.rs` (M1 review A).
- Windows: verify the `.claude.json` project-key form the collision check uses (M1 review A).
- Windows: port `mcp_binding_spawn.rs` (unix `#!/bin/sh` stand-ins) to Windows stand-ins (M1 review B).
- Daemon `SettingsSet`: apply all fields as one catalog update with one persist and one `SettingsChanged` (today each field short-circuits, persists and broadcasts separately, and a failed persist leaves memory changed). Pre-existing across all service-owned settings (M2 review A).
- Flaky: `crates/micold-core/tests/github_gh_cli.rs::a_typed_error_stands_at_any_exit_status` failed once with `ToolMissing` (stub executable race) during the M4 gate; passed on rerun. Not this feature's code.
- `SessionStop` (sidebar or `stop_session`) racing a start/respawn in flight: the spawn can register a live process after the stop reported `Idle` (M5 review A).
- M5 merge note: M4 added `policy::ConfirmedOp` and `PolicyDecision::Confirm`; M5's prebuilt protocol `ConfirmOperation` should be mapped from it.
- Client: a Settings save made while disconnected is overwritten by the daemon's `Welcome` on reconnect, for every service-owned field (M2 review A).
