# Research: The session service exposes an MCP server to the AI sessions it runs

**Feature**: 034-daemon-mcp-server | **Date**: 2026-09-29 | **Plan**: [plan.md](./plan.md)

Each entry records the decision, why, and what was rejected. Evidence is a path in this workspace,
a CLI's own `--help` (local binaries: `claude` 2.1.285, `copilot` 1.0.88, `pi` from the mise node 24
install), or the vendor's documentation. Items marked **probe** are confirmed by a real-CLI check in
[quickstart.md](./quickstart.md) §B before the milestone that relies on them merges.

## R1 — How Claude Code is given the tool server at launch

**Decision**: pass `--mcp-config <file>` pointing at a per-session JSON file the service writes into
its own data directory, and **not** `--strict-mcp-config`. The file holds one server:

```json
{"mcpServers":{"micold":{"type":"http","url":"http://127.0.0.1:<port>/mcp",
  "headers":{"Authorization":"Bearer <credential>"},"timeout":120000}}}
```

**Rationale**:
- `claude --help`: `--mcp-config <configs...>` "Load MCP servers from JSON files or strings". It is
  launch-only input, the same kind as the `--settings` file the hook receiver already passes
  (`crates/micold-daemon/src/state.rs` `activity_launch_for`, `crates/micold-daemon/src/hooks.rs`
  `prepare_settings`). No user or project file is touched (FR-003).
- Without `--strict-mcp-config`, `--mcp-config` servers are **added** to the user's, project's and
  plugins' servers. `--strict-mcp-config` would ignore every other configuration, which FR-003
  forbids ("MUST NOT displace a tool server the user configured").
- `type: "http"` with `headers` is Claude Code's documented shape for a remote Streamable HTTP
  server (code.claude.com/docs/en/mcp).
- `timeout: 120000` (ms) lifts the per-request first-byte timer, which is otherwise the greatest of
  60 s and the tool timeout. A destructive request may wait the full 60 s of FR-014 before it
  answers, so the default would race it (see R10).

**Alternatives rejected**:
- *`--strict-mcp-config`*: displaces user servers (FR-003).
- *Writing `.mcp.json` in the worktree or `claude mcp add`*: writes project or user configuration
  (FR-003), and `.mcp.json` servers need per-project approval.
- *Putting `mcpServers` in the existing `--settings` file*: settings carry no `mcpServers` key;
  only `enabledMcpjsonServers`/`disabledMcpjsonServers`, which gate `.mcp.json` servers.
- *A stdio shim command (`micold-daemon mcp-stdio`)*: needs a second binary path resolvable inside
  the session's environment (host and container), a process per session, and still needs a
  channel to the service. HTTP is what the hook receiver already proved in both placements.

## R2 — Tool permission prompts in Claude Code

**Decision**: add `--allowedTools mcp__micold` to the launch arguments, pre-approving this server's
tools only. The user's own permission rules are untouched.

**Rationale**: without it every tool call opens Claude Code's own permission prompt, so an agent
cannot call `whoami` on its first turn "with zero configuration steps" (SC-001) and a destructive
request would be confirmed twice, once by the CLI and once by FR-014. The spec's guards are FR-014
(destructive operations), FR-015/FR-015a (self and Default refusals) and FR-016 (cross-session
option), enforced by the service where the agent cannot bypass them. `--allowedTools` is additive to
the user's rules and lives on the command line only. Recorded in the ledger as agent-resolved.

