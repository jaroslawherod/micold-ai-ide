# Evidence index: quickstart Part B (§B1–§B6)

Each row is one step of [quickstart.md](../quickstart.md) Part B and the files that show its result.

The **final pass** (M7, T076, 2026-10-01) ran every step but one (§B2's repeat of §B1 step 7 from
Copilot) on the finished feature, in the real app on Xvfb, with the real `claude`, `copilot` and `pi`
and real model calls:

- [m7-b1-b3-real-cli/](m7-b1-b3-real-cli/NOTES.md): §B1 steps 1–6, §B2, §B3. 13 of 13 results passed.
- [m7-b4-b6-real-cli/](m7-b4-b6-real-cli/NOTES.md): §B1 step 7, §B4, §B5, §B6. 11 of 11 checks passed.

Each directory's `NOTES.md` says how the pass was run, what deviated from the quickstart and what
was not run. The **earlier** column names the evidence recorded when the step's milestone shipped.

## §B1: Claude Code binding

| Step | Final pass (M7) | Earlier |
|---|---|---|
| 1. Hash the user's Claude files | `m7-b1-b3-real-cli/b1-captures.txt` | M1: [m1-real-cli.md](m1-real-cli.md) |
| 2–3. The agent lists the tools and calls `whoami`, `list_worktrees`, `list_sessions` with no prompt | `m7-b1-b3-real-cli/b1-claude-terminal.png`, `b1-sidebar.png` | M1: [m1-real-cli.md](m1-real-cli.md) |
| 4. The hashes again: no `micold` entry, no `.mcp.json` | `m7-b1-b3-real-cli/b1-captures.txt`, NOTES.md *B1.4* | M1: [m1-real-cli.md](m1-real-cli.md) |
| 5. `--mcp-config`, no `--strict-mcp-config`, file mode `-rw-------` | `m7-b1-b3-real-cli/b1-captures.txt` | M1: [m1-real-cli.md](m1-real-cli.md) |
| 6. `POST /mcp` with no credential answers `401` | `m7-b1-b3-real-cli/b1-captures.txt` | M1: [m1-real-cli.md](m1-real-cli.md) |
| 7. Allow after about 55 s: the tool result arrives | `m7-b4-b6-real-cli/b4d-after-allow-agent-result.png`, `daemon-tool-calls.txt`, NOTES.md *B4 D* | M5: [m5-real-cli.md](m5-real-cli.md), case `claude-55s` |

## §B2: Copilot binding

| Step | Final pass (M7) | Earlier |
|---|---|---|
| Copilot accepts the server entry and calls `whoami` with no prompt | `m7-b1-b3-real-cli/b2-copilot-terminal.png`, `b2-captures.txt` | M1: [m1-real-cli.md](m1-real-cli.md) |
| §B1 step 7 from Copilot (Allow after about 55 s) | Not repeated in the final pass | M5: [m5-real-cli.md](m5-real-cli.md), case `copilot-55s` |

Copilot stays bound: no probe failed, so `provider.rs` and the user guide's list of bound CLIs are
unchanged (FR-005).

## §B3: create and delegate, readiness per CLI

| Step | Final pass (M7) | Earlier |
|---|---|---|
| `create_worktree` and `create_session` with a first prompt (Claude Code): the row appears, `prompt_delivered: true` | `m7-b1-b3-real-cli/b3a-claude-results.png`, `b3a-sidebar-feat-x.png`, `b3a-feat-x-session.png`, `b3a-create-worktree-agent-args.txt` | M3: [m3-real-cli.md](m3-real-cli.md) |
| The same with `ai_cli` set to `copilot` and to `pi` | `m7-b1-b3-real-cli/b3b-agent-results.png`, `b3b-copilot-session.png`, `b3b-pi-session.png`, `b3b-sidebar.png` | M3: [m3-real-cli.md](m3-real-cli.md) |
| From a Default session, `create_worktree` works (constitution 1.7.0) | `m7-b1-b3-real-cli/b3c-create-worktree.png`, `b3c-sidebar-before.png` | None: the rule changed in M6 |
| From a Default session, `rename_worktree` and `delete_worktree` are refused by policy | `m7-b1-b3-real-cli/b3c-rename-delete-refused.png`, `b3c-sidebar-after.png` | None |
| One audit line per changing call, with no prompt text | `m7-b1-b3-real-cli/b3d-daemon-log-mcp-lines.txt` | M3: [m3-real-cli.md](m3-real-cli.md) *Audit lines* |

