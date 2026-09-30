---
feature: 034-daemon-mcp-server # spec-kit feature directory name
loop: outside-in # outside-in | inside-out
profile: .specify/memory/tdd-profile.md # stack profile the commands must read
spec_criteria: 22 # acceptance scenarios in spec.md (US1 6, US2 5, US3 6, US4 5)
planned_at: d12893f4 # short SHA the list was derived from
updated_at: d12893f4 # short SHA of the last change to this file
suite_baseline: green # fast subset `mise run test-core` at d12893f4: 1264 passed, 0 failed, 6 ignored
# The workspace suite was not re-run here (slow, shared build lock). Its baseline is CI's green
# `main` at 13967080 (the merge-base of this branch; ci.yml run conclusion: success). The branch
# adds only docs on top of it, so the workspace suite is green at planned_at by construction.
workspace_suite_baseline: ci-green-main@13967080
---

# Test List: The session service exposes an MCP server to the AI sessions it runs

## How to read the traces

`USn-ASm` is acceptance scenario *m* of user story *n* in `spec.md`. `FR-*` and `SC-*` are the
spec's requirement and success-criterion ids. The spec's *Edge Cases* carry no ids, so they are
recorded here as invariants with a stable id, each naming the edge case it quotes:

| id | spec.md *Edge Cases* entry |
|---|---|
| EC-1 | Empty project |
| EC-2 | Unknown or malformed target |
| EC-3 | Invalid names |
| EC-4 | Concurrency (Principle II) |
| EC-5 | Several windows attached |
| EC-6 | No window attached |
| EC-7 | Service restart |
| EC-8 | Stale credential |
| EC-9 | Target changes while a confirmation is pending (FR-014) |
| EC-10 | Hidden assistant-owned worktrees (feature 014) |
| EC-11 | Session respawn |
| EC-12 | Sandboxed placement (feature 027) |
| EC-13 | Unsupported CLI |
| EC-14 | User already configured a server with the same name |
| EC-15 | Many requests |
| EC-16 | Long outputs |
| EC-17 | Cross-platform (Principle VI) |
| EC-18 | Disabled |

Invariants recorded from `plan.md` / `tasks.md` rather than `spec.md`, with their rationale:

| id | invariant | rationale |
|---|---|---|
| INV-1 | Behaviour of code moved or rewired by this feature (hook receiver, `route()` worktree arms, `SessionStop`) is unchanged for the existing user path except where a behaviour below says otherwise | plan.md *Risks* (`route()` extraction regressions); FR-009 requires the agent path to equal the user path, which only holds if the user path is pinned first |
| INV-2 | Every wire change bumps `PROTOCOL_VERSION` once and the `schema_hash.rs` pin moves with it | tasks.md *Protocol*; contracts/protocol-delta.md |
| INV-3 | No code outside `crates/micold-core/src/provider.rs` matches on the concrete AI CLI | plan.md *Constraints* (feature 026 rule) |

## Outer loop: acceptance behaviors

