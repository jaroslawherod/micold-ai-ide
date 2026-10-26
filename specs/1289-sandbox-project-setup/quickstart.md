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
| A15 | Protocol round-trip of every new message; version 70 | core `messages.rs` tests |
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

Recorded by the close unit on 2026-10-26, after M7 (#1476) merged at `308ab390a`. The close unit's
container has no container runtime (the `docker` client is installed, no engine runs) and no Claude
Code sign-in, so nothing that needs a sandbox or an AI CLI was run by hand here. Each step says what
proved it instead.

### Part A

Every A and G row ran in `scripts/gate.sh` before each milestone's PR (the ledger's *Handover* lists
each green gate) and in CI's Linux `Test (full workspace)` step. Counts at the last milestones:
`features_setup` 33, `setup_output_geometry` 11, `setup_tag_geometry` 3 (client); `setup_model`
13, `setup_output` 12, `setup_shell` 4, `streaming_child` 3, `toolchain_path` 17 (core, macOS);
`setup_run` 12 and `mcp_create_worktree_setup` 2 (daemon, macOS).

### Part B

| Step | Result | Evidence |
|---|---|---|
| B1 PATH | Order proven in a real container, not by hand | CI `sandbox against a real runtime (linux)`, run [40305573743](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/40305573743) at M6's merge `419198cb9`: `sandbox_real_toolchain_path_puts_the_home_folders_after_the_offered_bins ... ok` (offered bin, then `~/.local/bin`, then the mise shims). The new-tab pickup of `probe-tool` (scenario 7) was not run: no container runtime here |
| B2 MCP server via `uvx` (SC-001) | Not run | Needs a container runtime, network access for `uv`, and a signed-in Claude Code. A1–A3 cover the `PATH` that `uvx` resolves on |
| B3 setup (SC-002) | Run on a host daemon, with `sleep`/`true` in place of `mise trust && mise install` | Visual passes M5 and M7 (`visual-pass/m5-u1-u2-notes.md`, `m7-u3-u4-notes.md`): a command saved in Settings › Project, a worktree created from the form, the row *Setting up…* then *Set up*, output shown as it printed (`m6-u4-*-running-*`). `python --version` with no install step (SC-002) needs the sandbox image and was not run |
| B4 failure (SC-004, SC-006) | Pass, host daemon | `echo before; exit 2`: *Setup failed*, one notification "Setup failed in demo-app / … : Exited with status 2.", output ends *Exited with status 2.* (`m5-u2-*`, `m6-u4-*-failed-*`, `m7-u3-*-failed-notification`). Run setup from the row menu then ran to *Set up* with `echo ok` (`m7-u3-light-row-setup-succeeded`) |
| B5 stop | Pass, host daemon | `sleep 900`, Stop setup from the row menu, the card menu and the output dialog: *Setup stopped*, `ps` showed no `sleep` after each (`m7-u3-light-rows-stopped-tags`, `m7-u4-*-after-stop`) |
| B6 AI-created | Not run by hand | Needs a signed-in Claude Code. A7 (`mcp_create_worktree_setup`, both cases) proves the tool result's `"setup": "started"`/`"none"`; the row tag is the same `SetupChanged` path B3 showed |
| B7 reconnect | Not run by hand | A11 and A13 (`a_running_record_found_at_start_is_interrupted_with_its_output`, `attaching_later_gets_each_locations_last_run`) cover a daemon restart and a late attach. The M5 and M7 visual passes killed the host daemon mid-run and saw *Setup interrupted* after the client respawned it; a client quit and reopen while the daemon kept running was not driven |
| B8 host daemon (FR-008) | Proven by tests | A2's "a daemon without `offered.json` leaves `PATH` as it was" (`state::offered_tests::a_missing_or_malformed_file_changes_nothing`, green on Linux, macOS and Windows) and every visual-pass run, which used a host daemon and ran the command in the user's own environment |

**Visual pass, U1–U6, light and dark** (run per milestone on Xvfb with Mesa lavapipe, a host daemon,
1600×1000 and 640×480; not redone here):

| Contract | Milestone | Verdict | Evidence |
|---|---|---|---|
| U1 Settings › Project | M5 | Pass (V-F2 placeholder fixed) | `m5-u1-*`, `m5-u1-u2-notes.md` |
| U2 row tags, all five | M5, M7 (Stopped) | Pass | `m5-u2-*`, `m7-u3-*-rows-*`, `m7-u3-light-default-stopped-tag` |
| U3 menu entries | M6 (Setup output), M7 (Run, Stop) | Pass (M6 V-F2 menu left open, fixed) | `m6-u3-*`, `m6-r2-*-menu-*`, `m7-u3-*-menu-*`, `m7-u3-u4-notes.md` |
| U4 output dialog: empty, running, failed, dropped, stop | M6, M7 | Pass after M6 round 2 (V-F1 follow on resize, V-F3 scrollbar, fixed) | `m6-u4-*`, `m6-r2-*`, `m7-u4-*`, `m6-u4-u6-notes.md` |
| U5 failure notification | M5, M7 | Pass, one per run, none on stop | `m5-u2-light-failed-row-notification-tooltip`, `m7-u3-*-failed-notification` |
| U6 delete while running | M6 | Pass | `m6-u6-*` |

M7's visual pass found that clearing a setup command did not persist (V-F1); it was fixed in M7. Busy
(a second Run setup while one runs) is not reachable from the UI and was not seen on screen; A9 covers
the daemon's answer.

### What CI's macOS and Windows runners proved (T061)

Evidence: the `build + test` jobs of run [40305573743](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/40305573743)
(main push at M6's merge `419198cb9`, which runs the daemon suites in full) and of PR #1476's run
[40314588666](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/40314588666) (green;
an ordinary pull request runs the core suite on both, not the daemon's).

| Suite | macOS | Windows |
|---|---|---|
| `setup_model`, `setup_output` (core) | 13, 12 passed | 13, 12 passed |
| `setup_shell` (core) | 4 passed | 3 passed (the `sh` case is `cfg(unix)`) |
| `streaming_child` (core) | 3 passed (Unix variant) | 4 passed (Windows variant: job-object tree kill) |
| `toolchain_path` (core) | 17 passed | compiled out (`#![cfg(unix)]`: a sandbox is a Linux container) |
| daemon lib `setup::tests`, `state::offered_tests`, `catalog::…keeps_a_setup_command` | all passed (lib 202/202) | all passed; the lib target was red on two `supervisor::container_git_tests` (a `\` path separator in another feature's container git env), none of 1289's |
| daemon `setup_run` (A6, A8–A13), `mcp_create_worktree_setup` (A7) | 12 and 2 passed (`mcp` binary 214/214) | compiled out (`#![cfg(unix)]`) |
| client `features_setup`, `features_settings`, geometry gates | not run (Linux full-workspace step only) | not run (Linux only) |

So the setup model, the output buffer and the process-tree stop are proven on all three platforms;
the daemon's end-to-end setup runs on Linux and macOS; the client reducer and geometry on Linux only.
What needs a sandbox, `sandbox_real_toolchain_path`, compiles to 0 tests outside its feature and runs
only in the Linux `sandbox against a real runtime` job (1 passed at `419198cb9`): GitHub's macOS and
Windows runners have no container runtime.