**Alternatives rejected**: *leave the CLI's prompt on* (double confirmation; SC-001 fails in
practice); *`permissions.allow` inside the `--settings` file* (equivalent, but couples the tool
server to the hook receiver's file, which exists only when the hook receiver bound).

**Probe**: that a `--mcp-config` server needs no "trust this server" approval in an interactive
session, and that the server-wide rule `mcp__micold` covers every tool (quickstart §B1).

## R3 — GitHub Copilot CLI

**Decision**: bind Copilot with `--additional-mcp-config @<file>` (same JSON shape, plus
`"tools":["*"]`) and `--allow-tool micold`.

**Rationale**: `copilot --help`: `--additional-mcp-config <json>` takes a JSON string or `@file`,
is repeatable, and "augments config from ~/.copilot/mcp-config.json for this session", i.e. additive
and launch-only, so FR-002 requires the binding ("MUST be bound if their CLI accepts a tool server
supplied at launch"). Remote HTTP endpoints are supported. `--allow-tool` pre-approves a server's
tools, the analogue of R2.

**Probe**: the exact server-entry shape (`tools` field, header support) and Copilot's request
timeout against a 60 s confirmation wait (quickstart §B2). If a probe fails, the Copilot binding
ships disabled with its reason logged and documented (FR-005), which is the spec's own fallback.

**Alternatives rejected**: *writing `~/.copilot/mcp-config.json` or `.github/mcp.json`* (FR-003).

## R4 — Pi

**Decision**: Pi is **not** bound. It starts exactly as today, the service logs "no tool server:
Pi has no MCP support", and the user guide says so (FR-005).

**Rationale**: Pi deliberately has no MCP support (the author's post "What I learned building an
opinionated and minimal coding agent", section *No MCP support*; the package README has no MCP
mention; `pi --help` has no MCP flag). FR-002 binds a CLI only "if their CLI accepts a tool server
supplied at launch", so Pi falls under FR-005.

**Alternative rejected**: *a TypeScript bridge extension loaded with `-e`* that re-declares every
tool and forwards to the endpoint. It would work (feature 029 already loads a component with `-e`),
but it is a second implementation of the tool catalog in a second language, goes beyond FR-002's
condition, and doubles the conformance surface. Listed as a follow-up in the ledger.

## R5 — The server implementation: hand-rolled Streamable HTTP, not `rmcp`

**Decision**: implement the small MCP subset directly on the service's existing hand-rolled,
bounded HTTP/1.1 handling: stateless Streamable HTTP, one `POST /mcp` per JSON-RPC message, a
plain `application/json` reply (no SSE, no `Mcp-Session-Id`). Methods: `initialize`,
`notifications/initialized` (answered `202`), `ping`, `tools/list`, `tools/call`. `GET` and
`DELETE` answer `405`, which the specification allows for a server with no standalone stream.

**Rationale**:
- The hook receiver already parses bounded HTTP over a `tokio::net::TcpStream`
  (`crates/micold-daemon/src/hooks.rs` `parse_head`, `respond`, `drain`, `MAX_HEAD`). Extracting that
  into a shared module gives the tool server the same bounds and the same "never log the body"
  rule without a new HTTP stack.
- The workspace has no HTTP framework (root `Cargo.toml`: no axum, hyper, tower). `rmcp` 3.5.0
  (Apache-2.0, official SDK, released 2026-09-28) brings one, ships roughly weekly with API churn in
  its 3.x line, and would serve five methods. The constitution asks for minimal dependencies
  justified against the principles; this one is not.
- Stateless JSON replies are valid Streamable HTTP and what `rmcp`'s own default stateless mode
  does for modern clients.

**Protocol revision**: `initialize` answers with the client's `protocolVersion` when it is one of
`2025-03-26`, `2025-06-18`, `2025-11-25`, `2026-07-28`, else the newest of those. Claude Code
negotiates across these (its v1 and v2 MCP runtimes). **Probe**: Claude Code accepts a plain JSON
reply to `tools/call` (quickstart §B1).

**Alternatives rejected**: *`rmcp` with its axum transport* (dependency weight and churn for five
methods); *SSE responses* (needed only for server-initiated messages, which this feature has none
of; progress during a confirmation wait is not required because R1 lifts the timeout).

## R6 — One listener or two

**Decision**: a **separate** loopback listener for the tool server (`127.0.0.1:0`, ephemeral port),
sharing the extracted HTTP module with the hook receiver.

**Rationale**: `contracts/hooks.md` (feature 010) and `hooks.rs`'s module doc promise the hook
receiver "exposes no capability beyond reporting one session's activity"; a leaked hook token can
lie about one activity signal and nothing more. Routing `/mcp` through the same listener with the
same token would turn a hook token into a management credential. Separate credentials on a separate
listener keep that promise literally true. The body bound also differs (hook payloads up to 4 MiB,
MCP requests 1 MiB).

**Alternative rejected**: *one listener, two routes, two token maps*: saves a port but blurs the
hook contract for no user benefit.

## R7 — Credentials and FR-007 on each platform

**Decision**:
- **Credential**: a fresh `Uuid::new_v4()` in simple form per session (122 random bits, the same
  generator the hook receiver uses), held only in service memory in a `credential → SessionId`
  map. Idempotent for a session's lifetime (so a crash-respawn reuses it), dropped when the
  session is deleted, and gone when the service exits (FR-006, *Service restart* edge case).
- **The config file** holds the literal credential and is written owner-only: on Unix in a
  directory created `0700` and a file created `0600` (the patterns of
  `crates/micold-core/src/endpoint.rs` `DirBuilder…mode(0o700)` and
  `crates/micold-core/src/protocol/auth.rs` `.mode(0o600)`); on Windows with a protected DACL
  granting only the current user's SID (`D:P(A;;GA;;;<sid>)`, the SDDL
  `crates/micold-daemon/src/singleton.rs` already builds for the pipe). The directory is
  `<data_dir>/mcp/`, beside `hooks/`.
- **Network**: bound to `127.0.0.1` only, so no other host can connect (FR-007, first sentence).

**What "no other local user can reach it" means here**: a loopback TCP port can be *connected to*
by any local account on every platform; AI CLIs speak HTTP to a URL and accept no Unix socket or
named pipe for a remote MCP server, so a listener only the user can open does not exist for this
transport. The guarantee FR-007/SC-009 need is therefore met as: another account cannot read any
credential (owner-only files), and without one every request is refused with `401` and no data
(FR-006, SC-006). The spec's FR-007 and SC-009 are sharpened to say exactly that (decision D10 in
the ledger; the requirement's intent — another account cannot use or observe the tool server — is
unchanged).

