# Quickstart: validating the tool server

**Feature**: 034-daemon-mcp-server | Contracts: [binding.md](./contracts/binding.md),
[mcp-tools.md](./contracts/mcp-tools.md), [protocol-delta.md](./contracts/protocol-delta.md)

## Part A — automated (every milestone)

```bash
mise run test-core     # policy table, JSON-RPC codec, tool schemas, submission encoding, settings
mise run gate          # fmt, clippy, workspace tests, scripts/tests
```

| Milestone | Tests that must pass (names are the tasks' files) |
|---|---|
| M1 | `crates/micold-core/tests/mcp_jsonrpc.rs`, `mcp_policy.rs` (read rows), `mcp_binding_plan.rs`; `crates/micold-daemon/tests/mcp_endpoint.rs` (auth, 401 sameness, 405, bounds), `mcp_read_tools.rs` (US1 s2–s3, empty project, other project → not_found), `mcp_binding_spawn.rs` (US1 s4–s6: config files byte-identical, toggle off, unbound log line), `mcp_binding_file_mode.rs` (0600/0700; Windows DACL) |
| M2 | `mcp_create_worktree.rs` (US2 s1–s2, SC-005 10-way race), `mcp_create_session.rs` (US2 s3–s5, FR-017 bounds) |
| M3 | `mcp_policy.rs` (destructive rows), `mcp_lifecycle_tools.rs` (US3 s1–s6), `mcp_confirmations.rs` (first answer wins, 60 s, no window, target gone, late joiner), client `features_agent_confirm.rs` |
| M4 | `mcp_cross_session.rs` (US4 s1–s5, option change applies to the next request), `submission_encoding.rs` |
| M5 | `mcp_audit_log.rs` (SC-010, no prompt text at any level) |

Real-runtime (off by default): `mise run image && mise run test-sandbox` runs
`sandbox_real_mcp.rs` — a sandboxed session's binding answers `whoami` from inside the container.

## Part B — with a real AI CLI

Prerequisites: a build of this branch (`mise run run`), a throwaway git repository with two
worktrees `a`, `b`, and `claude` and `copilot` signed in. Record each step's result in
`specs/034-daemon-mcp-server/evidence/`.

### B1 — Claude Code binding (M1; closes R1, R2 and R5's probes)

1. Hash `~/.claude.json`, `~/.claude/settings.json` and the repo's `.mcp.json` (if any).
2. Start a Claude Code session in `b`. Ask: "List the tools you have from the micold server, then
   call whoami, list_worktrees and list_sessions."
3. **Expect**: no trust prompt for the server, no per-tool permission prompt; `whoami` names this
   session and worktree `b`; the lists match the sidebar; this session has `is_caller: true`.
4. Re-hash the files from step 1: identical (US1 s4, SC-002).
5. `ps -o args= -p <claude pid>` shows `--mcp-config <data_dir>/mcp/<uuid>.json` and no
   `--strict-mcp-config`; `ls -l` on that file shows `-rw-------`.
6. `curl -s -o /dev/null -w '%{http_code}' -X POST http://127.0.0.1:<port>/mcp -d '{}'` → `401`.
7. (after M3) Ask the agent to delete a throwaway worktree; answer **Allow** after ~55 s. **Expect**
   the tool result arrives (the per-server `timeout` lifted the first-byte timer, R1).

### B2 — Copilot binding (M1; closes R3's probe)

Start a Copilot session; ask it to call `whoami`. **Expect** the same as B1 step 3. Record the
server-entry shape that worked. After M3, repeat B1 step 7 from Copilot. If Copilot
rejects the server entry, record the error, switch Copilot to `Unsupported` in `provider.rs` with the
observed reason, and update the user guide's bound-CLI list (FR-005).

### B3 — Create and delegate (M2)

From the session in `b`: "Create a worktree on a new branch feat-x and start a Claude Code session in
it with the prompt 'print the branch name'." **Expect**: row `feat-x` appears in every open window
within 2 s (SC-003) without a refresh, a session under it receives the prompt, and the tool result
reports `prompt_delivered: true`. Repeat with `ai_cli` set to `copilot` and to `pi` (readiness
signals of R12: output settled; component `session_start`). From a **Default** session, the
`create_worktree` request is refused by policy.

### B4 — Confirmation dialog (M3; visual pass)

With two windows open, from the session in `b` ask to delete worktree `feat-x` with
`stop_sessions: true`. **Expect**: both windows show the confirmation naming the calling session,
"delete worktree" and `feat-x`; allowing it in one withdraws it from the other; the worktree goes.
Repeat and decline: the tool result is "refused by policy: declined by the user". Repeat with no
window open: "needs confirmation" at once. Run with the `visual-pass` skill; save screenshots under
`evidence/`.

### B5 — Cross-session (M4)

With the option at **Auto**: from S1, `read_session_output` on S2 returns S2's recent text;
`send_session_input` with a two-line prompt arrives in S2 as one submission. Switch to **Confirm
each send** without restarting: the next send prompts. Switch to **Off**: both refused by policy.

### B6 — Settings rows (M1, M4; visual pass)

Settings → Environment shows "Let AI sessions manage worktrees and sessions" (toggle) and "Let
agents read and type into other sessions" (Auto / Confirm each send / Off), built from the shared
settings row components, in light and dark themes.
