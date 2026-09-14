# Quickstart §B — a real `pi` session in the application

Recorded 2026-09-13, after the feature merged (`3607ba8e`).

## Setup

- **Display.** No person at a screen. The app ran on a private Xvfb display with Mesa's lavapipe
  (software Vulkan), driven by `xdotool`, following `.claude/skills/visual-pass`. This covers static
  appearance and state changes. It says nothing about rendering on a real GPU.
- **Build.** Client and daemon came from one locked debug build of `9cb97120` (the branch head that
  was rebase-merged) and were pinned outside `target-shared`. Both binaries contain the new strings,
  and the daemon log shows `client attached to daemon`.
- **Pi.** `pi` 0.85.1 on Node v24.21.0, installed into a private npm prefix. `PI_CODING_AGENT_DIR`
  pointed at a fresh directory.
- **Model.** No real provider. A local OpenAI-compatible mock was registered through Pi's
  `models.json`, made the default in Pi's `settings.json`, and answered every turn with
  `MOCK reply #<n> to: <last user message>`. `<n>` counts the user messages Pi sent, so it shows how
  much history Pi loaded.
- **Project.** A fresh one-commit git repository, opened by seeding `projects.json`. The app ran with
  private `XDG_*` directories. `claude` and `copilot` were not on `PATH`.

## Results

| Step | Result |
|---|---|
| B.1 open a project | **Pass.** |
| B.2 Pi as a one-off, identifiable row and tab | **Pass, by the no-default route.** Claude Code, the default CLI, was not installed, so `+` on the project row offered *Pi Coding Agent* and the app said so. The explicit override picker, with the default CLI available, was not exercised. The terminal shows `pi v0.85.1` in the project directory with `[Extensions] pi-activity.ts`. The row carries the text label `pi` (FR-009), and the pinned AI tab shows the glyph plus `pi` (FR-010). See [`quickstart-B-sidebar.png`](quickstart-B-sidebar.png), rows 1 and 5. |
| B.3 one conversation file under the app's id | **Pass.** After one message, `sessions/--<cwd>--/` held exactly `2026-09-13T18-21-19-646Z_b111368a-78c6-4f22-9e2c-476925c90fcd.jsonl`, the session id the app launched with (FR-005a, SC-004b). The activity log for that id recorded `agent_start` … `agent_settled`, and the row gained its badge (row 2). |
| B.4 resume by hand | **Pass.** With the app and daemon stopped, `pi --session-id b111368a-… -p "resumed by hand"` in the project answered `MOCK reply #2`: Pi loaded the earlier turn. Still one file. |
| B.5 restart and resume | **Pass.** Restarting only the client reattached to the running `pi`. Stopping client *and* daemon ended `pi` (`session_shutdown` logged). On relaunch the daemon logged `presented interrupted-resumable sessions after restart count=1` and `session started … launch=Resume`. The terminal showed all earlier turns, including the hand-resumed one, and the next message got `MOCK reply #3` ([`quickstart-B-resume.png`](quickstart-B-resume.png)). Still one file (FR-005, SC-002). |
| B.6 `/name` and the fallback chain | **Pass after finding 1 was fixed** (re-run 2026-09-14). On 2026-09-13 the running row showed Pi's terminal title (`π - demo`, then `π - my task - demo`). The re-run used the same setup, with client and daemon from one locked build of the fix commit below. A new session's row read *New session*. After the first message it read `why is the row wrong`. After `/name my task`, which recorded `session_info` `name: "my task"`, it read `my task` within seconds ([`quickstart-B-label-chain.png`](quickstart-B-label-chain.png), top to bottom). |
| B.7 close and do not rediscover | **Pass.** Closing the row left `<id>.jsonl` and `<id>.archived` side by side. Reopening the project and pressing *Refresh* did not bring the row back (FR-016). |
| B.8 a `pi` run outside the app | **Pass.** With the app stopped, `pi -p "typed outside the app"` in the project created a second conversation. On relaunch the daemon logged `adopted sessions started outside this application count=1`. The row reads `typed outside the app` with the `pi` label and an empty badge slot (row 4), which is how `Unknown` renders (`activity_badge.rs`: `Unknown => None`) (FR-015, FR-013, FR-011 first-message fallback). The *Default* group was collapsed after this relaunch, so the row appeared only after expanding it (finding 3). |

## Findings

1. **Fixed: a running Pi session's row showed Pi's terminal title, not the name Pi recorded (FR-011).**
   `overlay_live_summaries` (`crates/micold-daemon/src/state.rs`) replaces the label with
   `live.last_title`, the terminal's OSC title, whenever one exists. That is right for Claude Code,
   whose terminal title *is* its summary. Pi sets its title to `π - <cwd basename>`, and to
   `π - <name> - <cwd basename>` once named. So for every supervised Pi session, the FR-011 chain
   (recorded name, then first message, then placeholder) is never visible. `read_title` is only
   reached for discovered rows. The unit and integration tests cover `parse_title` and discovery,
   and do not run a live `pi` that sets a title, which is why this passed the gate.
   **Fix, in two parts.** Feature 029-persistent-session-names (T030, `30e1fd70`) gave each CLI
   `name_in_terminal_title`. Pi's strips `π - … - <folder>` and reads the unnamed `π - <folder>` as
   no name, so the banner no longer reaches the row. That alone left a running unnamed row on
   *New session* until a refresh, skipping the first-message fallback. So a live session is now
   flagged when it starts or its activity moves, and the supervisor's blocking hop reads the
   flagged, still-unnamed sessions' names from their stores.
   `a_running_pi_row_reads_its_first_message_until_it_is_named`
   (`crates/micold-daemon/tests/activity_pipeline.rs`) covers the whole chain against a terminal
   that retitles the way Pi does.
2. **Each new Pi session opens with a warning.** Pi prints `Warning: No project session found with id
   '<uuid>'; creating a new session with that id.` in yellow as the terminal's first line. This
   follows from addressing a new conversation by `--session-id` (FR-005a) and is Pi's own output.
   It is harmless but reads as an error to a first-time user.
3. **Not specific to 029: the project-root group is collapsed after relaunch.** After relaunching
   with no running session, *Default* was collapsed, so the adopted row was hidden until expanded.
   After the relaunch in B.5, which had a running session, it was expanded. Recorded so it is not
   mistaken for discovery failing.

`fd not found` and `ripgrep not found` in the terminal come from the recording host and offline mode,
not from the application.

## Not covered

- Rendering on a real GPU or display; see Setup.
- A session in a linked worktree: the demo project had none, so every session ran at the project
  root. The code path differs only in `cwd`.
- The override picker while the default CLI is installed.
