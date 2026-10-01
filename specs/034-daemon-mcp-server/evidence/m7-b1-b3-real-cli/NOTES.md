# M7 quickstart B1-B3 against the real CLIs (Claude Code, Copilot, Pi)

Date 2026-10-01. Claude Code 2.1.287, GitHub Copilot CLI 1.0.90 (self-updated to 1.0.91 mid-run; the B2 session ran 1.0.90, the
later ones 1.0.91), Pi 0.85.1. Linux.

## How it was run

Xvfb :89 (1600x1400) + lavapipe, not a real display. Client and daemon: the pinned pair copied to a private dir, run side by side.
Private XDG_DATA_HOME / XDG_CONFIG_HOME / XDG_RUNTIME_DIR (/tmp/vp89) / XDG_STATE_HOME. Daemon environment: private
`CLAUDE_CONFIG_DIR` (copy of credentials, settings.json, `.claude.json` with the throwaway repo trusted and onboarding done),
private `COPILOT_HOME` (config.json/settings/permissions/skills, repo appended to `trustedFolders`), `ANTHROPIC_MODEL=haiku`
(honoured: Claude showed "Haiku 4.5"), all inherited CLAUDE*/ANTHROPIC* session variables unset. Everything was real: real
`claude`, `copilot`, `pi` from PATH, real agents answering, real model calls. Nothing stubbed. Sessions were created from the
sidebar (+ on the row, then the CLI menu); the prompts were typed with xdotool into each session's terminal. Pi runs with the
user's real HOME (its own config is not redirected).

Throwaway repo `proj2` (one commit, `main`). Worktrees `a` and `b` were created THROUGH the app's New worktree dialog (type
`feat`, name `a` / `b`), so the dialog named them dir `feat-a` / `feat-b`, branch `feat/a` / `feat/b`; the sidebar shows them as
"A" / "B" with a `feat` chip and the tools call them `feat-a` / `feat-b` (ref and display_name). Below, "a"/"b" mean those two.
(An earlier repo `proj` with worktrees made by `git worktree add` was discarded: the sidebar showed "No worktrees yet" for
worktrees git knew but the app had not created. Not investigated further; see Deviations.)

## Results

- B1.1 hashes: PASS (recorded, `b1-captures.txt`). `.mcp.json` absent before.
- B1.2-3 PASS. `b1-claude-terminal.png`, `b1-sidebar.png`. Claude Code started in `b` with no trust or onboarding question; the
  prompt got "Called micold 3 times" and no permission prompt. It listed 15 tools (create_session, create_worktree,
  delete_session, delete_worktree, get_session, interrupt_session, list_branches, list_sessions, list_worktrees,
  read_session_output, rename_worktree, send_session_input, start_session, stop_session, whoami). whoami: ai_cli claude_code,
  project proj2, session b705761c-..., worktree feat-b. list_worktrees: Default (0 sessions), feat-a (0), feat-b (1), as in the
  sidebar. list_sessions: one session, `is_caller: true`, worktree feat-b, lifecycle running.
- B1.4 PASS. `.claude.json` changed (a746df32... -> 3ff927c4...; Claude's own bookkeeping: lastGracefulShutdown, lastVersionBase
  and so on for the project), no `micold` entry anywhere in its `mcpServers` keys (the only `mcp` key is
  `claudeAiMcpEverConnected`); settings.json identical (9a546cf5...); no `.mcp.json` in the repo or any worktree.
- B1.5 PASS. `b1-captures.txt`: `claude --session-id ... --settings <data>/hooks/<id>.json --mcp-config <data>/mcp/<uuid>.json
  --allowedTools mcp__micold`; no `--strict-mcp-config`; file mode `-rw-------`. The entry: type http, 127.0.0.1 url, an
  Authorization header (redacted), timeout 120000.
- B1.6 PASS. POST /mcp -d '{}' with no token returned 401.
- B2 PASS. `b2-copilot-terminal.png`, `b2-captures.txt`. Copilot accepted the entry (no error): `copilot --session-id ... --no-remote
  --additional-mcp-config @<data>/mcp/<uuid>.json --allow-tool micold`; file mode -rw-------, entry has `tools: ["*"]`, type http.
  Asked to call whoami: "whoami (MCP: micold)" ran with no permission prompt; result {"ai_cli":"copilot", project proj2,
  session 2c01f9cc-..., worktree "feat-a"}.
- B3a PASS. `b3a-claude-results.png`, `b3a-sidebar-feat-x.png`, `b3a-feat-x-session.png`, `b3a-create-worktree-agent-args.txt`.
  create_worktree result (app_created true, ref feat-x) and create_session result `{"lifecycle":"running","prompt_delivered":
  true,...}`; row feat-x was in the sidebar when the answer was read (the agent took 15 s to answer, so the 2 s appearance was NOT
  timed from the screen; the daemon log shows the two calls 5.5 s apart); the session under it shows `print the branch name`
  once and "The current branch is feat/x". The agent itself passed branch `feat/x`, name `feat-x`.
- B3b-copilot PASS. `b3b-copilot-session.png`: prompt shown once, Copilot ran `git branch --show-current`, answered `feat/x`;
  tool result prompt_delivered true. `b3b-agent-results.png`.
- B3b-pi PASS. `b3b-pi-session.png`: prompt submitted once; Pi answered with the provider 400 "Third-party apps now draw from
  your extra usage" (expected; the prompt being submitted is the check); prompt_delivered true. `pi` was present.
- B3c-create PASS. `b3c-create-worktree.png`, `b3c-sidebar-before.png`: from a Default session, create_worktree
  {branch from-default, mode new_branch} returned the row (ref from-default); row "From default" appeared.
- B3c-rename PASS and B3c-delete PASS. `b3c-rename-delete-refused.png`, `b3c-sidebar-after.png`: both returned
  `refused_by_policy: a session running in the project root (Default) may not rename or delete worktrees (Constitution Principle
  III); ask from a session that runs in a worktree`; no dialog appeared; sidebar unchanged (before/after crops are identical).
- B3d PASS. `b3d-daemon-log-mcp-lines.txt`: 7 `micold::mcp` `tool call` lines (caller, op, target, outcome), one per changing
  call, outcomes ok / refused_by_policy; no prompt text (the log was grepped for the prompts: none). Read-only calls
  (whoami, list_*) are not logged.

## Deviations

- Worktrees `a`/`b` are dir `feat-a`/`feat-b` because the dialog needs a type + name; branch names `feat/a`, `feat/b`.
- A first attempt used worktrees made with `git worktree add`; the sidebar did not list them ("No worktrees yet" after Refresh
  and Open). Not run to ground; restarted on a fresh repo with dialog-made worktrees.
- B3b: my typed prompt went to the session `feat-x` (Claude) instead of the one in `b`, because the sidebar rows had shifted
  after the click. So the two create_session calls (copilot, pi) were made by the agent in session bf3032d8 (worktree feat-x), not
  by the one in `b`. The agent itself made both calls; the results are the same kind.
- In the copied settings.json I removed `permissions.defaultMode: "auto"` so a permission prompt would not be masked. The hash
  check is against that private copy.
- Copilot's startup shows "Failed to load 1 skill" (from the copied skills dir); unrelated.
- The 2 s sidebar timing (SC-003) was not measured from the screen.
- No curl calls stood in for an agent: every tool call above is the agent's own, except the 401 probe.

## Not run

- B1.7 / B2 permission-timeout (delete confirmation after ~55 s), B4, B5, B6: not in this task's scope.
- Real-GPU rendering; Copilot/Pi calling tools other than create_session; the daemon in a container.
