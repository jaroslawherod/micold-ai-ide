# Implementation Plan: Structured Session Permission Prompts — approve, deny, plan approval and questions

**Branch**: `claude/project-thread-tt60gq` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/942-structured-permission-prompts/spec.md`

## Summary

A structured session's CLI is launched with `--permission-prompt-tool
mcp__micold__permission_prompt`, a tool the daemon serves on the session service's existing tool
server (feature 034) behind the session's own credential. Each call becomes a pending request in a
`PromptRegistry` kept inside the daemon state's lock, as the agent confirmations of feature 034 are:
it is broadcast to every window, replayed to a window that connects, and holds the call until the
first decision, which the daemon turns into the CLI's allow/deny answer. Requests are withdrawn
(answered deny) when the turn ends, the process ends, or the session stops or is removed. A pending
request drives the session's activity through the same `Notification` path as a terminal session's
permission stop. Each structured session has a persisted permission mode, passed at launch with
`--permission-mode` and changed while running by a stream-json `set_permission_mode` control
request; the CLI's own mode reports are followed. All parsing, classification, answer building,
always-allow rules and validation live in a render-free core module, checked against fixtures
recorded from the real CLI. The window shows requests under their tool card in #939's transcript,
plus a plan view and a question view, and puts the mode control and the waiting state beside
#941's composer. The daemon side lands first behind a switch that stays off until the window can
answer. Details: [research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain)

**Primary Dependencies**: existing only — `serde`/`serde_json` (call and answer shapes, wire),
`tokio` (`oneshot` waits, the tool server's tasks), `tracing`, `iced` 0.14 and the shared
`ui/material` library. No new crate.

**Storage**: `permission_mode` in the existing catalog file (`StoredSession`, additive defaulted
field, no `schema_version` bump). Pending requests and always-allow rules in daemon memory only
(spec *Assumptions*).

**Testing**: `cargo test` — core unit tests and fixture tests
(`crates/micold-core/tests/structured_permission.rs` over
`tests/fixtures/permission-prompt/*.json`); daemon integration tests against #938's stand-in CLI,
extended to call the prompt tool (R13); client reducer tests; one geometry gate; quickstart Part B
visual pass for draw code.

**Target Platform**: Linux, macOS, Windows (daemon + client)

**Project Type**: desktop app — daemon + iced client over the shared render-free core

**Performance Goals**: a withdrawal reaches every client within 1 s of the turn's end, the process
end or the stop (SC-003); no request waits on anything but the user (FR-003).

**Constraints**: the CLI's tool-call timeout must not end a pending request (R3); a request frame
stays far below `MAX_FRAME_LENGTH` (input preview 64 KiB, R10); terminal sessions unchanged
(FR-018).

**Scale/Scope**: a handful of structured sessions, each with at most a few requests pending at
once (parallel tool calls).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
unchanged.

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every decision — call parsing, classification, choice validation, answer JSON, always-allow rules, mode names and launch arguments, control-line encoding and parsing, client drafts, `can_submit`, placement — is in `micold-core::structured::{permission, control}` or the render-free `features/permission_prompts.rs`, driven by failing tests first, with recorded fixtures as the oracle (FR-020). Daemon behaviour by stand-in integration tests (FR-019). Only draw code uses the GUI exception, validated by quickstart §B. |
| **II. Multi-Session** | PASS | Requests, rules and control waiters are keyed by `SessionId`; a request is raised only through its own session's credential and decided only with its session named (`OtherSession` refusal, FR-001, FR-012); the mode is persisted per session; a test runs two structured sessions with requests pending at once. |
| **III. Worktree Integration** | PASS | No new location or file operation; the CLI runs in the session's `SessionLocation::cwd` as before. |
| **IV. Local-First** | PASS | The prompt tool is the loopback tool server; nothing leaves the device; rules are never written to the user's Claude settings (FR-010). Fixtures are files. |
| **V. Rust + iced** | PASS | `RequestKind` carries only the fields its kind has (a plan approval without a plan cannot be built); `PromptChoice` is validated against the kind before any answer; a request's terminal state is the removal of its entry, so it cannot be decided twice. iced only. |
| **VI. Cross-Platform** | PASS | Loopback TCP and stdin control lines on every platform; no OS branch in core; the stand-in's prompt client is `std::net`. |
| **VII. Documentation** | PASS | `docs/user-guide/worktrees-and-sessions.md` (structured section: prompts, Always allow and its scope, plan approval, questions, modes, Bypass's risk; "permission prompts" removed from the not-yet list) in M8, the milestone that makes the feature user-visible (FR-021). |
| **VIII. Reusable UI** | PASS | New shared `ChoiceList` and `PromptCard` in `ui/material/` with builder APIs, showcase entries and theming; the rest reuses `Button`, `TextField`, `Select`, `Dialog` and #939's `DisclosureCard`. |

No violations; Complexity Tracking is empty.

## Integration points with work not on `main`

| ID | Needs | From | Assumed shape (named where the spec of that work does not exist) |
|---|---|---|---|
| I-1 | `ui::transcript::view` and its per-entry draw fns; `EntryKey::Tool(tool_use_id)`; the card-draw seam | #939 M5–M6 (core model on `main`; spec `specs/939-structured-transcript-view`) | The request block is drawn by `ui/transcript/permission.rs` directly under the card whose key is `EntryKey::Tool(id)`, through the `entries.rs` tool-card fn taking an optional trailing element; requests whose card is not held are drawn after the last entry. |
| I-2 | Composer area, idle/busy indicator, interrupt | #941 (no spec on `main` or any origin branch on 2026-10-21) | A per-session composer row below the transcript with a status slot; this feature puts the mode `Select` in it and replaces the busy text with *Waiting for you* while `waiting(session)`; the deny message and plan note are typed in the request block itself, not the composer, so they do not depend on #941. |
| I-3 | Control requests on stdin and their correlated answers | #941's interrupt (P-8: `subtype: interrupt`) | `structured::control` and `StructuredSession`'s `ControlWaiters` are built here (M5). If #941 lands first with its own, M5 uses it. |
| I-4 | Interrupt withdraws requests | #941 | Covered without #941: an interrupt ends the turn with a `result` (P-8), and a `Result` withdraws (R4). |
| I-5 | Structured sessions accept `SessionInterrupt` | #941 | None needed by this feature. |

M7 starts only when I-1 is on `main`; M8 only when I-2 is.

## Project Structure

### Documentation (this feature)

```text
specs/942-structured-permission-prompts/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── claude-permission.md   # launch flags, the prompt tool, control lines
│   └── wire.md                # client ↔ daemon additions
└── tasks.md                   # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── session.rs                       # + PermissionMode; Session.permission_mode
├── store.rs                         # + StoredSession.permission_mode (serde default, skip when Default)
├── mcp/binding.rs                   # + plan(…, timeout_ms): PROMPT_TOOL_TIMEOUT_MS for structured (R3)
├── mcp/jsonrpc.rs                   # route(…, ListScope): tools/list per caller kind
├── mcp/tools.rs                     # + PROMPT_TOOL (listed for structured callers only)
├── protocol/messages.rs             # + PermissionRequest, PromptId, PromptChoice, QuestionAnswer,
│                                    #   Resolution, WithdrawReason; Permission{Requested,Resolved,
│                                    #   Decision,Input,InputText}, SetPermissionMode;
│                                    #   SessionSummary/SessionCreate.permission_mode
├── protocol/version.rs              # PROTOCOL_VERSION +1 in M2, M4 and M5
└── structured/
    ├── event.rs, parse.rs           # + Init.permission_mode, Status, ControlResponse
    ├── control.rs                   # NEW: encode_set_mode, request ids
    ├── transcript/build.rs          # absorb Status, ControlResponse (#939's module)
    ├── log_line.rs                  # line text for the two new events (while #938's log exists)
    └── permission/                  # NEW, render-free
        ├── mod.rs                   # launch_args(mode, prompt_tool)
        ├── request.rs               # PromptCall, parse_call, Malformed, RequestKind, classify, Question
        ├── decision.rs              # validate, answer JSON, DecisionRefusal, preview cut
        └── rules.rs                 # AllowRule
crates/micold-core/tests/
├── structured_permission.rs         # NEW: fixture-driven (FR-020, SC-004)
├── structured_parse.rs              # + prompt-*.jsonl lines
└── fixtures/permission-prompt/      # NEW: recorded cases + README (R12)
    fixtures/stream-json/prompt-*.jsonl  # NEW: the same runs' stdout

crates/micold-daemon/src/
├── permission.rs                    # NEW: PromptRegistry, serve_prompt (the tools/call handler),
│                                    #   decide, withdraw, activity calls (R4, R8, R9)
├── mcp/server.rs                    # route by caller kind; permission_prompt → permission::serve_prompt
├── structured.rs                    # ControlWaiters; write_control; reader resolves waiters;
│                                    #   launch flags; Result/TurnEnded → withdraw hook
├── state.rs                         # registry field; replay on handshake; withdraw on stop/remove/
│                                    #   process end; PermissionDecision, SetPermissionMode; follow
│                                    #   reported mode; set_permission_prompts switch (R14)
├── catalog.rs                       # set_permission_mode; create with mode
└── server.rs                        # dispatch the new ClientMsgs
crates/micold-daemon/tests/
├── support/structured.rs            # stand-in: `prompt`, `prompt-async`, control answers (R13)
├── structured_session.rs            # FR-006a argv list: --permission-prompt-tool and
│                                    #   --allow-dangerously-skip-permissions now expected (superseded)
├── structured_permission.rs         # NEW: US1, US5, US6, edge cases
└── structured_permission_mode.rs    # NEW: US2, US3 mode parts

crates/micold-client/src/
├── features/permission_prompts.rs   # NEW render-free state (data-model *Client state*)
├── features/session.rs              # mode in the structured create flow; Bypass confirmation step
├── shell/daemon_sync.rs             # Permission* messages ↔ features; SetPermissionMode replies
├── app.rs                           # State.permission_prompts
├── ui/transcript/permission.rs      # NEW draw code: request block, plan view, question view (M7)
├── ui/transcript/entries.rs         # tool card takes the request block (I-1)
├── ui/<composer module of #941>     # mode Select, Waiting for you, Bypass label (M8, I-2)
├── ui/confirm_bypass.rs             # NEW: the Bypass confirmation dialog (like confirm_delete.rs)
├── ui/material/choice_list.rs       # NEW shared component
├── ui/material/prompt_card.rs       # NEW shared component
├── ui/material/mod.rs               # register both
└── showcase/                        # entries for ChoiceList, PromptCard
crates/micold-client/tests/
├── features_permission_prompts.rs   # NEW reducer tests
├── structured_start_menu.rs         # + mode on create, Bypass confirmation step
├── layout_snapshot.rs               # + gates/prompt_card_fits.rs
└── gates/prompt_card_fits.rs        # NEW geometry gate
scripts/capture-permission-prompt.sh # NEW (R12), sources capture-stream-json.sh's scrubber
scripts/capture/prompt-stub.py       # NEW stub MCP server the capture runs
scripts/tests/capture-permission-prompt.test.sh  # NEW: the scrubber on a prompt fixture
docs/user-guide/worktrees-and-sessions.md        # FR-021 (M8)
```

**Structure Decision**: the existing three-crate split. Every decision is in
`micold-core::structured::{permission, control}` or a render-free client module; the daemon holds
and routes, the client draws.

## Requirement Coverage

Test layers: **core** = `micold-core` unit/fixture tests; **daemon** = `micold-daemon` integration
tests against the stand-in CLI; **client** = `micold-client` reducer tests; **gate** = geometry gate;
**§B** = [quickstart.md](quickstart.md) Part B.

| FR | Design | Test layer |
|---|---|---|
| FR-001 | R1 (credential = caller; prompt tool listed and callable for structured callers only); R14 launch flag | daemon (terminal caller: not listed, call `unknown_tool`; session B's credential cannot decide A's request; argv has the flag); core (`tools/list` per scope) |
| FR-002 | R2; data-model `PromptCall`, `RequestKind` | core (classify each fixture; shape misfits → `Tool`); daemon (request carries tool, input, id, kind) |
| FR-003 | R3 (binding timeout), R4 (no deadline) | daemon (no answer while pending across 2 s and a client reconnect); §B 9 (> 10 min with the real CLI) |
| FR-004 | R2 `Malformed` | core (each reason); daemon (deny at once, no `PermissionRequested`, log has tool name, no input) |
| FR-005 | R4 (inside the state lock), R10; wire `PermissionRequested`/`Resolved` | daemon (two clients get it; a client connecting while pending gets it once; race test: connect during open) |
| FR-006 | R11; I-1; `ui/transcript/permission.rs` | client (placement under card / at end; preview cut flag); §B 1, 4 |
| FR-007 | R11 plan view | client (plan text held whole; actions per kind); §B 6 |
| FR-008 | R5 `validate`; client `can_submit` | core (`Incomplete` cases); client (submit disabled until every question answered; Other); §B 7 |
| FR-009 | R8; I-2 | daemon (activity `AwaitingInput` while pending, `Working` after a decision; needs-permission notification counted once with the CLI's own `Notification` hook also firing); client (`waiting`); §B 1 |
| FR-010 | R5, R7, R9; data-model `PromptChoice` | core (answer JSON per choice equals fixture; rules per tool); daemon (each choice reaches the stand-in; first of two decisions wins; always-allow: second identical call answered without a request; plan approve sets mode before the answer) |
| FR-011 | R4 withdrawal | daemon (result, `TurnEndedWithoutResult`, process kill, stop, remove, hang-up: withdrawn to every client ≤ 1 s, stand-in gets deny when still connected; nothing pending after a daemon restart) |
| FR-012 | data-model `DecisionRefusal` | core (refusal sentences); daemon (answered, withdrawn, unknown id, other session, wrong kind: `OperationError`, stand-in unchanged) |
| FR-013 | R6; data-model `PermissionMode` | core (legacy record → `Default`; `launch_args(Default)` adds no `--permission-mode`); daemon (argv) |
| FR-014 | R11 creation flow + `ui/confirm_bypass.rs` | client (`structured_start_menu`: mode on create; Bypass needs the confirmation step); §B 5, 6 |
| FR-015 | `store.rs`, `SessionSummary.permission_mode`, launch | core (store round trip, summary serde); daemon (mode at launch, after restart, after daemon restart and resume) |
| FR-016 | R6 *Following Claude*, R7 | core (`Status`/`Init` parse); daemon (stand-in writes a `status` line → summary and record change) |
| FR-017 | R6 *Change while running*; data-model `ControlRequest` | core (`encode_set_mode`); daemon (running: control line on stdin, `success` → persisted and broadcast, `error` → unchanged with the CLI's text, no answer → unchanged after 10 s; pending request untouched; not running: persisted, used at next launch) |
| FR-018 | R3 (binding timeout only for structured), R14 | daemon (terminal argv and binding file unchanged; terminal `SessionCreate` with a mode refused); gate (every existing terminal test green) |
| FR-019 | R13 | daemon (both new suites run with the stand-in only) |
| FR-020 | R12 | core `structured_permission` over the recorded fixtures |
| FR-021 | user guide (M8) | M8 diff review (each named topic present; "permission prompts" gone from the not-yet list) |
| SC-001 | R4 | daemon (stand-in gets no reply until decided, then exactly the decision; turn ends with its result) |
| SC-002 | R4 first-wins | daemon (two clients decide concurrently, 50 iterations: one decision reaches the stand-in, both see one resolution) |
| SC-003 | R4 | daemon (withdrawal within 1 s per cause; late decision refused) |
| SC-004 | R12 | core fixture test |
| SC-005 | R6 | daemon (four modes × launch, restart, daemon restart; change then next request) |
| SC-006 | R11 | §B 1, 6, 7 (press counts recorded) |
| SC-007 | — | gate; the superseded #938 argv assertion is the one edited test (below) |

**SC-007 and the superseded assertion.** `crates/micold-daemon/tests/structured_session.rs`
asserts feature 938 FR-006a: no `--permission-mode`, `--permission-prompt-tool` or
`--dangerously-skip-permissions` in a structured argv. This feature's FR-001 replaces FR-006a's
refusal, so that list changes in M2 (and in M8, when the switch goes) to what this feature launches; it is the one existing
test edited, and it still asserts that a Default session carries no `--permission-mode`. Every
other existing test passes unchanged.

## Delivery

Eight milestones; each merges to `main` on its own. M1–M6 do not depend on #939's view or #941.
The two daemon steps of the first sketch (hold/decide/withdraw; mode) are each split in two to stay
near the 800-line milestone budget, along acceptance scenarios ([tasks.md](tasks.md) *Milestones*).

| M | Delivers | Depends on |
|---|---|---|
| M1 | Core: `PermissionMode`, `launch_args`, `permission::{request, decision, rules}`, `control`, the new events and parse, transcript absorbs; recorded fixtures, capture script | — |
| M2 | Daemon: prompt tool listed per caller, `PromptRegistry`, hold and decide, always allow, wire `Permission*` (+1), stand-in `prompt`, binding timeout, the switch (off) | M1 |
| M3 | Daemon: activity while pending; withdrawal on turn end, process end, stop, remove, hang-up | M2 |
| M4 | Daemon: mode persisted, in summaries and create, launched (+1) | M3 |
| M5 | Daemon: `SetPermissionMode` via control request, following reported modes, plan approval sets the mode (+1) | M4 |
| M6 | Client render-free: `features/permission_prompts.rs`, mode in the create flow's state, `daemon_sync` wiring; no draw code | M5 |
| M7 | Window, transcript side: `ChoiceList`, `PromptCard`, request block, plan view, question view, geometry gate | M6, I-1 |
| M8 | Window, composer side: mode `Select`, *Waiting for you*, Bypass confirmation and label, the switch turned on and removed, user guide, Part B | M7, I-2 |

## Complexity Tracking

None.