The 2 s appearance of the new row (SC-003) was not timed from the screen in either pass; the
automated suite times it.

## §B4: confirmation dialog

| Step | Final pass (M7) | Earlier |
|---|---|---|
| Both windows show the confirmation with the calling session, the operation and the target | `m7-b4-b6-real-cli/b4a-dialog-window1-top-window2-bottom.png`, `b4a-both-windows.png` | M5: `m5-b4-confirm-dialog/a-dark-both-windows.png`, `a-light-window1.png` |
| Allow in one window withdraws it from the other; the worktree goes | `m7-b4-b6-real-cli/b4b-both-windows-after-allow.png` | M5: `m5-b4-confirm-dialog/b-window1-after-allow.png`, `b-window2-after-allow.png`, `result-B-allow.json` |
| Deny: "refused by policy: declined by the user" | `m7-b4-b6-real-cli/b4c-deny-agent-result.png` | M5: `m5-b4-confirm-dialog/c-window2-after-deny.png`, `result-C-deny.json` |
| No window open: "needs confirmation" at once | `m7-b4-b6-real-cli/b4e-curl-no-window.txt` (sent with `curl`, not by the agent) | M5: `m5-b4-confirm-dialog/result-F-no-window.json` |

Escape and a second dialog over the confirmation were checked only in M5
([m5-b4-confirm-dialog/NOTES.md](m5-b4-confirm-dialog/NOTES.md)).

## §B5: cross-session

| Step | Final pass (M7) | Earlier |
|---|---|---|
| Auto: `read_session_output` returns the other session's text | `m7-b4-b6-real-cli/b5a-read-s2-output-no-dialog.png` | M6: `m6-b5-b6-cross-session/result-B-read.json` |
| Auto: a two-line `send_session_input` arrives as one submission | `m7-b4-b6-real-cli/b5b-s2-one-submission-pong.png` | M6: `m6-b5-b6-cross-session/b-s2-terminal-auto.png`, `result-B-send.json` |
| Confirm each send, saved without a restart: the next send asks; a read does not | `m7-b4-b6-real-cli/b5c-setting-confirm-each-send.png`, `b5c-confirm-dialog-no-text.png`, `b5c-allow-s2-received-ok.png`, `b5c-deny-agent-result.png`, `b5c-s2-after-deny-no-nope.png`, `b5c-read-at-confirm-no-dialog.png` | M6: `m6-b5-b6-cross-session/c-confirm-dialog.png`, `c-s2-after-allow.png`, `c-s2-after-deny.png`, `result-C-*.json` |
| Off: read and send are both refused by policy | `m7-b4-b6-real-cli/b5d-setting-off.png`, `b5d-off-read-and-send-refused.png`, `b5d-s2-untouched.png` | M6: `m6-b5-b6-cross-session/d-s2-after-off.png`, `result-D-*.json` |
| Back to Auto: a send goes through with no dialog | `m7-b4-b6-real-cli/b5e-setting-auto.png`, `b5e-auto-send-no-dialog.png`, `b5e-s2-auto-again.png` | M6: `m6-b5-b6-cross-session/e-s2-after-auto-again.png`, `result-E-*.json` |

The 60 s timeout of a send at Confirm each send was checked only in M6
(`m6-b5-b6-cross-session/extra-timeout-send-while-confirm-no-click.json`).

## §B6: Settings rows

| Step | Final pass (M7) | Earlier |
|---|---|---|
| The toggle and the cross-session select in Settings → Environment, dark theme | `m7-b4-b6-real-cli/b6-dark-environment.png`, `b6-dark-select-menu.png` | M2: `m2-b6-settings-toggle/dark-toggle.png`; M6: `m6-b5-b6-cross-session/a-dark-environment.png`, `a-dark-select-menu.png` |
| The same, light theme | `m7-b4-b6-real-cli/b6-light-environment.png`, `b6-light-select-menu.png` | M2: `m2-b6-settings-toggle/light-toggle.png`; M6: `m6-b5-b6-cross-session/a-light-environment.png` |

## What no pass covered

- A real GPU: every pass rendered with lavapipe on Xvfb.
- macOS and Windows: every pass ran on Linux.
- Tool calls from Copilot other than `whoami` (M1, M7) and `delete_worktree` (M5).
