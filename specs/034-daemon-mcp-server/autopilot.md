# Autopilot ledger — 034-daemon-mcp-server

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: "the daemon server should expose mcp server that will be automaticly binded into AI sessions. Should allow to manage the sessions and worktries."
- **Kind**: feature
- **Worktree branch**: feat/daemon-should-expose-mcp-server-for-agent
- **Started**: 2026-09-29
- **Phase**: 4-milestone M7
- **Next step**: M7 PR #521 open: the orchestrator waits on CI and merges. Then Phase 5 (close).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #455 | Spec | merged | 8ff6e0561386ba21219411bc16f828f08b177961 |
| #465 | Design (PR 2) | merged | a5fcc795294b6dcd2972ab5e3d31a814b41d8fb7 |
| #469 | M1: bound sessions with the read tools | merged | 3c8d332ddc4f4497d0caa1764ae2fbf2c301c6f3 |
| #474 | M2: Settings toggle for the tool server | merged | 7964273e |
| #498 | M3: create_worktree / create_session | merged | 2486d096bfb6ddbba8f6b6cad1f2489f3a2e481c |
| #509 | M4: start_session / rename_worktree | merged | f19454dcccf06d54c8da403a7a05e3931a0152b8 |
| #516 | M5: confirmations and the destructive tools | merged | fc1c3ad207bd2439035d0d66047034d7304d7907 |
| #519 | M6: cross-session read and send; Default may create a worktree | merged | e108619b041f290b415b62f1a76c95bfbe1b4801 |
| #521 | M7: read latency, sandbox placement, the full real-CLI pass, final docs | open | |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T022, T078–T082, T100 | Bound sessions (Claude, Copilot) with the read tools | #469 | merged |
| M2 | T023–T028, T083, T101 | Settings toggle for the tool server (protocol 17) | #474 | merged |
| M3 | T029–T042, T072–T073, T084–T088, T102, T105 | create_worktree / create_session with first prompt, audit line | #498 | merged |
| M4 | T043–T048, T089 | start_session / rename_worktree; policy rows for the destructive tools | #509 | merged |
| M5 | T049–T059, T090–T094, T103 | Confirmations in app windows; destructive tools (protocol 18) | #516 | merged |
| M6 | T060–T071, T095–T099, T104 | Cross-session read/send under the FR-016 setting (protocol 19); a Default session may create a worktree (constitution 1.7.0) | #519 | merged |
| M7 | T074–T077 | Read latency, sandbox placement, final real-CLI pass, user guide | #521 | PR open |

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
| D21 | 4-milestone M6 | May a Default session create a worktree through `create_worktree`? (supersedes D1 for create) | Yes: constitution 1.7.0 amends Principle III with that exception; rename and delete stay refused from Default (FR-015a) | _(decided by user)_ | .specify/memory/constitution.md 1.7.0, constitution-history.md; spec.md#FR-015a; `mcp_create_worktree.rs::a_default_session_creates_a_worktree_as_any_session_does` |
| D22 | 4-milestone M6 | What do `read_session_output` and `send_session_input` answer for a target with no running primary process? | `conflict`, naming `start_session`, for both; checked after policy and before any confirmation, so nobody is asked about a send that cannot be typed | agent-resolved | contracts/mcp-tools.md *Order of checks*; FR-012a's `interrupt_session` precedent; `mcp_cross_session.rs::a_session_that_is_not_running_is_a_conflict_naming_start_session` |
| D23 | 4-milestone M6 | Should `send_session_input` type into a session whose CLI may be showing its folder-trust question, and may the text hold control characters? | No to both. The D20 trust check runs before any confirmation and answers `conflict`; a control character other than a line break or a tab is `invalid_input` (Ctrl-C would be an unconfirmed interrupt, against FR-014) | agent-resolved | M6 review A F1, F2; D20; contracts/mcp-tools.md; `mcp_cross_session.rs::a_cli_that_would_ask_to_trust_the_folder_gets_nothing_typed`, `::line_breaks_alone_and_control_characters_are_invalid_input` |

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
| M5 | A (sonnet re-review) | 2 | ac6aaebe56cdac342e724f809336270975084e90:8e2052f9 | CLEAN; the 8 fixes hold |
| M5 | B (conformance) | 1 | 3161139f516b488e6189b6a5b65b139385d8284a:5c4bf06c5e3cbbe45974430cfcffe78a72d8eb74 | CHANGES; Verify green (mcp_audit_log 4, mcp_confirmations 18, mcp_lifecycle_tools 24, features_agent_confirm 12, layout_snapshot 43, protocol_roundtrip 9, schema_hash 9, mcp_tools_catalog 19, mcp_policy 12). F1 MAJOR untested review A fixes (fixed: 2 tests with red evidence, cycle 34; the untestable `delete_session` re-resolve removed), F3 MINOR (follow-up), F4 MINOR (guide fixed), F2 MINOR declined |
| M5 | visual pass §B4 | 1 | 3161139f516b488e6189b6a5b65b139385d8284a:5c4bf06c5e3cbbe45974430cfcffe78a72d8eb74 | PASS, 6 of 6 checks (evidence/m5-b4-confirm-dialog) |
| M5 | B (sonnet re-review) | 2 | a0030724f7b0e34ff5dbab78a3e584ca02dee3b6:26e750151e1c667231867949df627b2dd0646364 | CLEAN; fixes hold, declines stand; Verify green (mcp_confirmations 19, mcp_lifecycle_tools 25, mcp_audit_log 4); 1 MINOR fixed (guide rewrapped) |
| M6 | A (code-review high) | 1 | 800ded94aefc484ad7bb8561f2c9012142d58436:95195525b563af7cddf21fe81c0001b415c3d254 | CHANGES, 7 findings. Fixed: F1 control characters in the text are `invalid_input` (Ctrl-C was an unconfirmed interrupt); F2 nothing is typed into a CLI that would ask to trust its folder (`conflict`, D23); F3 text of line breaks only is `invalid_input`; F5 the bracketed-paste test waits for a raw-mode marker. Declined: F4, F6, F7 and F1's unbracketed inner line break (below) |
| M6 | B (conformance) | 1 | 800ded94aefc484ad7bb8561f2c9012142d58436:5fd07a884c05b1fcdcd22d2a2b81a1120dd76497 | CLEAN; Verify green (test-core 1469: mcp_policy 18, mcp_tools_catalog 29, settings_roundtrip 28, protocol_roundtrip 10; mcp_cross_session 17, scrollback_range 11, mcp_audit_log 5, mcp_create_worktree 12, mcp_lifecycle_tools 25; features_settings 63, layout_snapshot 43). 3 MINOR, all fixed (contract rows name D22's conflict; T030/T036 note D21; constitution-history's follow-ups marked landed) |
| M6 | visual pass §B5, §B6 | 1 | 800ded94aefc484ad7bb8561f2c9012142d58436:5fd07a884c05b1fcdcd22d2a2b81a1120dd76497 | PASS, 5 of 5 checks (evidence/m6-b5-b6-cross-session) |
| M6 | A (sonnet re-review) | 2 | 505b4f9293f1e227962c8633669f484f24450d8e:0cd5dd5429c6b1049e8fa805a2f703ac96347bb7 | CLEAN; the fixes hold, the declines stand |
| M6 | B (sonnet re-review) | 2 | 505b4f9293f1e227962c8633669f484f24450d8e:0cd5dd5429c6b1049e8fa805a2f703ac96347bb7 | CLEAN; Verify green (test-core 1471; mcp_cross_session 19, scrollback_range 11, mcp_audit_log 5, mcp_create_worktree 12, mcp_lifecycle_tools 25; features_settings 63, layout_snapshot 43). 1 MINOR fixed (FR-012a states D23's refusals). Not counted: run because review A's fixes changed code |
| M7 | visual pass §B1–§B3 (real CLIs) | 1 | binaries of aacbd7cd | PASS, 13 of 13 results (evidence/m7-b1-b3-real-cli) |
| M7 | visual pass §B1 step 7, §B4–§B6 (real CLI) | 1 | binaries of aacbd7cd | PASS, 11 of 11 checks (evidence/m7-b4-b6-real-cli) |
| M7 | A (code-review high) | 1 | 2c2187a68ac5cb2404919efd4024ecf8da78a577:9f352fa5ab6df230f0aaf8ade688562ada991e83 | CLEAN; 3 MINOR, all declined as follow-ups (below) |
| M7 | B (conformance) | 1 | 2c2187a68ac5cb2404919efd4024ecf8da78a577:9f352fa5ab6df230f0aaf8ade688562ada991e83 | CLEAN; Verify green (mcp_audit_log 5, mcp_read_latency 1; sandbox record checked: cycle 41, 29 passed). 3 MINOR: 2 fixed (T076 names the Copilot 55 s step that stands on M5's evidence; the B1–B3 count is 13 of 13), 1 declined |

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
| M5 | B round 1 | F2: cycle 33's two client design changes have no observed red run | MINOR; the changed assertions in `overlay_registry.rs` and `features_agent_confirm.rs` are the red evidence, and replaying phase A's reducer under them needs a checkout of another commit's file only to paste output |
| M5 | A round 1 | `delete_session` re-archives an already archived session and reports success (the `owner.is_none()` branch is not reachable) | The re-resolve added for it was removed in review B: no test can reach it, since every archive path withdraws the pending prompt first and the request ends `not_found`. What remains is a race of microseconds between the answer and the archive, whose outcome (the session is deleted) is what was asked |
| M5 | A round 1 | `SessionStop` racing a start/respawn in flight: the spawn registers after the stop reported `Idle` | Pre-existing for the sidebar's stop (the old kill-only arm raced the same spawns); the start gate serialization is feature 026's. Follow-up below |
| M5 | A round 1 | `headline` repeats `operation_phrase`'s delete-worktree suffixes | Cosmetic; two short phrases in one file |
| M5 | A round 1 | `wire_operation` mirrors `policy::ConfirmedOp` onto the protocol's `ConfirmOperation` | Layering: the policy type is daemon-internal and the wire type is the versioned protocol; a `From` in core would tie the policy to protocol 18. `SendInput` arrives with M6 |
| M6 | A round 1 | F4: `plain_tail`'s `truncated` is false for an unviewed session whose scrollback overflowed, when the request covers everything retained | Reachable only with the scrollback limit set below the 2,000-line maximum of a read (default 10,000): otherwise an overflowed terminal retains more than any request and `start > 0` already says `true`. The flag is a hint and no returned line is wrong. Follow-up below |
| M6 | A round 1 | F1 (part): an inner line break submits early when the target has not enabled bracketed paste | Already a follow-up from M3 review A for the first prompt; the same `encode_submission`. Both real CLIs enable bracketed paste before they take input |
| M6 | A round 1 | F6: a mistyped `cross_session_access` in a hand-edited settings file fails the whole document | MINOR; the same strict parsing as every other enum in the settings file, and the file is written by the app. Follow-up below |
| M6 | A round 1 | F7: the PTY write is a blocking `write_all` on the runtime thread; PTY and framer are read under two locks | MINOR; the same write as the first prompt's (M3) and a window's keystrokes. A restart between the two reads answers `conflict`, which is true an instant later. Follow-up below |
| M7 | A round 1 | F1: an unknown `cross_session_access` reads as `Off` with no log line, and the next save stores `"off"` over it | MINOR; reading it as `Off` is the choice of `aacbd7cd` (U233): the safe value, and the rest of the file is kept. Keeping the unknown stored value across a save and logging it is a follow-up below |
| M7 | A round 1 | F2: `mcp_read_latency.rs` kills the started session's PTY inline, not in a `Drop` guard, so a panic leaves it to `PtySession::drop` | MINOR, test-only; the gate was already running on this tree. Follow-up below |
| M7 | A round 1 | F3: `sandbox_real_mcp.rs`'s host-loopback probe has no timeout when a foreign listener accepts and never answers | MINOR, test-only, in a suite that is off by default; not re-verified by the reviewer. Follow-up below |
| M7 | B round 1 | F3: `aacbd7cd` changes `settings.rs` under M7 with no task row | MINOR; a new task would change M7's range (T074–T077) after the milestones were cut (D18). It is traceable through U233, cycle 40, this ledger (M6 review A F6) and the PR body |

## Handover

None.

## Open escalation

None.

Resolved 2026-10-01, category 4 (environment): the disk was too full for the M5 gate (3.2 GB free). The user approved `SWEEP_ARGS='--maxsize 50GB' mise run sweep`; `/` then had 21 GB free.

Resolved 2026-10-01, category 4 (environment): the M5 gate ran out of disk during `cargo test --workspace` (21 GB was not enough for a full rebuild). The sweep was re-run and the Go build cache cleaned (`go clean -cache`), with the user's approval; `/` then had 75 GB free. No file was truncated.

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
- Windows: seven M5 tests in `mcp_lifecycle_tools.rs` are `#[cfg(unix)]` (the fixture's sessions are `#!/bin/sh` stand-ins), so Windows CI does not run an allowed stop, interrupt or delete through `/mcp`; port the stand-ins (M5 review B).
- Client: a Settings save made while disconnected is overwritten by the daemon's `Welcome` on reconnect, for every service-owned field (M2 review A).
- `read_session_output`'s `truncated` can be false for a session nobody views whose scrollback (set below 2,000 lines) overflowed: `Framer::scrolled_off` advances only in `frame()`. Derive it from the grid or count evictions in the PTY reader (M6 review A).
- `send_session_input` and the first prompt write to the PTY with a blocking `write_all` on a runtime thread, with text bounded only by the 1 MiB request body; move it to `blocking` and cap the text (M6 review A).
- A theme change saved in Settings shows only after the client restarts (seen in the M5 and M6 visual passes); not this feature's code.
- Unconfirmed, from the M7 visual passes: worktrees made with plain `git worktree add` under `.claude/worktrees/` before the app first opened the project (seeded through `projects.json`) were not listed in the sidebar, even after Refresh. Not investigated; not this feature's code.
- `read_session_output` is timed by `mcp_read_latency.rs` on unix only (the stand-in CLI is `#!/bin/sh`); goes with the Windows stand-in port above.
- An unknown `cross_session_access` in the settings file is read as `Off` silently and overwritten with `"off"` on the next save; log the unrecognised value and keep the stored one while the in-memory value is the fallback (M7 review A).
- `mcp_read_latency.rs`: move the PTY kill into a `Drop` guard, as `mcp_cross_session.rs` and `mcp_lifecycle_tools.rs` do (M7 review A).
- `sandbox_real_mcp.rs`: wrap the host-loopback `post_mcp` probe in a timeout (M7 review A).