**Alternatives rejected**:
- *Credential in an environment variable with `${VAR}` expansion in the config*: Claude Code
  supports it, Copilot's support is unverified, and the variable is inherited by every process the
  agent runs anyway. The file is owner-only, which is the property FR-007 asks for.
- *Unix socket / named pipe*: not an MCP transport the CLIs accept for a server they connect to.

**Follow-up (not this feature)**: the hook receiver's `--settings` file is written with the default
umask (`hooks.rs` `prepare_settings`), so its token file is not owner-only. Recorded under
*Follow-ups not done* in the ledger.

## R8 — The sandbox placement (feature 027)

**Decision**: no change for the sandbox. The service runs **inside** the container and spawns its
sessions there (`crates/micold-daemon/src/server.rs` `tcp_listen_addr`/`serve_tcp`), so its
`127.0.0.1` listener is on the container's own loopback, which is where the CLI runs. The port is
not published (`crates/micold-core/src/sandbox/argv.rs` publishes only the control port), so
nothing outside the container can reach it (FR-007). The config file lives under the service's data
directory, which is inside the container and readable by the session's CLI. Worktree paths the tool
server returns are the catalog's paths as the service holds them; on Linux and macOS projects are
mounted at the same absolute path (`sandbox/pathmap.rs`, "identity"), and on Windows the service
holds the container-side paths, which is what an agent inside the container sees (*Sandboxed
placement* edge case).

**Rationale**: the hook receiver works in the container by the same argument today. With outbound
network disabled the container's loopback interface still exists.

**Alternative rejected**: *publishing the tool server's port to the host*: nothing outside the
container needs it, and a published port widens FR-007's surface.

**Probe**: covered by the existing real-runtime suite pattern (`crates/micold-daemon/tests/
sandbox_real_*.rs`, feature `sandbox-real-runtime`), extended with one test that a sandboxed
session's binding answers `whoami`.

## R9 — Reusing the sidebar's code paths (FR-009)

**Decision**: extract the bodies of the `WorktreeCreate`, `WorktreeDelete` and `WorktreeRename`
arms of `route()` (`crates/micold-daemon/src/server.rs`) into a new
`crates/micold-daemon/src/ops.rs` with functions that return a typed outcome instead of replying to
a `ClientId`. `route()` keeps its behaviour by wrapping them; the tool server calls the same
functions. Session operations already exist as `DaemonState` methods usable without a client
(`create_session`, `begin_start`, `remove_session`, `delete_session`, `live_session`,
`worktree_live_sessions`), and are called directly. `spawn_session_start` is a free function in
`server.rs`, not a method; it moves to `ops.rs` with the worktree bodies so the tool handlers reach
it.

**Rationale**: FR-009 requires the same validation, naming, provenance record
(`record_worktree_provenance`, feature 029) and safeguards as the user action. Duplicating the
inline bodies would drift; the existing tests (`mutation_semantics.rs`, `mutation_atomicity.rs`,
`worktree_provenance_rpc.rs`) pin the behaviour the extraction must keep.

**Naming**: `create_worktree`'s optional worktree name is the directory name; when omitted it is
`naming::dir_name_from_branch(branch)`, the rule the dialog uses for an existing branch. Mode maps to
`CreateMode::{NewBranch, ReuseLocal, TrackRemote{remote}}`; `Overwrite` is not offered (it discards
a branch, and FR-014 lists no create as destructive). The pre-flight (`worktree::preflight`) runs
first and a mode incompatible with the situation (`CreateMode::is_compatible_with`) fails as a
conflict naming the `BranchSituation`, as the dialog would (US2 scenario 2).