One per acceptance scenario (US3-AS2 is two behaviors: it states two results joined by
"instead"). The highest level the default suite can reach is a **daemon integration test**: a real
`DaemonState`, a real TCP `POST /mcp` with the session's bearer credential, a stand-in CLI spawned
through the real spawn path, fake windows registered with `state.register`. That is weaker than an
end-to-end run with the real `claude` / `copilot` binaries, which the default suite cannot host; the
real-CLI half of each scenario is quickstart §B, run per milestone (T100–T104) and re-run in full by T076, and the container placement is
`sandbox_real_mcp.rs` (U147). The profile's acceptance runner (`sandbox_real_*`) hosts only U147.

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1 | A session S3 spawned in worktree `b` with a Claude stand-in, no user action, answers `tools/list` and `whoami` over `POST /mcp` with the credential from its own binding file | US1-AS1, FR-001, FR-002, SC-001 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| A2 | S3's `list_worktrees` returns `default`, `a` and `b` with branch, status and `app_created`, the set the sidebar's snapshot holds | US1-AS2, FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| A3 | S3's `list_sessions` returns S1, S2, S3 with label, AI CLI, lifecycle, activity and worktree, and only S3 has `is_caller: true` | US1-AS3, FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| A4 | After bound spawns, fixture `~/.claude.json`, `~/.claude/settings.json`, `<project>/.mcp.json` and `~/.copilot/mcp-config.json` hash identically to before | US1-AS4, FR-003, SC-002 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| A5 | A Pi session spawns with exactly its pre-feature argv and one `info` line "no tool server: Pi has no MCP support" (the user-guide half is not testable, see *Out of scope*) | US1-AS5, FR-005, EC-13 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| A6 | With `tool_server_enabled = false` a new session spawns unbound with "disabled in settings" logged, and a session bound before the change still answers `whoami` | US1-AS6, FR-004, EC-18 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| A7 | `create_worktree {branch: feat-x}` from a worktree session creates `.claude/worktrees/feat-x` recorded app-created, a fake window receives `CatalogChanged` holding it within 2 s, and the result carries its ref, branch and path | US2-AS1, FR-009, FR-011, SC-003 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |
| A8 | `create_worktree` for a branch that exists or is checked out elsewhere fails `conflict` naming the pre-flight situation, and the worktree list and directory tree are unchanged | US2-AS2, FR-013 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |
| A9 | `create_worktree` then `create_session {worktree: feat-x, ai_cli, prompt}` (two calls) creates and starts a session a fake window sees, and the stand-in CLI receives the prompt as its first submitted input after its ready signal (`prompt_delivered: true`) | US2-AS3, FR-017, SC-007 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| A10 | `create_session` without `ai_cli` creates a session running the Settings `default_ai_cli` | US2-AS4 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| A11 | `create_session` with a CLI not on the session environment's `PATH` fails `service_error` naming that CLI, and no session record exists afterwards | US2-AS5, FR-013 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| A12 | `start_session` on an `Idle`, a `Failed` and an `InterruptedResumable` session moves each through `Starting` to `Running`, as the protocol start does | US3-AS1, FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| A13 | An allowed `stop_session` on a `Running` sibling ends its processes, every fake window receives it as `Idle`, and a later `start_session` resumes its conversation | US3-AS2, FR-009, FR-011 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| A14 | An allowed `interrupt_session` on a `Running` sibling writes `0x03` to its primary PTY and it stays `Running` | US3-AS2 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| A15 | `delete_worktree` on a worktree with live sessions and no `stop_sessions` fails `conflict` naming those sessions, and the worktree, its sessions and its branch are unchanged | US3-AS3, FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| A16 | Each of `delete_worktree`, `delete_session`, `stop_session` and `interrupt_session` on another target raises a confirmation in the fake windows and changes nothing until it is answered | US3-AS4, FR-014 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| A17 | `stop_session` and `delete_session` on the caller, and `delete_worktree` of the caller's hosting worktree, fail `refused_by_policy` with no confirmation raised and nothing changed | US3-AS5, FR-015 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| A18 | From a Default session, `create_worktree`, `rename_worktree` and `delete_worktree` each fail `refused_by_policy` naming Principle III and nothing changes | US3-AS6, FR-015a | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| A19 | At Auto and at Confirm each send, S1's `read_session_output` on sibling S2 returns S2's last N primary-terminal lines as plain text with no confirmation raised | US4-AS1, FR-012, FR-016 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |
| A20 | At Auto, S1's `send_session_input` to an awaiting-input S2 writes the text to S2's primary PTY as typed and submitted (ends `\r`) | US4-AS2, FR-016 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |
| A21 | Every session-targeting and worktree-targeting tool aimed at another project's session or worktree fails `not_found` with the same message as an unknown ref | US4-AS3, FR-010, EC-2 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |
| A22 | At Off, `read_session_output` and `send_session_input` on S2 fail `refused_by_policy` and S2's PTY receives nothing | US4-AS4, FR-016 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |
| A23 | At Confirm each send, `send_session_input` raises a confirmation: allowed delivers as A20, declined fails `refused_by_policy`, timed out or no window fails `needs_confirmation`, S2 untouched in the last three | US4-AS5, FR-014, FR-016 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |

## Inner loop: unit behaviors

Grouped by the component in plan.md *Project Structure*. Characterization baselines come first in
each group that changes existing code.

### `crates/micold-daemon/src/http.rs` (extracted from `hooks.rs`; T003, T007)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1 | The hook receiver's end-to-end behaviour (auth, bounds, activity, `SessionStart` accepted without a transition) is unchanged | INV-1 | characterization | BASELINE | `crates/micold-daemon/tests/hooks_receiver.rs` (whole file, existing) |
| U2 | A well-formed head parses method, path and case-insensitive headers; a malformed request line is rejected | INV-1 | characterization | BASELINE | `crates/micold-daemon/src/http.rs::tests::{parses_a_well_formed_hook_request_head, header_names_are_case_insensitive, a_malformed_request_line_is_rejected}` (moved from `hooks.rs`) |
| U3 | `find_head_end` reports no head until the blank line arrives | INV-1 | characterization | BASELINE | `crates/micold-daemon/src/http.rs::tests::find_head_end_needs_the_blank_line` (moved from `hooks.rs`) |
| U4 | A body exactly at the caller-chosen limit is read in full | FR-001 | example | DONE | `crates/micold-daemon/src/http.rs::tests::a_body_exactly_at_the_limit_is_read_in_full` |
| U5 | A body one byte over the caller-chosen limit is refused as too large and drained | FR-001 | example | DONE | `crates/micold-daemon/src/http.rs::tests::a_body_one_byte_over_the_limit_is_refused_and_drained` |

