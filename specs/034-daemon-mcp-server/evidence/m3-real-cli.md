# M3 evidence: quickstart B3 against the real CLIs (T102)

Date: 2026-09-30. Claude Code 2.1.285, GitHub Copilot CLI 1.0.88 (1.0.89 after its self-update),
Pi Coding Agent 0.85.1, Linux.

## How it was run

A throwaway probe test (not committed) built a `DaemonState` over a git repository with worktree
`b`, sessions S1 (root) and S3 (`b`), a running hook receiver (as `server::run` binds one), the Pi
activity component on, and the real tool server. It then called the tools through `POST /mcp`
exactly as an agent would: `create_worktree {branch: feat-x}` from S3 and from S1, then
`create_session {worktree: feat-x, ai_cli, prompt: "print the branch name"}` for each CLI. The real
`claude`, `copilot` and `pi` from the user's `PATH` ran as the sessions' processes. After 35–45 s
the probe read each session's screen from its `Term`. A second probe sampled the output counter
every 100 ms from spawn and typed the prompt at a chosen moment, to see when each CLI takes input.

The probe's own environment had Claude Code's `CLAUDE_CODE_*` variables removed, since it ran
inside a Claude Code session and the application's service does not.

## Findings

1. **Claude Code posts no `SessionStart` over an HTTP hook.** With the receiver running, the
   `UserPromptSubmit`/`Stop` hooks arrived (activity went to `AwaitingInput` after a prompt typed
   by hand), but `SessionStart` never did in 40 s, so the planned `HookSessionStart` readiness
   timed out at 60 s with `prompt_delivered: false`. Output settles about 1 s after spawn and a
   prompt typed at 2.6 s is accepted and answered. Claude readiness is therefore the
   output-settled rule (research R12's own fallback; FR-017 "for a CLI that reports no such
   signal"). Changed test-first: `input_readiness.rs::claude_is_ready_once_its_output_has_settled`
   and `mcp_create_session.rs::with_the_hook_receiver_running_claude_is_ready_once_its_output_settles`
   (cycle 23). `hooks.rs` is back to ignoring `SessionStart`.
2. **Copilot's output-settled rule holds** when the folder is trusted: ready 6.5–8.3 s after the
   request, prompt shown and answered (`git branch --show-current` → `feat-x`).
3. **Pi's `session_start` event works**: ready 0.34–0.48 s after the request, well before Pi drew
   anything; the prompt appeared as the first input and was submitted. (Pi's reply was an HTTP 400
   from its model provider about third-party usage billing, unrelated to the session service.)
4. **A folder the CLI does not trust yet shows a trust question first.** In a repository neither
   CLI had trusted:
   - Claude Code shows "Is this a project you created or one you trust?" with **No, exit**
     preselected. Its output settles on that screen, so under the output-settled rule the
     prompt's Enter would answer it (and Claude would exit).
   - Copilot shows "Confirm folder trust" with **1. Yes** preselected. Its output settled on that
     screen, the prompt's Enter selected Yes, and the prompt text itself was lost
     (`prompt_delivered: true`, empty input box). The trust was for that session only; Copilot's
     `trustedFolders` was not changed.
   A worktree inherits trust from its project: with only the project root trusted (a private
   `CLAUDE_CONFIG_DIR` / `COPILOT_HOME` copy naming it), no question appeared in
   `.claude/worktrees/<name>` and every prompt below was delivered. Resolved (ledger D20, decided
   by the user): before waiting, the service reads the CLI's own trust record, read-only, and when
   the CLI would ask it types nothing and returns `prompt_delivered: false` with a `prompt_reason`
   (cycle 24).

## Results (project trusted by each CLI)

| Quickstart step | Result |
|---|---|
| `create_worktree feat-x` from the session in `b` | Pass: the row, `app_created: true`; a registered window saw it after 29 ms (SC-003: 2 s) |
| Claude Code session in `feat-x` with the prompt | Pass: `prompt_delivered: true` after 3.7 s; screen shows `❯ print the branch name` and "The current branch is feat-x." |
| Copilot session with the prompt | Pass: `prompt_delivered: true` after 8.3 s; screen shows the prompt and `feat-x` |
| Pi session with the prompt | Pass: `prompt_delivered: true` after 0.34 s; screen shows the prompt submitted |
| `create_worktree` from the Default session | Pass: `refused_by_policy`, naming Constitution Principle III |
| Audit lines | Pass: one `INFO micold::mcp` line per call with caller, op, target, outcome; the prompt text is not in the log |

## Deviations from quickstart §B3

- Driven by a probe test, not from an app window; the sessions, tool server and hook receiver are
  the service's own, the window is a registered fake client.
- The trusted runs used a private copy of each CLI's configuration naming the probe project as
  trusted, so the user's own `~/.claude.json` and `~/.copilot/config.json` were never written. The
  copies were deleted after the run.

## Audit lines (verbatim, trusted run)

```text
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000003 op=create_worktree target=feat-x outcome=ok
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000001 op=create_worktree target=feat-y outcome=refused_by_policy
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000003 op=create_session target=feat-x outcome=ok
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000003 op=create_session target=feat-x outcome=ok
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000003 op=create_session target=feat-x outcome=ok
```