**Progress**: the create arm streams `OperationProgress` to the requesting client through
`state.frame_sender(id)`. `ops::create_worktree` takes a progress sink (`Option<…>`); the tool server
passes none.

**Invalid names**: the dialog's messages come from `naming::derive` / `NamingError`
(`crates/micold-core/src/naming.rs`) and from git's own ref-format check in `worktree::preflight`;
`create_worktree` reports the same text as `invalid_input`.

**Session stop and interrupt have no sidebar action** (the client never sends `SessionStop`,
`SessionKill` or `SessionInterrupt`; only `SessionDelete`), so FR-009 compares them with the protocol
operation (spec clarification, plan round). The protocol's `SessionStop` arm only removes the live
entry and kills the tree, and leaves the catalog lifecycle `Running` with no broadcast (its
`TODO(T053)`). This feature therefore adds `DaemonState::stop_session(id)`: take the live entry,
kill its process tree off the lock, set the record's lifecycle to `Idle`, keep its credential (revoked
only on delete), and `broadcast_catalog`. The conversation stays on disk,
so the session is resumable. A SIGTERM-then-kill sequence was rejected: the existing stop, delete and
worktree-delete paths all kill the tree (Windows job objects, feature 030, have no SIGTERM), and a
second stop semantics would diverge from them. The protocol's `SessionStop` arm is pointed at the new
method, which fixes its missing broadcast for any future client too.

**No record for a missing CLI**: `DaemonState::create_session` writes the catalog record before any
spawn, so `create_session` first checks availability (`provider.is_available` against the
env-include-resolved `PATH`, as `AiCliAvailabilityRequest` does) and fails with `service_error`
naming the CLI before a record exists (US2 scenario 5).

**Alternatives rejected**: *duplicate the route() bodies in the tool handlers* (drift from the
sidebar's validation, which FR-009 forbids); *have the tool server act as a fake client over the
protocol* (a second codec and a synthetic `ClientId` for every call, and attachment exclusivity
per project would bump the user's window).

## R10 — Confirmations (FR-014, FR-016 Confirm each send)

**Decision**: the service holds a registry of pending confirmations and pushes them to every
connected window with two new daemon messages and one client message (protocol 16 → 17 in M5, see
[contracts/protocol-delta.md](./contracts/protocol-delta.md)). The tool call waits on a oneshot
channel for at most 60 s. The first answer removes the entry and broadcasts a withdrawal. With no
client connected the request fails at once with "needs confirmation". Deleting the target or the
calling session, or stopping the caller, resolves the entry with "not found" and withdraws the
prompt. A window that connects while a prompt is pending receives it after its handshake.

**Abandoned requests**: if the agent's HTTP connection closes while its request waits (the CLI
timed out or was stopped), the handler notices the closed socket, resolves the entry as abandoned,
broadcasts `ConfirmationWithdrawn` and performs nothing, so the user can never allow an operation
whose result nobody receives (FR-013).

**Rationale**: `DaemonState::broadcast` already reaches every client; a full `CatalogChanged`
cannot carry a question and its answer. A oneshot per request keeps the wait off the state lock.
The 60 s wait fits inside the 120 s per-server timeout of R1.

**Alternatives rejected**: *an MCP elicitation request to the agent's CLI* (asks the agent's user in
the agent's own terminal, not "in an application window" as FR-014 requires, and depends on CLI
support); *a desktop notification* (not an answer channel).

## R11 — Reading another session's output (FR-012)

**Decision**: a new `Framer` helper returns the last N lines of the **primary** process's terminal
(scrollback plus screen) as plain text: each `WireLine.text` right-trimmed, escape sequences never
present because the grid holds cells, not bytes. N defaults to 200, is clamped to 2,000, and below 1
is invalid input. `truncated` is true when older lines existed.

**Alternative rejected**: *keeping a separate raw-byte ring per session and stripping escapes*: a
second buffer that duplicates what the grid already holds, and escape stripping never renders
cursor movement correctly.

**Rationale**: `Framer::scrollback_range` (`crates/micold-daemon/src/framer.rs`) and
`oldest_available`/`newest` already address lines by stable `LineId`; the `ScrollbackRequest` arm of
`route()` shows the lock pattern. The primary process is read even when another shell instance is
attached, because FR-012 names the session's primary terminal.

## R12 — Typing into a session (FR-016, FR-017)