### `crates/micold-core/src/mcp/jsonrpc.rs`, `errors.rs` (T004, T008)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U6 | A request with an `id` parses as a request carrying that id and method | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U7 | A message without an `id` parses as a notification | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U8 | Malformed JSON answers error `-32700` with `id: null` | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U9 | An unknown method answers error `-32601` | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U10 | `initialize` echoes each of `2025-03-26`, `2025-06-18`, `2025-11-25`, `2026-07-28` when the client sends it | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U11 | `initialize` with any other `protocolVersion` answers `2026-07-28` | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U12 | `initialize` declares `capabilities.tools.listChanged = false` and `serverInfo.name = "micold"` | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U13 | `ping` answers `{}` | FR-001 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U14 | A success result serialises as `content[0].text` + `structuredContent` + `isError: false` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U15 | A failure serialises as `"<category>: <message>"` text + `structuredContent.error{category,message}` + `isError: true` | FR-013 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |
| U16 | Each of the six categories serialises as its snake_case name | FR-013, SC-010 | example | PENDING | planned: `crates/micold-core/tests/mcp_jsonrpc.rs` |

### `crates/micold-daemon/src/mcp/server.rs`, `credentials.rs` (T005, T010)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U17 | The listener's address is `127.0.0.1` with an ephemeral port | FR-001, FR-007 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U18 | A request with no `Authorization` header answers `401` with an empty body | FR-006, SC-006 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U19 | A malformed `Authorization` header answers `401` byte-identical to U18 | FR-006, SC-006 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U20 | An unknown bearer answers `401` byte-identical to U18 | FR-006, SC-006, FR-007 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U21 | `GET /mcp` and `DELETE /mcp` answer `405` | FR-001 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U22 | Any path other than `/mcp` answers `404` with an empty body | FR-001 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U23 | A head over 8 KiB answers `431` | FR-001 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U24 | A body over 1 MiB answers `413` | FR-001 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U25 | `notifications/initialized` answers `202` with an empty body | FR-001 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U26 | Two sessions receive different credentials, and each credential's `whoami` names its own session | FR-006, EC-4 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U27 | A credential answers `401` after its session is deleted | FR-006, SC-006, EC-8 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U28 | A fresh `DaemonState` (service restart) answers `401` to every credential issued before it | FR-006, EC-7, EC-8 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |
| U29 | Neither the request body nor the credential appears in the log at any level | FR-006, FR-018 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_endpoint.rs` |

### `crates/micold-daemon/src/platform/{mod,unix,windows}.rs` `write_owner_only` (T006, T009)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U30 | On Unix the created directory has mode `0700` | FR-007, SC-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_file_mode.rs` |
| U31 | On Unix the written file has mode `0600` | FR-007, SC-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_file_mode.rs` |
| U32 | Rewriting an existing file replaces its bytes and it stays `0600` | FR-007 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_file_mode.rs` |
| U33 | On Windows the file's DACL is protected and its only ACE grants the current user's SID (red/green observable only on the `windows-latest` CI leg) | FR-007, SC-009, EC-17 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_file_mode.rs` |

### `crates/micold-core/src/provider.rs` (T013, T017, T033, T039)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U34 | No code outside `provider.rs` names a concrete CLI implementation | INV-3 | characterization | BASELINE | `crates/micold-client/tests/no_concrete_implementations.rs` (existing) |
| U35 | Claude's `tool_server_support()` is `McpConfigArg` | FR-002 | example | PENDING | planned: `crates/micold-core/tests/ai_cli_provider_seam.rs` |
| U36 | Copilot's `tool_server_support()` is `AdditionalMcpConfig` | FR-002 | example | PENDING | planned: `crates/micold-core/tests/ai_cli_provider_seam.rs` |
| U37 | Pi's `tool_server_support()` is `Unsupported { reason: "Pi has no MCP support" }` | FR-002, FR-005, EC-13 | example | PENDING | planned: `crates/micold-core/tests/ai_cli_provider_seam.rs` |
| U38 | Claude's `input_readiness()` is `HookSessionStart` | FR-017 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |
| U39 | Pi's `input_readiness()` is `ExtensionEvent("session_start")` | FR-017 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |
| U40 | Copilot's `input_readiness()` is `OutputSettled` | FR-017 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |

### `crates/micold-core/src/mcp/binding.rs` (T011, T012, T018)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U41 | For `McpConfigArg` the plan's last arguments are `--mcp-config <file> --allowedTools mcp__micold`, with nothing after them | FR-002 | example | PENDING | planned: `crates/micold-core/tests/mcp_binding_plan.rs` |
| U42 | The Claude file is exactly `{"mcpServers":{"micold":{"type":"http","url":…,"headers":{"Authorization":"Bearer <cred>"},"timeout":120000}}}` | FR-002, FR-006 | example | PENDING | planned: `crates/micold-core/tests/mcp_binding_plan.rs` |
| U43 | For `AdditionalMcpConfig` the plan's last arguments are `--additional-mcp-config @<file> --allow-tool micold` | FR-002 | example | PENDING | planned: `crates/micold-core/tests/mcp_binding_plan.rs` |
| U44 | The Copilot file is the Claude entry plus `"tools":["*"]` | FR-002 | example | PENDING | planned: `crates/micold-core/tests/mcp_binding_plan.rs` |
| U45 | For `Unsupported` the outcome is `Skipped(Unsupported(reason))` with no arguments | FR-005, EC-13 | example | PENDING | planned: `crates/micold-core/tests/mcp_binding_plan.rs` |
| U46 | No plan ever contains `--strict-mcp-config` | FR-003 | example | PENDING | planned: `crates/micold-core/tests/mcp_binding_plan.rs` |
| U47 | `mcpServers.micold` at the top level of `~/.claude.json` is detected as taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U48 | `mcpServers.micold` under the session's project path in `~/.claude.json` is detected as taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U49 | `mcpServers.micold` under a different project's path in `~/.claude.json` is not taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U50 | With `CLAUDE_CONFIG_DIR` set, an entry in `$CLAUDE_CONFIG_DIR/.claude.json` is taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U51 | With `CLAUDE_CONFIG_DIR` set, an entry only in `~/.claude.json` is not taken (assumption A-3) | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U52 | An entry in `<cwd>/.mcp.json` is taken for Claude | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U53 | An entry in Copilot's `mcp-config.json` is taken for Copilot | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U54 | A server with any other name (e.g. `micold2`) is not taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U55 | An absent file is not taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U56 | A malformed file is not taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U57 | An unreadable file is not taken | FR-003, EC-14 | example | PENDING | planned: `crates/micold-core/tests/mcp_name_collision.rs` |
| U58 | Each `SkipReason` renders exactly its contracts/binding.md §5 text | FR-005 | example | PENDING | planned: `crates/micold-core/tests/mcp_binding_plan.rs` |

### `crates/micold-daemon/src/state.rs` spawn wiring (T016, T020, T024, T026)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U59 | A Claude session's spawn argv ends with the binding arguments | FR-002 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U60 | A Copilot session's spawn argv ends with `--additional-mcp-config @<file> --allow-tool micold` | FR-002 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U61 | The binding file is `<data_dir>/mcp/<uuid>.json` and is owner-only | FR-003, FR-007 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U62 | Twenty bound spawns leave every fixture configuration file byte-identical | SC-002, FR-003 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U63 | A name collision spawns the session unbound, with its pre-feature argv, logging the colliding path | FR-003, FR-005, EC-14 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U64 | A respawn of the same session reuses its credential and rewrites its file | FR-006, EC-11 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U65 | A session respawned after a crash (`Restarting`) answers `whoami` with its credential | EC-11 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U66 | An `InterruptedResumable` session started after a service restart gets a new credential in its file, and the pre-restart one answers `401` | EC-7, FR-006 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U67 | A Regular-terminal session gets no binding and no tool-server log line | FR-002 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U68 | With no tool server running, an AI-CLI session starts unbound with "tool server unavailable" logged | FR-005 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U69 | When the binding file cannot be written, the session still starts, unbound, with "could not write the binding: …" logged | FR-005 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U70 | Each unbound AI-CLI spawn logs exactly one skip line | FR-005 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |
| U71 | Re-enabling `tool_server_enabled` binds the next spawn again | FR-004 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_binding_spawn.rs` |

