# Quickstart: Terminal History That Survives a Session Service Restart

Contracts: [saved-history-file](contracts/saved-history-file.md), [setting](contracts/setting.md),
[stop-request](contracts/stop-request.md). Types: [data-model.md](data-model.md).

## Part A: automated

```bash
mise run test-core     # format, damage checks, fixture, schedule, store, texts, owner_only, settings, protocol
mise run gate          # fmt, clippy, the whole workspace, then scripts/tests
mise run image && mise run test-sandbox   # needs a container runtime; the sandbox rows below
```

Expected:

- `crates/micold-core/tests/terminal_history_format.rs` covers saved-history-file §2, §4 and §5
  (round trip, every damage reason, random and truncated bytes, the pinned fixture).
- `crates/micold-core/tests/terminal_history_schedule.rs` covers FR-003, FR-004, FR-007 and SC-003.
- `crates/micold-core/tests/terminal_history_store.rs` covers save, load, forget, the setting,
  sweep, purge, the container rule and the file modes (saved-history-file §3 and §6).
- `crates/micold-core/tests/schema_hash.rs` and `protocol_roundtrip.rs` cover setting §2.
- `crates/micold-daemon/src/history.rs` unit tests cover capture and seed (FR-001, FR-009, FR-011,
  FR-012).
- `crates/micold-daemon/tests/history_restart_in_run.rs` covers story 1 scenarios 9 and 10, story 2
  scenario 8, FR-010, FR-014, `ESC[2J` and the alternate screen (R13, R16).
- `crates/micold-daemon/tests/history_service_restart.rs` covers story 1 scenarios 2 to 7.
- `crates/micold-daemon/tests/history_stop_request.rs` covers story 1 scenarios 1 and 8 and
  stop-request §6.
- `crates/micold-daemon/tests/history_setting.rs` covers story 2 scenarios 2 to 7.
- `crates/micold-daemon/tests/history_damaged.rs` covers story 3.
- `crates/micold-daemon/tests/history_removal.rs` covers story 4.
- `crates/micold-daemon/tests/history_timing.rs` covers FR-013, SC-004 and SC-005.
- `crates/micold-client/tests/features_settings.rs` and `settings_sections.rs` cover story 2
  scenario 1 and setting §4.
- `crates/micold-daemon/tests/sandbox_real_history.rs` covers FR-021, FR-022 and the sandbox stop.

## Part B: visual pass (run with the `visual-pass` skill)

Needs a private Xvfb display, a scratch data directory (`XDG_DATA_HOME`) and the real session
service and client built from the branch. Use a fake AI CLI that prints on the normal screen where
a step does not name a real one:

```bash
mkdir -p "$SCRATCH/bin"
cat > "$SCRATCH/bin/pi" <<'EOF'
#!/bin/sh
i=1; while [ $i -le 200 ]; do printf '\033[1;32mline %s\033[0m plain \033[38;5;208morange\033[0m\n' "$i"; i=$((i+1)); done
exec cat
EOF
chmod +x "$SCRATCH/bin/pi"
```

| Step | Do | Pass when |
|---|---|---|
| B1 | Start a session on the fake CLI, wait for line 200, stop it, start it | Scrolling up shows lines 1 to 200 in bold green and orange, then one dim line `── session restarted at <date> <time> <offset> ──`, then the new lines 1 to 200. Story 1 scenario 9, SC-011 |
| B2 | From B1: **Restart service** in *Session service diagnostics*, then open the session | The history from B1, its separator, and a second separator with a later time. Story 1 scenarios 1, 2 and 4, SC-001, SC-010 |
| B3 | B2 in the light and in the dark theme | The separator is readable and distinct from program output in both. FR-009 |
| B4 | Open the session of B2 in a second window | Both windows show the same history and separators. Edge case *Several windows* |
| B5 | Make the window narrower than the separator, restart the session | The separator is one row; older lines reflow. Edge case *Terminal size changed* |
| B6 | Settings → Terminal | A checkbox **Save terminal history**, ticked, with the note of setting §4, aligned with the section's other controls and inside the page width. Story 2 scenario 1, FR-031 |
| B7 | Untick it, Save; `grep -r "line 200" "$XDG_DATA_HOME"` | Nothing found, and `terminal-history/` holds no file. The terminal still shows its history. Story 2 scenarios 5 and 6, SC-008 |
| B8 | With saving off: stop and start the session, then **Restart service** and open it | The stop and start shows the history and a separator; after the service restart the terminal is empty with no separator. Story 2 scenarios 2 and 8 |
| B9 | Saving on again; let a session save; overwrite its file with `head -c 4096 /dev/urandom`; **Restart service**; open it | The session runs; the terminal shows one dim line `── earlier output could not be restored ──` and no garbage; *Session service diagnostics* lists the warning with the reason. Story 3 scenarios 1 and 2 |
| B10 | A session with a saved file; **Remove** it | Its file is gone from `terminal-history/`. Story 4 scenario 1, SC-007 |
| B11 | `ls -la "$XDG_DATA_HOME/micold-ai-ide/terminal-history"` | Directory `drwx------`, files `-rw-------`. SC-009 |
| B12 | Real Pi Coding Agent: a short conversation, **Restart service**, open the session | The earlier conversation is above the separator; Pi's resumed view is below it. Edge case *A session that resumes* |
| B13 | Real Claude Code with `CLAUDE_CODE_NO_FLICKER=0` in the include script: the same | As B12 |
| B14 | Real Claude Code and Copilot CLI with their defaults: the same | Record what is shown. Expected: the CLI's own full-screen view with its resumed conversation; the saved screen and the separator are not visible while it runs (D11). The user guide's sentence matches what is seen |

Where a real CLI is not installed or not signed in, record the step as not run and say so; B1 to
B11 do not depend on one.

Container placement (B15, needs a container runtime and `mise run image`): run B1 and B2 with the
session service in the sandbox. The separator shows the host's local time and offset, and the file
is in the host's `terminal-history/` with the modes of B11. Where no runtime is installed, record
B15 as covered by `sandbox_real_history.rs` in CI's sandbox job and say so.

Not automatable here: a real Windows logout or reboot (stop-request §5), and the file's DACL as seen
by a second Windows account. Both are covered as far as CI can by the `cfg(windows)` tests of
stop-request §6 and of `owner_only`; record them as manual checks for a Windows machine.

Save screenshots under `specs/041-terminal-scrollback-persistence/evidence/`.