**Decision**: a pure function in `micold-core` encodes a submission: the text wrapped in bracketed
paste (`ESC[200~ … ESC[201~`) when the target terminal has bracketed-paste mode on, followed by a
carriage return. The service writes it straight to the primary PTY (`PtySession::write_input`), as
`SessionInterrupt` does with `0x03`, not through the client input log, whose serials belong to the
client stamper (`state.rs` `session_input`).

**Alternative rejected**: *through `session_input` with a synthetic serial*: it would corrupt the
client stamper's monotonic sequence (G2, protocol.md §7).

**Rationale**: AI CLIs enable bracketed paste; without it a multi-line prompt submits at its first
newline. The mode is read from the target's `Term` at write time.

**Initial prompt (FR-017)**: a fresh session never reads `AwaitingInput` before its first prompt
(`activity.rs`: only `Stop`/`Notification` lead there, and `SessionStart` is ignored by
`hooks.rs`), so the prompt waits on a separate **ready-for-input** signal, chosen per CLI behind the
provider seam (`AiCliProvider::input_readiness() -> InputReadiness`):
- `InputReadiness::HookSessionStart` (Claude Code): the hook receiver's `SessionStart` post, which
  it already receives and ignores, now also marks the session ready. The activity FSM is unchanged.
  When the hook receiver is not running (bind failure), Claude falls back to the output-settled
  rule. If the §B3 probe shows bytes written right at `SessionStart` are lost, readiness becomes
  "`SessionStart` and then output settled", which the same two signals already provide.
- `InputReadiness::ExtensionEvent("session_start")` (Pi): the activity component adds
  `session_start` to the events it reports; the tail marks the session ready. With the component
  declined (FR-012e), Pi falls back to the output-settled rule.
- `InputReadiness::OutputSettled` (Copilot, which writes nothing before the first prompt —
  026 research: its `events.jsonl` appears on the first user message): the primary terminal has
  produced output and then none for 1.5 s. This is terminal evidence used only for readiness,
  never for the activity badge, so `activity.rs`'s rule that activity never becomes `AwaitingInput`
  from terminal evidence still holds.

The tool call then writes the submission. The 60 s bound counts from the `create_session` request
(so with R1's 120 s per-server timeout the call always answers in time); on timeout or a failed start
it returns `prompt_delivered: false` and a later signal delivers nothing. **Probe**: quickstart
§B3 per CLI.

**Alternatives rejected**: *a prompt argument at launch* (`claude "<prompt>"`, `copilot -i`): the
spec's Assumption says typed input, and argv is readable by other local accounts on Linux
(`/proc/<pid>/cmdline`), which would expose prompt text; *writing immediately after spawn* (bytes can
be discarded or garbled while the TUI initialises); *`AwaitingInput`* (never fires first, above).

## R13 — Settings (FR-004, FR-016)

**Decision**: two new application-wide settings on the service side, following the
`pi_activity_component` template (`crates/micold-core/src/settings.rs`, `catalog.rs`,
`DaemonSettings`/`SettingsSet` in `protocol/messages.rs`, the Environment page
`crates/micold-client/src/ui/settings/environment.rs`):
- `tool_server_enabled: bool`, default `true` (FR-004), read when a session is spawned.
- `cross_session_access: CrossSessionAccess { Auto, ConfirmEachSend, Off }`, default `Auto`
  (FR-016), read on every request.

**Alternative rejected**: *one tri-state setting for both* (FR-004 and FR-016 are separate options by
the spec); *per-project settings* (the spec defines application-wide Settings options).

Each lands with the milestone that first uses it, so each milestone's protocol bump carries only its
own wire delta (M2: 15 → 16; M5: confirmation messages, 16 → 17; M6: 17 → 18).

## R14 — Name collision with a user-configured server

**Decision**: the server is named `micold`. Before binding, the provider checks, read-only, whether
the user's own configuration already defines a server of that name (Claude: `~/.claude.json`
top-level and per-project `mcpServers`, and the worktree's `.mcp.json`; Copilot:
`~/.copilot/mcp-config.json`). If it does, the session starts without a binding and the log names the
collision (FR-003 "MUST NOT displace", FR-005).

**Rationale**: Claude Code's precedence between `--mcp-config` and other scopes is undocumented and a
name maps to one definition. Skipping is the only choice that provably neither displaces nor is
displaced. Reading a file is not modifying it (FR-003 forbids create/modify/delete).

**Alternative rejected**: *a random per-session server name*: tool names become unstable
(`mcp__micold_3f2a__whoami`), which breaks R2's allow rule and the agent's recall across sessions.