### `crates/micold-core/src/settings.rs`, `protocol/{messages,version}.rs` (T023, T026, T049, T053, T062, T067)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U72 | `tool_server_enabled` defaults to `true` | FR-004 | example | PENDING | planned: `crates/micold-core/tests/settings_roundtrip.rs` |
| U73 | A settings file written before the field loads `tool_server_enabled = true` | FR-004 | example | PENDING | planned: `crates/micold-core/tests/settings_roundtrip.rs` |
| U74 | `tool_server_enabled = false` survives a save/load round trip | FR-004 | example | PENDING | planned: `crates/micold-core/tests/settings_roundtrip.rs` |
| U75 | `DaemonSettings` and `SettingsSet` round-trip `tool_server_enabled` | FR-004 | example | PENDING | planned: `crates/micold-core/tests/protocol_roundtrip.rs` |
| U76 | `cross_session_access` defaults to `Auto` | FR-016 | example | PENDING | planned: `crates/micold-core/tests/settings_roundtrip.rs` |
| U77 | A settings file written before the field loads `Auto` | FR-016 | example | PENDING | planned: `crates/micold-core/tests/settings_roundtrip.rs` |
| U78 | `ConfirmEachSend` and `Off` each survive a save/load round trip | FR-016 | example | PENDING | planned: `crates/micold-core/tests/settings_roundtrip.rs` |
| U79 | `DaemonSettings` and `SettingsSet` round-trip `cross_session_access` | FR-016 | example | PENDING | planned: `crates/micold-core/tests/protocol_roundtrip.rs` |
| U80 | `ConfirmationRequested`, `ConfirmationWithdrawn` and `ConfirmationAnswer` round-trip | FR-014 | example | PENDING | planned: `crates/micold-core/tests/protocol_roundtrip.rs` |
| U81 | `ConfirmOperation::SendInput` has no field that can carry input text | FR-018 | example | PENDING | planned: `crates/micold-core/tests/protocol_roundtrip.rs` |
| U82 | Each wire change moves `PROTOCOL_VERSION` and the pinned schema hash together | INV-2 | example | PENDING | `crates/micold-core/tests/schema_hash.rs` (existing; pin updated per bump) |

