# Quickstart: Per-project setup command and PATH for project toolchains

Validation guide for feature 1289. Part A runs in CI. Part B needs a real sandbox and a display,
and is run by the `visual-pass` skill and by hand. Contracts: [toolchain-path](contracts/toolchain-path.md),
[wire-and-settings](contracts/wire-and-settings.md), [setup-ui](contracts/setup-ui.md).

## Part A: automated

Run `scripts/gate.sh`, or `mise run gate` where mise is available. The scenarios it must cover,
with the test files the tasks unit creates:

| # | Scenario | Layer, file |
|---|---|---|
| A1 | P3's table, row for row; duplicates; no `HOME`; empty vars | core: `crates/micold-core/tests/toolchain_path.rs` |
| A2 | A sandbox daemon (with `offered.json`) gives a terminal and an AI CLI launch the P2 order; one with no CLIs listed still adds the home folders; a daemon without `offered.json` leaves `PATH` as it was | daemon: extends the `with_offered_path` test near `state.rs:8389` |
| A3 | Availability: a stub `claude` placed only in `$HOME/.local/bin` of a sandbox daemon is not offered ahead of the offered one, and a stub tool in the shims folder resolves on `spawn_path_for` | daemon unit |
| A4 | `SetupRun` transition table; `OutputBuffer` (CR/LF, lone CR, ANSI stripping, 10,000-line cap with `dropped`, indices); `shell_line` table; `SetupCommand::parse` with blank input | core: `crates/micold-core/tests/setup_model.rs`, `setup_output.rs`, `setup_shell.rs` |
| A5 | `StreamingChild`: interleaved output in order per stream; stdin at EOF (`read` gets 0 bytes); `terminate` kills a grandchild (`sh -c 'sleep 300 & wait'`); spawn failure | core: `crates/micold-core/tests/streaming_child.rs` (Unix and Windows variants) |
| A6 | Create through `WorktreeCreate` with a command `echo hi; touch marker`: `SetupChanged` Running then Succeeded; the marker exists; a watcher gets `hi`; the create answers before setup ends | daemon: `crates/micold-daemon/tests/setup_run.rs` |
| A7 | Same through MCP `create_worktree`: the result has `"setup": "started"`; with no command it has `"none"` | daemon: `crates/micold-daemon/tests/mcp_create_worktree_setup.rs` |
| A8 | `exit 3` gives Failed(Exit 3) and the last line *Exited with status 3.*; a missing folder gives NotStarted | daemon `setup_run.rs` |
| A9 | `SetupRun` while running gives `Busy`; `SetupStop` gives Stopped and the process group is gone; Run setup on Default runs in the project root | daemon `setup_run.rs` |
| A10 | `WorktreeInclude` and refresh-discovered worktrees start nothing (FR-020) | daemon `setup_run.rs` |
| A11 | Restart: a `Running` record on disk loads as Interrupted with its log; `unwind` interrupts a live run | daemon `setup_run.rs` |
| A12 | Delete while running: stopped and its end awaited before the git remove, then the record and log are removed, and `SetupChanged { run: None }` is pushed. Once the delete has answered, no `<location>.json` or `.log` exists and no further `SetupChanged` for that location arrives (a late `finish` is dropped). Delete, recreate the same `dir_name`, run a failing command: its `RunId` differs from the deleted run's | daemon `setup_run.rs` |
| A13 | Attach catch-up: a client attaching after a run ended gets its `SetupChanged` | daemon `setup_run.rs` |
| A14 | `settings.json` round-trip of `setup_commands`; a host-daemon service-settings write keeps the map | core `settings.rs` tests; daemon `catalog.rs` tests |
| A15 | Protocol round-trip of every new message; version 69 | core `messages.rs` tests |
| A16 | Client reducer: menu predicates (Story 3 scenarios 3–5); one notification per `RunId`, also through catch-up, and a failure in a worktree deleted and recreated under the same location notifies again (new `RunId`); `ProjectSetupSet` sent on connect and on save to the daemons that hold the project, carrying each daemon's catalog path (a mapped container daemon gets the mapped path) | client: `crates/micold-client/tests/features_setup.rs` |
| A17 | Settings › Project: a blank value clears the entry; the field follows the open project | client: `crates/micold-client/tests/features_settings.rs` (extend) |
| G1 | Geometry gate: a row with a setup tag keeps its name and state icon where they were, and a long display name with a tag is truncated, not pushed | client: `crates/micold-client/tests/setup_tag_geometry.rs` (pattern: `jira_status_geometry.rs`) |
| G2 | Geometry gate: the output dialog at 10,000 lines builds only the visible rows, and the action row stays inside the dialog at the smallest window | client: `crates/micold-client/tests/setup_output_geometry.rs` |

## Part B: real sandbox and visual pass

Prerequisites: a container daemon on `ghcr.io/jaroslawherod/micold-basic`, and a git project
whose repository has a `.mise.toml` pinning one tool (for example `python = "3.12"`). Build the
image with `mise run image` (or use the published one), then `mise run test-sandbox` for the
sandbox suites.

1. **B1, PATH (Story 1).** In a sandbox session's regular terminal, run
   `mkdir -p ~/.local/bin && printf '#!/bin/sh\necho probe\n' > ~/.local/bin/probe-tool && chmod +x ~/.local/bin/probe-tool`.
   In a new terminal tab, `probe-tool` prints `probe` without a restart (scenario 7). Run
   `echo $PATH`: the offered CLI bin comes first, then `~/.local/bin`, then the mise shims folder.
2. **B2, MCP server.** Install `uv` into `~/.local/bin` (`curl -LsSf https://astral.sh/uv/install.sh | sh`).
   Add `.mcp.json` with `{"mcpServers":{"t":{"command":"uvx","args":["mcp-server-time"]}}}`.
   Start a Claude Code session: `/mcp` lists `t` as connected (SC-001).
3. **B3, setup (Story 2).** Settings › Project: set **Setup command** to
   `mise trust && mise install`, then **Save**. Create a worktree from the form:
   - the row shows *Setting up…* at once, and a session starts in it at once;
   - **Setup output** shows mise's output as it prints;
   - the row then shows *Set up*;
   - a session in it runs `python --version` with no install step (SC-002).
4. **B4, failure.** Change the command to `echo before; exit 2` and create a worktree:
   - the row shows *Setup failed*;
   - one error notification reads *Setup failed in <project> / <worktree>: Exited with status 2*;
   - the output view ends with *Exited with status 2.* (SC-004).
   Then **Run setup** with the command fixed: Running, then *Set up* (SC-006).
5. **B5, stop.** Set the command to `sleep 600` and create a worktree. Choose **Stop setup**: the
   row shows *Setup stopped*, and `ps` in a terminal shows no `sleep`.
6. **B6, AI-created.** In a Default session, ask Claude to call `create_worktree`. The tool result
   has `"setup": "started"`, and the new row shows *Setting up…*.
7. **B7, reconnect.** While `sleep 60; echo done` runs, quit the app and reopen it. The row still
   shows *Setting up…*, and the output view later shows `done`.
8. **B8, host daemon.** On a host daemon, run the same command. It runs in the user's own
   environment, and `echo $PATH` in a host session is unchanged from before the feature (FR-008).

**Visual pass** (`visual-pass` skill, light and dark): the five row tags in U2, the three menu
entries in U3, and the output dialog in U4, empty, mid-run, failed and with dropped lines. Check
U1's section, with a project open and with none. Screenshots go in `visual-pass/`.

## Results

(Filled in by the close unit.)
