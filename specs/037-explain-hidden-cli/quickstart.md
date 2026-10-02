# Quickstart: Explain Why an AI CLI Is Not Offered

Contract rows are in [contracts/reason-wording.md](contracts/reason-wording.md) (W, U) and
[contracts/availability-answer.md](contracts/availability-answer.md) (A, S, C).

## Part A: automated

```bash
mise run test-core     # core: SpawnEnv::classify, explain and start_refusal over every state × place
mise run gate          # fmt, clippy, the whole workspace, then scripts/tests
```

Expected:

- `crates/micold-core/tests/cli_reason.rs` covers R2's table and W1–W3 (rules W2a–W2f, W3a–W3c).
- `crates/micold-core/tests/schema_hash.rs` and `protocol_roundtrip.rs` cover A1.
- `crates/micold-daemon/tests/ai_cli_availability.rs` covers S1–S7 and A3 (no extra script run).
- `crates/micold-daemon/tests/session_start.rs` covers U4, and `tests/mcp_create_session.rs` U5.
- `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs` covers U1, U2 and W5.
- `crates/micold-client/tests/unavailable_default_says_so.rs` covers U3.
- `crates/micold-client/tests/directory_availability.rs` covers C2, C3 and U6's rule.
- `crates/micold-client/src/ui/material/menu_anatomy.rs` covers W7.

## Part B: visual pass (run with the `visual-pass` skill)

Needs a private Xvfb display, a scratch data directory and the real session service. Put a fake
CLI in a scratch directory that only the include script adds to the `PATH`:

```bash
mkdir -p "$SCRATCH/bin" && printf '#!/bin/sh\n' > "$SCRATCH/bin/pi" && chmod +x "$SCRATCH/bin/pi"
printf 'export PATH="%s/bin:$PATH"\n' "$SCRATCH" > "$SCRATCH/env.sh"
```

`claude` and `copilot` stand-ins go on the login `PATH` the same way where a step needs them.
Seed `settings.json` per step, start the client, and open Settings → Environment.

| Step | Seed (`env_include_enabled`, `env_include_script_path`) | Pass when |
|---|---|---|
| B1 | `false`, `$SCRATCH/env.sh` | the note under **Default AI CLI** is W2's `IncludeOff` row for Pi Coding Agent, and the selector does not list it. US1 scenario 1 |
| B2 | from B1: tick **Source a script before each session**, Save, reopen Settings | the selector lists Pi Coding Agent and no note is shown, with no restart. US1 scenario 2, SC-002 |
| B3 | `true`, `""` | the `NoScriptPath` row. It does not say to turn the setting on. US1 scenario 4a |
| B4 | `true`, `/tmp/does-not-exist.sh` | the `ScriptNotFound` row, naming "your home directory". US1 scenario 4 |
| B5 | `true`, a script that runs `exit 3` | the `ScriptFailed` row. US1 scenario 4 |
| B6 | `true`, a script that runs `sleep 30`, timeout `1` | the `ScriptTimedOut` row, naming "Script path" and "Timeout". US1 scenario 3 |
| B7 | `true`, a script that adds nothing | the `Applied` host row: "was not found on the PATH sessions get for your home directory". US1 scenario 5 |
| B8 | every CLI on the login `PATH` | no note. US1 scenario 6 |
| B9 | B1's seed, stored default Pi Coding Agent, a project open; press start on its row | a notification with `start_refusal`'s `IncludeOff` sentence, the list of available CLIs opens, nothing starts. US2 scenario 1 |
| B10 | a session on Pi started under B2's settings; then B6's script, Save; restart the session | one banner gives the `ScriptTimedOut` reason, says to restart afterwards, and does not say "install"; the pane gives the same sentence when it has no terminal content to keep (after a restart in place it keeps the terminal and the bar reads "failed restart"). US2 scenario 2 |
| B11 | two available CLIs and Pi missing for a project's directory; press the row's chevron | the list shows the two CLIs, a divider, and the note naming Pi Coding Agent with the reason and action for that directory. Pressing the note does nothing. US3 scenarios 1 and 4 |
| B12 | one available CLI | the row has no chevron and nothing new. US3 scenario 1a |
| B13 | every CLI available for the row | the list has no divider and no note. US3 scenario 2 |

Also check: the Settings note wraps inside the field's column and does not overflow the page; the
menu note wraps inside the panel, the panel stays inside the window when opened from the lowest
row, and both read correctly in the light and dark themes.

Container placement (B14, needs a container runtime and `mise run image`): with the `:dev` image
and environment-include off, the notes under *Image reference* and **Default AI CLI** give the
`IncludeOff` image row. With a script that succeeds, they give today's "isn't in *<image>*"
sentence. US2 scenarios 4 and 5, FR-005. Where no runtime is installed, record B14 as covered by
Part A (`missing_cli_is_reported_where_it_is_chosen.rs` asserts both image rows) and say so in the
evidence.

Save screenshots under `specs/037-explain-hidden-cli/evidence/`.