### `crates/micold-core/src/mcp/tools.rs` catalog and validation (T014, T019, T031, T037, T044, T046, T061, T066)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U83 | `tools/list` names exactly the tools whose handlers ship in the current milestone | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U84 | Every listed tool has a JSON-Schema `inputSchema`, `readOnlyHint` on read-only tools | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U85 | `destructiveHint` is set on exactly `delete_worktree`, `delete_session`, `stop_session`, `interrupt_session` | FR-008, FR-014 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U86 | An unknown tool name fails `invalid_input` | FR-008, FR-013 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U87 | `list_worktrees` with a non-boolean `include_hidden` fails `invalid_input` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U88 | A `session` that is not a UUID string fails `invalid_input` | FR-008, EC-2 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U89 | `WorktreeRef` parses `default` as the project root and any other name as a named worktree | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U90 | `create_worktree` without `mode` means `new_branch` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U91 | `create_worktree` with `mode: track_remote` and no `remote` fails `invalid_input` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U92 | `create_worktree` with an `overwrite` field fails `invalid_input` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U93 | `create_worktree` without `branch` fails `invalid_input` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U94 | `create_session` accepts `ai_cli` of `claude_code`, `copilot`, `pi` and rejects any other value as `invalid_input` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U95 | `rename_worktree` or `delete_worktree` targeting `default` fails `invalid_input` | FR-008 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U96 | `delete_worktree` defaults to `stop_sessions: false`, `delete_branch: true` | FR-008, FR-009 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U97 | `read_session_output` without `lines` means 200 | FR-012 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U98 | `lines: 0` fails `invalid_input` | FR-012 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U99 | `lines: 1` is accepted as 1 | FR-012 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U100 | `lines: 2000` is accepted as 2000 | FR-012 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U101 | `lines: 2001` is clamped to 2000 | FR-012, EC-16 | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U102 | `send_session_input` with `text: ""` fails `invalid_input` | FR-012a | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |
| U103 | `send_session_input` with a one-character `text` is accepted (assumption A-4 for whitespace) | FR-012a | example | PENDING | planned: `crates/micold-core/tests/mcp_tools_catalog.rs` |

### `crates/micold-core/src/mcp/policy.rs` `decide` (T030, T036, T043, T046, T060, T066)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U104 | `create_worktree` from a Default caller is `refused_by_policy` naming Principle III | FR-015a | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U105 | `create_worktree` from a worktree caller proceeds | FR-015a | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U106 | `rename_worktree` from a Default caller is refused; from a worktree caller it proceeds | FR-015a | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U107 | `delete_worktree` from a Default caller is refused before any confirmation | FR-015a | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U108 | Every non-worktree operation from a Default caller is decided exactly as from a worktree caller | FR-015a | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U109 | `delete_worktree` of the caller's hosting worktree is `refused_by_policy` | FR-015 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U110 | `delete_worktree` of another worktree is `Confirm` | FR-014 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U111 | `stop_session` on self is `refused_by_policy`; on another session it is `Confirm` | FR-014, FR-015 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U112 | `delete_session` on self is `refused_by_policy`; on another session it is `Confirm` | FR-014, FR-015 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U113 | `interrupt_session` on self is `invalid_input`; on another session it is `Confirm` | FR-014, FR-015 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U114 | `start_session` proceeds | FR-009 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U115 | `read_session_output` on another session proceeds at Auto and at ConfirmEachSend, and is refused at Off | FR-016 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U116 | `send_session_input` on another session proceeds at Auto, is `Confirm(SendInput)` at ConfirmEachSend, and is refused at Off | FR-016 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |
| U117 | `read_session_output` and `send_session_input` on self are `invalid_input` under every option value, Off included | FR-015 | example | PENDING | planned: `crates/micold-core/tests/mcp_policy.rs` |

### `crates/micold-core/src/mcp/submission.rs` (T033, T039)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U118 | Bracketed encoding is `ESC[200~ text ESC[201~ \r` | FR-017, US4-AS2 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |
| U119 | Unbracketed encoding is `text \r` | FR-017, US4-AS2 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |
| U120 | A multi-line text encodes with exactly one trailing `\r` | US4-AS2 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |
| U121 | With no output yet, the settled rule never reports ready | FR-017 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |
| U122 | 1.5 s of silence after output reports ready; 1.499 s does not | FR-017 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |
| U123 | New output during the silence window restarts it | FR-017 | example | PENDING | planned: `crates/micold-core/tests/input_readiness.rs` |

