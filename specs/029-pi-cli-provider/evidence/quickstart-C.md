# Quickstart §C — the activity component inside a real `pi`

Recorded 2026-09-13 for T056 (Principle I evidence for the spawn-wiring exception).

## Setup

- `pi` 0.85.1 on Node 22.23.2, as shipped in `micold-daemon:dev` built from this branch. The branch was later rebased onto a `main` whose image moved to Node 24; §C itself was not re-run there, but the rebuilt image reports `pi` 0.85.1 on Node v24.21.0 and `mise run test-sandbox` passes against it
  (`mise run image`). Neither `pi` nor Node is installed on the recording host, so every step ran in
  that image.
- `docker run --network none`: the container has no route off the device.
- **Model provider.** No real one. A local mock (`node mock.mjs`) listened on `127.0.0.1:18080` and was
  registered through Pi's own `models.json` as an `openai-completions` provider. Its first reply is one
  `bash` tool call (`echo SECRET_TOOL_OUTPUT`). Once a tool result is present it answers
  `SECRET_RESPONSE_TEXT`. The prompt was `SECRET_PROMPT_TEXT`. So a whole turn ran, tool call
  included, and every piece of conversation content was greppable.
- The component was `crates/micold-daemon/assets/pi-activity.ts` byte for byte. It was loaded with
  `pi -e <file> --provider mock --model m --session-id <uuid> -p …` and
  `MICOLD_PI_ACTIVITY_LOG=$PI_CODING_AGENT_DIR/micold-activity/<uuid>.jsonl`.

## Results

| Step | Result |
|---|---|
| C.1 events arrive | **Pass.** One run wrote, in order: `agent_start`, `turn_start`, `tool_execution_start`, `tool_execution_end`, `turn_end`, `turn_start`, `turn_end`, `agent_settled`, `session_shutdown`. Each maps through `pi_event` to working, then awaiting input, then ended. The badge itself was not watched in a GUI. The same lines through the daemon's tail are covered by `activity_pipeline.rs::a_pi_activity_log_moves_the_badge_through_the_unchanged_machine`. |
| C.2 nothing but activity | **Pass.** Every line has exactly the keys `at` and `type`. A grep for `SECRET`, `bash`, `echo`, `/tmp`, `mock` and the model id matched 0 lines. |
| C.3 user's `pi` untouched | **Pass.** `$PI_CODING_AGENT_DIR/extensions` and `<worktree>/.pi` do not exist, and `git status --porcelain` is empty. A `pi` run without `-e`, with `MICOLD_PI_ACTIVITY_LOG` set, wrote no log. |
| C.4 declining | **Pass, at the spawn rather than by `ps`.** `crates/micold-daemon/tests/pi_launch_wiring.rs` puts a recording `pi` on `PATH` and starts sessions through `DaemonState::start_session`. With the switch on, the process gets `--session-id <uuid> -e <materialised component>` and the log variable. With it off, the args are exactly `--session-id <uuid>` and the variable is unset. That the badge then reads Unknown is `activity_pipeline.rs::with_the_component_declined_a_pi_session_is_not_watched_and_reads_unknown`. Seeing one switch in the UI is covered by `features_settings.rs` and was not checked on a display. |
| C.5 failure degrades | **Failed as first written, fixed, re-run: pass.** See below. |
| C.6 no timer of ours | **Pass, structurally.** No timer, sleep or watch was added in any `src/` (T061). The tail is the existing `EventLogTail`. |
| C.7 nothing sent off the device | **Pass.** The run completed under `--network none`. `PI_OFFLINE=1`, `PI_SKIP_VERSION_CHECK=1` and `PI_TELEMETRY=0` are in the image's environment and, per `pi_launch_wiring.rs`, in every Pi session's environment whatever the switch says. |

`session_shutdown` is written when `-p` mode exits, which is `reason === "quit"`, and the TUI quit
takes the same path. The contract's note on the other reasons stands.

## C.5: Pi does not degrade on a broken extension

Pi exits 1 at startup, in `-p` mode and interactively alike, for **any** extension that fails to load:

```
Error: Failed to load extension "/tmp/broken.ts": Failed to load extension: ParseError: Unexpected token
Error: Failed to load extension "/tmp/throws.ts": Failed to load extension: boom
Error: Failed to load extension "/tmp/apichange.ts": Failed to load extension: Cannot read properties of undefined …
Error: Failed to load extension "/tmp/missing.ts": Extension path does not exist: /tmp/missing.ts
Hint: Start without extensions using "pi -ne".
```

So as first written, FR-012d held only for failures on our side of the spawn: an unwritable
component file or log directory means `-e` is omitted. Pi's extension API changing under a `pi` the
user installed would have stopped every Pi session from starting. Two changes followed:

1. `pi-activity.ts` wraps its whole body in `try`/`catch`. The same run with `pi.on` replaced by an
   undefined property now keeps an interactive session running (still up at the 8 s timeout, where it
   exited 1 before) and writes no log, so the badge reads Unknown.
2. `contracts/pi-cli.md` records that Pi treats load failure as fatal, and why FR-012d is met on
   our side.

A parse error in the shipped file itself is not guarded, because no code of ours runs before the
parse. This run loading the file cleanly is the check for that, and it has to be repeated whenever
the component changes, alongside C.2.

## Not run here

- **§B** (GUI start, resume, `/name`, close, and discovering a hand-run conversation) needs a display
  and a host `pi`. The same properties are covered headlessly by `pi_provider.rs`,
  `session_discovery.rs`, `session_archive_durable_marker.rs` and `session_reconciliation.rs`.
- **§D.1–2** in the application with sandboxed placement; since recorded in
  [`quickstart-D.md`](quickstart-D.md). `mise run test-sandbox` passed 23/23,
  including `sandbox_real_the_image_ships_every_ai_cli_the_application_offers`, which runs
  `pi --version` in a session inside the container.
