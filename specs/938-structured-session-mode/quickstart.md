# Quickstart: validating Structured Session Mode (#938)

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Parser, history, turn, log lines against fixtures | `cargo test -p micold-core --test structured_parse` and `mise run test-core` | every fixture line → one event, typed; tool calls paired; tolerance fixture: 0 failures, 0 lines lost (SC-001, SC-002) |
| Daemon against the stand-in | `cargo test -p micold-daemon --test structured_session --test structured_resume --test structured_lifecycle --test structured_history --test structured_saved_history` | US1–US5 and US7 scenarios; SC-003, SC-005, SC-006, SC-007 |
| Terminal sessions unchanged | `mise run gate` | every existing test green (SC-004) |
| Client reducer | `cargo test -p micold-client` | start-menu entries and chevron rule, kind on create, log model |

Contracts: [contracts/claude-stream-json.md](contracts/claude-stream-json.md),
[contracts/wire.md](contracts/wire.md). Entities: [data-model.md](data-model.md).

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` on `PATH`; `mise run app` (or the dev client) against a project.

1. Hover a worktree row: the chevron is there even with only Claude Code installed; its list shows
   "Claude Code (structured)" under "Claude Code".
2. Pick it: a session starts; its row is marked structured; the pane shows one plain line per
   event (init first), read-only, no input box.
3. Plain `+` press on another row: a terminal session, as before.
4. Stop and start the structured session: lines continue after the earlier ones; restart the
   daemon and view it again: with Save terminal history on, the earlier lines are there and the
   resumed run's lines follow them once it starts; with it off, the log starts at the resume.
5. A second window viewing the same session shows the same lines.
6. Light and dark theme: the log text and the marker read correctly in both.

Record the pass with the `visual-pass` skill.

## Capturing fixtures

`scripts/capture-stream-json.sh <name> "<prompt>"` in a scratch repo; check the README's CLI
version line; review the scrubbed output before committing.