### `crates/micold-daemon/src/mcp/tools.rs` read-only handlers (T015, T021)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U124 | `whoami` answers the caller's ref, project name and path, hosting worktree ref and AI CLI | FR-008, SC-001 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U125 | `list_worktrees` lists `default` first | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U126 | A worktree whose directory is gone reports `status: missing` | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U127 | A locked worktree reports `status: locked` | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U128 | A prunable worktree reports `status: prunable` | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U129 | Each worktree row's `session_count` equals the sessions it hosts | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U130 | An assistant-owned worktree is absent without `include_hidden` | EC-10 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U131 | With `include_hidden` it is present with `assistant_owned: true` | EC-10 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U132 | `list_sessions {worktree}` returns only that worktree's sessions | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U133 | `list_sessions {worktree}` with an unknown worktree fails `not_found` | FR-010, EC-2 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U134 | A Regular-terminal session is listed with `ai_cli: regular_terminal` | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U135 | `get_session` on a `Failed` session includes its `failure_reason`; on any other lifecycle it has none | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U136 | `list_branches` reports a branch checked out in a worktree with that worktree's ref, and a free one with `null` | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U137 | `list_branches` reports a remote-tracking branch with `kind: remote` | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U138 | An empty project answers `default` and the caller alone as success | EC-1 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U139 | An unknown session ref fails `not_found` | FR-010, EC-2 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |
| U140 | A returned worktree path uses the platform's native form (Windows separators on Windows; CI leg evidence) | EC-17 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_tools.rs` |

### `crates/micold-daemon/src/ops.rs` extraction (T029, T035)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U141 | The protocol worktree create / delete / rename paths behave as today (refusals without side effects, provenance, live-session refusal, stop-and-archive, branch keep/delete) | INV-1, FR-009 | characterization | BASELINE | `crates/micold-daemon/tests/mutation_semantics.rs::worktree_*`, `mutation_atomicity.rs`, `worktree_provenance_rpc.rs` (existing) |
| U142 | `ops::create_worktree` without a client yields the same catalog, provenance record and directory as `WorktreeCreate` | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/ops_extraction.rs` |
| U143 | `ops::delete_worktree` without a client yields the same catalog and disk result as `WorktreeDelete` | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/ops_extraction.rs` |
| U144 | `ops::rename_worktree` without a client yields the same catalog as `WorktreeRename` | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/ops_extraction.rs` |

### `crates/micold-daemon/src/mcp/tools.rs` `create_worktree` (T032, T038)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U145 | Without `name`, the worktree directory is `naming::dir_name_from_branch(branch)` | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |
| U146 | A name the naming rules reject fails `invalid_input` with the dialog's `NamingError` message and creates nothing | EC-3, FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |
| U148 | A branch name git rejects fails `invalid_input` with git's message and creates nothing | EC-3 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |
| U149 | `mode: existing_local` on an existing free branch succeeds | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |
| U150 | Ten concurrent creates of one branch from ten sessions give exactly one success, nine `conflict`, one worktree on disk | SC-005, EC-4 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |
| U151 | Back-to-back creates of distinct branches from one caller all succeed (no cap) | EC-15 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |

### `crates/micold-daemon/src/mcp/tools.rs` `create_session`, `state.rs` readiness (T034, T040, T041)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U152 | `create_session {worktree: default}` creates the session in the project root | FR-008 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U153 | `create_session` on an unknown worktree fails `not_found` and creates no record | FR-010, EC-2 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U154 | Without `prompt`, `create_session` returns `prompt_delivered: null` without waiting for readiness | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U155 | A ready signal just inside 60 s of the request delivers the prompt (`true`) | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U156 | No ready signal by 60 s after the request returns `prompt_delivered: false` | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U157 | A ready signal after the 60 s bound writes nothing to the PTY | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U158 | A session that fails to start returns `prompt_delivered: false` | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U159 | A `SessionStart` hook marks the session ready and leaves its activity `Unknown` | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U160 | A Pi `session_start` event marks the session ready | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U161 | A Copilot session becomes ready by the output-settled rule | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |
| U162 | A Claude session with no hook receiver running becomes ready by the output-settled rule | FR-017 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_session.rs` |

### `crates/micold-daemon/src/mcp/tools.rs` `start_session`, `rename_worktree` (T045, T047)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U163 | `start_session` on a `Running` session succeeds, reports `running`, and spawns nothing | FR-012a | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U164 | `start_session` on a `Starting` session succeeds and reports `starting` unchanged | FR-012a | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U165 | `start_session` on a `Restarting` session succeeds and reports `restarting` unchanged | FR-012a | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U166 | `rename_worktree` changes the display name and a fake window receives it in `CatalogChanged` | FR-009, FR-011, SC-003 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U167 | `rename_worktree` on an unknown worktree fails `not_found` | FR-010, EC-2 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |

### `crates/micold-daemon/src/state.rs` `stop_session`, `server.rs` `SessionStop` arm (T054)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U168 | `SessionStop` today kills the session's processes and drops it from the live registry | INV-1 | characterization | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` (write and see green before T054; becomes `BASELINE`) |
| U169 | `DaemonState::stop_session` leaves the record `Idle` | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U170 | `DaemonState::stop_session` keeps the session's credential valid | FR-006 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U171 | The protocol `SessionStop` now broadcasts a `CatalogChanged` with the session `Idle` | FR-009, FR-011 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |

