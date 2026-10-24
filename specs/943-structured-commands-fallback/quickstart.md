# Quickstart: validating Structured Session Commands (#943)

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Parsing the recorded command, model and auth fixtures | `cargo test -p micold-core --test structured_parse --test structured_commands` | `Init` lists, `CompactBoundary`, `ConversationReset`, synthetic assistant `model`, `error`, `is_synthetic`; `parse_initialize` on `command-initialize.jsonl`; `parse_set_model` success and `not_found` (FR-026, SC-001) |
| Command list, kinds, terminal-only, draft classification | `cargo test -p micold-core --lib -- structured::commands structured::model structured::control` | kind rule; alias `settings` → terminal-only; fallback list; `classify_draft` table (`/`, `/ x`, `/model`, `/model haiku`, `/mcp`, `/speckit-plan #1`, mid-text `/mcp`) (FR-001, FR-009–FR-011, FR-018) |
| Palette ordering and speed | `cargo test -p micold-core --test command_palette` | FR-008 tiers; SC-002 for every name in the recorded lists; 200 items filtered within budget (SC-003) |
| Terminal-only list equals the guide | `cargo test -p micold-core --test terminal_only_doc` | names, reasons, version match the guide's table (FR-012) |
| Transcript markers | `cargo test -p micold-core --test structured_transcript` | command chip from `UserSent`; `CommandOutput`; `Compacted` with summary; `Cleared` and eras; `ModelChanged`/refused; one `TerminalPeriod` per switch pair; `auth_failed` (FR-003–FR-006, FR-015, FR-022a, FR-023) |
| Persisted model and handle | `cargo test -p micold-core --test session_store --test protocol_roundtrip` | legacy record → `Default`, `None`; round trips; skipped when default (FR-017, FR-005a) |
| Daemon against the stand-in | `cargo test -p micold-daemon --test structured_commands --test structured_model --test structured_switch` | US1 (text unchanged on stdin, `/model` and terminal-only refused, never queued); `initialize` → `CommandsReported`; model idle/busy/refused/not running/concurrent/restart; handle after `/clear` used at restart and daemon restore; switch both ways with argv `--resume <handle> --model`, first input, confirmation, concurrent switches, message during switch held, queue held, Copilot refused (FR-026, SC-004–SC-006) |
| Window state | `cargo test -p micold-client --test features_command_palette --test features_session` | open/close rules, keys, Tab/Enter/click fill, Escape not interrupting, terminal-only offer, model picker items, switch offer and confirmation, kind change reattaches (US2, US3 scenarios 1–2, 5, US4 scenarios 1, 3, 5) |
| Everything else unchanged | `scripts/gate.sh` | every existing test green except the 938 FR-002 tests this feature replaces (SC-008) |

Entities: [data-model.md](data-model.md). Contracts: [contracts/](contracts/).

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` on `PATH`; #941's composer and #942's prompts on `main`; the dev
client against a scratch project with one project skill and one custom command.

1. Start a structured session. Type `/`: the palette lists skills, custom commands, built-ins and
   `/model`, grouped, with descriptions; `/config`, `/mcp`, `/agents`, `/login`, `/logout` say
   "opens in terminal" in words. Both themes.
2. Type `sp`: matches narrow at each key; `/speckit-plan` is selected after `/speckit-p`; Tab
   fills `/speckit-plan `; Enter sends nothing until pressed again (SC-002).
3. Send `/speckit-specify <text>`: runs to its first question, answered through #942's view
   (SC-007). Repeat with `/speckit-autopilot`.
4. Send `/context`: its output shows as markdown, no markup. Send `/compact`: busy until done, a
   *Compacted* marker with the summary collapsed. Send `/clear`: a *Cleared* marker, earlier turns
   marked; restart the session: Claude does not remember the cleared part (FR-005a).
5. Open the model picker from the view: Default and the reported models; pick Haiku while idle:
   marker, header shows Haiku (user's choice); pick during a long turn: the turn finishes on the old
   model; type `no-such-model`: the CLI's reason, model unchanged.
6. Send `/mcp`: the offer explains why; **Open in terminal**: the terminal shows Claude with `/mcp`
   typed in, same conversation; the queued count line shows any held message; **Open in structured
   view**: the transcript, a terminal-period marker, the same conversation, the draft still there.
7. Switch during a turn: the confirmation names the work that will end. Two windows: both follow.
8. A Copilot session offers no switch. A sandboxed Claude session switches inside its sandbox.
9. On macOS and Windows: palette keys, picker and switching behave the same.

## Results (close, 2026-10-24, `main` at 4910aeee)

Part A, run on Linux:

| Check | Result |
|---|---|
| Parsing the recorded fixtures | `structured_commands` 10 passed, `structured_parse` 20 passed |
| Command list, kinds, terminal-only, classification | core lib `structured::commands`/`model`/`control` 17 passed |
| Palette ordering and speed | `command_palette` 7 passed; the SC-002 check now also fails a list in which no name had a unique prefix to check (tdd-verify F3) |
| Terminal-only list equals the guide | `terminal_only_doc` 2 passed |
| Transcript markers | `structured_transcript` 71 passed |
| Persisted model and handle | `protocol_roundtrip` 45 passed, `session_store` 12 passed |
| Daemon against the stand-in | `structured_commands` 10, `structured_model` 10, `structured_switch` 11 passed |
| Window state | `features_command_palette` 18 passed, `features_session` 75 passed |
| Everything else unchanged | the close PR's full gate (`split-gate.sh`) green |

Part B ran live against `claude` 2.1.310 under Xvfb in both themes, recorded in
[visual-pass/m8-record.md](visual-pass/m8-record.md) (steps 1, 2, 4, 5) and
[visual-pass/m9-record.md](visual-pass/m9-record.md) (steps 3, 6, 7). Every step run passed, after
two bugs the passes found were fixed (the picker's name field took no keys; a switch's command went
out without its `/`). Not run, left for a human pass:

- Step 4: restarting after `/clear` to check Claude forgot the cleared part (FR-005a is covered by
  M5's daemon tests).
- Step 6: `/mcp` seen typed into the terminal — Claude Code first asked to trust the scratch folder,
  and the daemon does not type a switch's command over that prompt.
- Step 8: a Copilot session (no `copilot` CLI; covered by `features_session`) and a sandboxed
  session's switch (no container runtime; CI's sandbox job).
- Step 9: macOS and Windows.
- The `/login` card after an authentication failure (needs a signed-out CLI; M7's tests).
