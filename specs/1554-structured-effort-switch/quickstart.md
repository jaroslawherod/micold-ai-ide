# Quickstart: validating Structured Session Reasoning Effort (#1554)

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Levels, choice, support, argument | `cargo test -p micold-core --lib -- structured::effort structured::control structured::commands` | `EffortLevel::parse` (case, `default`/`auto`/`ultracode` refused as levels, length); `offered_levels`, `effective_level`, `availability` tables incl. unknown support and a model without effort; `parse_effort_arg` (`high`, `Max`, `default`, `auto`, `extreme`, `ultracode`, blank); `encode_apply_effort` level and null; `classify_draft` (`/effort`, `/effort  `, `/effort max`, `say /effort max`) (FR-002, FR-003, FR-007) |
| Recorded shapes | `cargo test -p micold-core --test structured_parse --test structured_commands` | `effort-change.jsonl` / `effort-launch.jsonl` answers parse; `parse_initialize` on `command-initialize.jsonl` fills `ReportedModel.effort`; `/effort` listed once as `App` with the CLI's `effort` built-in present (FR-009) |
| Marker and saved history | `cargo test -p micold-core --test structured_transcript` and `cargo test -p micold-core --lib -- structured::saved` | `EffortChanged`/`EffortRefused` → one marker each; a version-1 file with the old `CommandsReported` still reads (`effort: None`); new notes round-trip (FR-004, FR-010) |
| Store, argv, wire | `cargo test -p micold-core --test session_store --test protocol_roundtrip --test schema_hash` | legacy record → `Default`; `Default` not written; argv has `--effort <level>` only for `Some` (FR-010, FR-011) |
| Daemon against the stand-in | `cargo test -p micold-daemon --test structured_effort` | idle pick → one `apply_flag_settings` line, same pid, note, two clients see the summary ≤ 1 s; busy pick → no line until `result`, then after a deferred `set_model`; Default → `null`; refused idle → error, choice unchanged; refused deferred → `EffortRefused`, put back; process ended → error; not running → stored, note; `/effort …` messages never reach stdin and are never queued; relaunch, daemon restore and structured → terminal → structured all carry `--effort`; a stored level the model does not support → no `--effort`, launch succeeds; Copilot session refused; two sessions independent (FR-004–FR-013, SC-001–SC-004) |
| Window state | `cargo test -p micold-client --test features_command_palette --test features_effort_picker` | picker rows and marks, unsupported wording, `/effort` opens / picks / refuses with draft kept (FR-001–FR-003, FR-007) |
| Everything else unchanged | `scripts/gate.sh` (cloud) / `mise run gate` | every existing test green (SC-006) |

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` on `PATH`; the dev client against a scratch project.

1. Start a structured session: the status row shows `Effort: Default` beside the model (FR-001).
   Light and dark theme.
2. Open the effort picker: Default, low … max, Default marked "current" in words (FR-002). Pick
   `high`: the trigger says `Effort: high`, the transcript shows "Reasoning effort: high from here
   on", and a second window on the session shows both within a second (FR-004).
3. Send a long prompt; during the turn pick `low`: the turn finishes, the marker appears at its
   end, the next turn runs (US1 scenario 4).
4. Send `/effort max`, `/effort`, `/effort extreme` from the composer: max is picked; the picker
   opens; the accepted values are named and the draft stays. Nothing appears as a user message
   (US2).
5. Type `/eff`: one `/effort` row under **In this app** (FR-009).
6. Restart the session, then **Open in terminal**: the terminal Claude Code's `/effort` (no
   argument) reports the level picked last (`low` after step 3, `max` once step 4 has run);
   **Open in structured view**: the same level on the trigger (US3).
7. Guide: `docs/user-guide/worktrees-and-sessions.md` *Commands, the model and effort*:
   a. (M4) describes the picker, the levels, the unsupported-model wording and the terminal
      caveat;
   b. (M5) describes `/effort` (FR-014).

Entities: [data-model.md](data-model.md). Contracts: [contracts/](contracts/).
