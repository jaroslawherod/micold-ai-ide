# Visual pass, quickstart Part B (M2)

2026-10-09, Xvfb 1600x1400 + lavapipe, client and daemon built from this worktree (claude/project-thread-wysm57).
Private data home with a seeded git project. Daemon PATH: stand-in `codex` (`exec sleep 600`) plus `/usr/bin:/bin`; no `opencode`. Not a real display. Real CLIs (Part C) not exercised.

| Check | Verdict | What was seen |
|---|---|---|
| Chooser, Codex selectable | PASS | List shows `Codex` ([m2-2](visual/m2-2-chooser-note.png)). |
| Chooser, OpenCode unavailable with reason naming opencode | PASS with a precondition | With `codex` as the only CLI there is no chevron (needs 2+ available) and the primary press opens a list of just `Codex` with NO note ([m2-1](visual/m2-1-chooser-codex-only.png)), so OpenCode is not named. With a second stand-in (`claude`) added, the chevron appears and the list shows Claude Code, Codex and a note: "GitHub Copilot, Pi Coding Agent and OpenCode were not found on the PATH sessions get for <dir>: the login PATH plus what the startup script adds. Install them, or make the script add their directories." OpenCode is not an entry. |
| Settings, Default AI CLI lists all five providers | FAIL as worded | Dropdown lists only the two available entries, Claude Code and Codex ([m2-4](visual/m2-4-settings-default-cli.png)). The other three are named only in the note under the field ([m2-5](visual/m2-5-settings-note.png)): "GitHub Copilot, Pi Coding Agent and OpenCode were not found on the PATH ...". Unavailable providers are not hidden without a reason. |
| Codex session row labelled `codex` | PASS | Row "New session", tag `codex` ([m2-3](visual/m2-3-codex-row.png)); status bar "running". |
| Activity badge `Unknown` | PASS | Unknown draws no dot by design (`activity_badge::emphasis` returns None); the row shows an empty badge slot. No literal "Unknown" text exists in the UI. |