### `crates/micold-daemon/src/mcp/confirm.rs` and `server.rs` `ConfirmationAnswer` arm (T050, T055)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U172 | Every connected fake window receives the same `ConfirmationRequested` naming caller, operation and target | FR-014, EC-5 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U173 | The first `allow` answer performs the operation | FR-014 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U174 | After the first answer every window receives `ConfirmationWithdrawn` | FR-014, EC-5 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U175 | A second answer for the same id changes nothing | FR-014 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U176 | An answer for an unknown id changes nothing | FR-014 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U177 | A decline fails `refused_by_policy` "declined by the user" and changes nothing | FR-014, FR-013 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U178 | No answer by 60 s (paused clock) fails `needs_confirmation` and withdraws the prompt | FR-014 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U179 | An allow arriving just before 60 s still performs the operation | FR-014 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U180 | With no window connected the request fails `needs_confirmation` at once, with no broadcast | FR-014, EC-6 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U181 | Deleting the target while pending fails `not_found`, withdraws, changes nothing | EC-9 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U182 | Deleting the caller while pending fails `not_found`, withdraws, changes nothing | EC-9 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U183 | Stopping the caller while pending abandons the request, withdraws, changes nothing | EC-9 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U184 | Closing the agent's HTTP connection while pending abandons the request, withdraws, changes nothing | FR-014 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |
| U185 | A window handshaking while a prompt is pending receives it with the remaining time | FR-014, EC-5 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_confirmations.rs` |

### `crates/micold-daemon/src/mcp/tools.rs` destructive handlers (T051, T056)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U186 | `stop_session` on an `Idle` session succeeds, reports `idle`, raises no confirmation (assumption A-5) | FR-012a | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U187 | `interrupt_session` on a session that is not running fails `conflict` before any confirmation | FR-013 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U188 | An allowed `delete_worktree {stop_sessions: true}` stops its sessions and removes the worktree | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U189 | An allowed `delete_worktree` with the default `delete_branch` removes the branch | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U190 | An allowed `delete_worktree {delete_branch: false}` keeps the branch | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U191 | An allowed `delete_session` archives the record | FR-009 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U192 | After an allowed `delete_session`, the deleted session's credential answers `401` | FR-006, EC-8 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |
| U193 | Of two agents deleting one worktree, the second fails `not_found` | EC-4 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` |

### `crates/micold-daemon/src/framer.rs` `plain_tail` (T063, T068)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U194 | `plain_tail(n)` returns the last n lines of scrollback plus screen | FR-012 | example | PENDING | planned: `crates/micold-daemon/tests/scrollback_range.rs` |
| U195 | Lines carry no escape sequences and are right-trimmed | FR-012 | example | PENDING | planned: `crates/micold-daemon/tests/scrollback_range.rs` |
| U196 | `truncated` is true when older lines existed beyond n | FR-012, EC-16 | example | PENDING | planned: `crates/micold-daemon/tests/scrollback_range.rs` |
| U197 | `truncated` is false when the whole content fits in n | FR-012 | example | PENDING | planned: `crates/micold-daemon/tests/scrollback_range.rs` |

### `crates/micold-daemon/src/mcp/tools.rs` cross-session handlers (T064, T069)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U198 | `read_session_output` reads the primary terminal even when another shell instance is attached | FR-012 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |
| U199 | `read_session_output {lines: 5000}` on a long scrollback returns at most 2,000 lines | FR-012, EC-16 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |
| U200 | A two-line `send_session_input` reaches the PTY as one bracketed submission ending `\r` | US4-AS2, FR-016 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |
| U201 | A `SettingsSet` changing the option applies to the very next request of an already-running session | FR-016 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_cross_session.rs` |

### `crates/micold-daemon/src/mcp/tools.rs` audit line (T072, T073)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U202 | Every successful mutating call writes exactly one `info` line with caller, op, target and `outcome=ok` | FR-018, SC-010 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_audit_log.rs` |
| U203 | Every failed mutating call writes exactly one `info` line whose outcome is its category | FR-018, SC-010, FR-013 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_audit_log.rs` |
| U204 | No `prompt` or `text` value appears in the log at any level | FR-018, SC-010 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_audit_log.rs` |
| U205 | Every failure produced by the acceptance tests carries one of the six categories | SC-010, FR-013 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_audit_log.rs` |

### Timing and placement (T074, T075)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U206 | With 50 worktrees and 50 sessions, each read-only tool answers in under 1 s | SC-004 | example | PENDING | planned: `crates/micold-daemon/tests/mcp_read_latency.rs` |
| U147 | A sandboxed session's `whoami` answers from inside the container, its returned paths are the container's, and the host loopback does not answer on that port (`sandbox-real-runtime`) | FR-001, FR-007, SC-008, EC-12 | example | PENDING | planned: `crates/micold-daemon/tests/sandbox_real_mcp.rs` |

### `crates/micold-client/src/features/agent_confirm.rs` (T052, T057)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U207 | `ConfirmationRequested` adds a pending prompt and opens the dialog | FR-014 | example | PENDING | planned: `crates/micold-client/tests/features_agent_confirm.rs` |
| U208 | `ConfirmationWithdrawn` removes that prompt | FR-014, EC-5 | example | PENDING | planned: `crates/micold-client/tests/features_agent_confirm.rs` |
| U209 | `ConfirmationWithdrawn` for an unknown id changes nothing | FR-014 | example | PENDING | planned: `crates/micold-client/tests/features_agent_confirm.rs` |
| U210 | Allow sends exactly one `ConfirmationAnswer { allow: true }` and closes the prompt | FR-014 | example | PENDING | planned: `crates/micold-client/tests/features_agent_confirm.rs` |
| U211 | Deny sends exactly one `ConfirmationAnswer { allow: false }` and closes the prompt | FR-014 | example | PENDING | planned: `crates/micold-client/tests/features_agent_confirm.rs` |
| U212 | Several pending prompts are shown in arrival order | FR-014 | example | PENDING | planned: `crates/micold-client/tests/features_agent_confirm.rs` |
| U213 | The open dialog is a registered covered state of the geometry gates | FR-014 | example | PENDING | planned: `crates/micold-client/tests/support/covered_states.rs` + `layout_snapshot.rs` |

### `crates/micold-client/src/features/settings.rs` (T025, T027, T065, T070)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U214 | The Environment draft carries `tool_server_enabled` from `DaemonSettings` | FR-004 | example | PENDING | planned: `crates/micold-client/tests/features_settings.rs` |
| U215 | Toggling the row sends `SettingsSet { tool_server_enabled: Some(_) }` | FR-004 | example | PENDING | planned: `crates/micold-client/tests/features_settings.rs` |
| U216 | The Environment draft carries `cross_session_access` | FR-016 | example | PENDING | planned: `crates/micold-client/tests/features_settings.rs` |
| U217 | Choosing a value sends `SettingsSet { cross_session_access: Some(_) }` | FR-016 | example | PENDING | planned: `crates/micold-client/tests/features_settings.rs` |
| U218 | The Environment page with each new row is a registered covered state | FR-004, FR-016 | example | PENDING | planned: `crates/micold-client/tests/support/covered_states.rs` + `layout_snapshot.rs` |
| U219 | `create_worktree` through `POST /mcp` from a Default session's credential fails `refused_by_policy` naming Principle III, and nothing on disk or in the catalog changes | US3-AS6, FR-015a | example | PENDING | planned: `crates/micold-daemon/tests/mcp_create_worktree.rs` |

## Invariants and edge cases still to place

None. Every edge case EC-1…EC-18 has a numbered behavior above.

## Out of scope

- The user-guide half of US1-AS5 and all of FR-019: `crates/micold-core/tests/documentation_is_not_read.rs`
  forbids tests reading `docs/`, so documentation is checked by review in T022/T028/T042/T048/T059/T071/T077, not by a test.
- The real `claude` / `copilot` binaries accepting the flags, surviving a ~55 s confirmation wait,
  and their ready signals (R1–R3, R12): not hostable in the default suite; quickstart §B via T100–T104 per milestone, T076 as the final re-run.
- SC-001's "100 % of sessions" with a real CLI and SC-008 on macOS/Windows with real CLIs: quickstart §B
  on each platform; the automated proxies are A1, U33, U140 and CI's three-platform run.
- SC-009's *second local account*: no test can create another OS account. Proxies: U30–U33
  (owner-only file) and U18–U20 (no credential → identical `401`).
- Rendering of the confirmation dialog and settings rows (colour, weight): GUI-glue exception,
  quickstart §B4/§B6 with the `visual-pass` skill.
- Projects, Settings changes, force-kill, shell instances, claiming worktrees, MCP resources/prompts,
  remote access, external MCP clients: spec *Out of Scope* and the *Operations* "not offered" list.

## Assumptions (non-interactive run)

- A-1: US3-AS6's `delete_worktree` refusal is observable only once `delete_worktree` is listed
  (M5, T056); before that the call fails `invalid_input` as an unknown tool. A18 therefore turns
  green in M5; its create part is proved through `POST /mcp` in M3 by U219 (T032) and its rename
  part in M4.
- A-2: A self-target of `read_session_output` / `send_session_input` is `invalid_input` even at Off
  (U117): mcp-tools.md orders validation before policy.
- A-3: With `CLAUDE_CONFIG_DIR` set, `~/.claude.json` is not read (U51): contracts/binding.md §6
  says "when that variable is set … else `~/.claude.json`".
- A-4: `NonEmptyText` rejects only the empty string; whitespace-only text is accepted (FR-012a says
  "empty text").
- A-5: `stop_session` on an `Idle` target succeeds without a confirmation (U186): the contract lists
  "Idle → success, unchanged" before "other session → confirm".
- A-6: US3-AS2 is two acceptance behaviors (A13 stop, A14 interrupt); both trace to US3-AS2.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time, so this file is readable on
its own:

- Single test (integration): `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
  (assert on the `N passed` count; a filter matching nothing exits 0)
- Single test (in `src/`): `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`, matches CI)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Acceptance (real runtime): `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1`
  (daemon half: `mise run image && mise run test-sandbox`)
- Coverage: none (`null` in the profile)
- Mutation: none (`null` in the profile; `/speckit.tdd.verify` spot-checks by hand)
- Property: none (`null`; invariants are sampled at their boundaries)
