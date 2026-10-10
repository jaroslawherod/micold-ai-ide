# Research: Show Claude Plan Usage and the Next Limit Reset

Feature 490. Each decision records what was chosen, why, and what was rejected. Sources:
<https://code.claude.com/docs/en/statusline> (read 2026-10-10), the workspace at `fddb3ebf`.

## R1. Usage source (FR-012) — status-line `rate_limits`, behind a source seam

- **Decision**: take readings from the `rate_limits` object Claude Code passes on stdin to its
  status-line command, in the Claude Code sessions the application runs. Everything after the parse
  works on a source-neutral `UsageReading` (data-model.md), so the source is one module on each
  side: `micold_core::plan_usage::status_line` (parse) and the daemon's status-line relay (R3). The
  store, the wire message, the settings and the indicator do not know where a reading came from.
- **Rationale**: the only documented source that reports plan percentages and reset times
  (`rate_limits.five_hour` / `seven_day`, `used_percentage` 0–100, `resets_at` epoch seconds;
  present for Pro/Max subscribers after a session's first API response). It needs no credential
  and makes no request of its own. Decision 1 in the ledger (orchestrator default, awaiting user
  confirmation).
- **What a different answer changes**: option B (local usage logs) replaces the parse module and
  the relay with a log reader that yields `UsageReading`; the store, wire, settings, indicator and
  their tests stay. Option C ships nothing. The relay and its settings-file block are one
  milestone (plan.md, M2), so dropping or swapping them leaves M1 and M3 intact.
- **Alternatives rejected**: Anthropic Admin/usage APIs (API-key organisations, not plans; need an
  admin key — FR-013); `/api/oauth/usage` (undocumented, forbidden by the issue and FR-012);
  Claude Code's local transcripts (token counts, not plan percent or reset — fails FR-004,
  SC-002); scraping `/usage` output from the PTY (undocumented, brittle; 026 research R4 measured
  PTY scraping as unreliable).

## R2. How the status line reaches the application — the existing `--settings` file

- **Decision**: add a `statusLine` block to the per-session `--settings` file the hook receiver
  already writes (`crates/micold-daemon/src/hooks.rs`, `prepare_settings`, `settings_json`), only
  while **Show Claude plan usage** is on. The block runs the relay (R3):
  `{"type":"command","command":"<quoted daemon exe> status-line <quoted relay file>"}`, plus the
  user's own `padding` and `refreshInterval` when they set them (R4).
- **Rationale**: no user configuration is modified (the 026 rule); `--settings` already reaches
  every Claude session the application starts, including respawns. Command-line settings outrank
  user and project settings, so the block takes effect without editing the user's files; managed
  settings still outrank it (R8).
- **Alternatives rejected**: writing `statusLine` into `~/.claude/settings.json` (modifies user
  configuration, affects sessions outside the app); a second `--settings` flag (Claude Code takes
  one); an `http` status line type (none exists — only `command`).

## R3. The relay — `micold-daemon status-line <relay-file>`

- **Decision**: the status-line command is the daemon's own executable with a `status-line`
  argument, dispatched in `main.rs` before the runtime, the singleton or the log start. It:
  1. reads stdin to EOF; over 1 MiB no reading is taken, and the user's command still receives
     the whole input;
  2. reads the relay file (owner-only, beside the settings file): the receiver URL
     `http://127.0.0.1:<port>/status/<session-uuid>`, the bearer token, and the user's status line
     command if any (R4);
  3. in parallel: parses `rate_limits` with `plan_usage::status_line::extract` and, when it holds
     at least one window, POSTs only that object (never the rest of stdin) to the URL with the
     token, 500 ms total timeout; and runs the user's command (R4) with the same stdin bytes,
     copying its stdout to ours;
  4. exits with the user command's exit status, or 0 when there is none, after the POST has
     finished or timed out. Every failure is silent on stdout and stderr (FR-015, FR-016, FR-020).
- **Rationale**: the daemon executable exists wherever the daemon runs — the host install, the
  macOS bundle, the Windows install and the sandbox image — so no new binary needs packaging
  (028, 030, 027 untouched). The token stays out of the command line (other local users can read
  `ps`); the relay file is written with `micold_core::owner_only` like the 034 binding file.
  Posting only `rate_limits` keeps transcript paths and cwd out of the daemon (FR-014).
- **Windows**: the release daemon is a GUI-subsystem executable. Claude Code spawns the command
  through Git Bash or PowerShell with piped stdio, and a GUI-subsystem child inherits the pipe
  handles it is given, so stdin/stdout work; the command quotes the executable path with forward
  slashes (the statusline doc's Git Bash rule). Quickstart §B4 confirms on Windows.
- **Alternatives rejected**: a new `micold-statusline` binary (packaging in three installers and
  the image for one small entry point); a shell script (`jq`, `curl` not on every machine, no
  Windows parity — Principle VI); posting the full stdin (sends transcript path and cwd into the
  daemon for no use).

## R4. Keeping the user's own status line (FR-020)

- **Decision**: when the daemon prepares a session's settings with the switch on, it resolves the
  user's effective `statusLine` the way Claude Code's precedence does for non-managed sources:
  `<cwd>/.claude/settings.local.json`, then `<cwd>/.claude/settings.json`, then
  `<CLAUDE_CONFIG_DIR or ~/.claude>/settings.json`; the first file with a `statusLine` object of
  `type: "command"` wins. Only the `statusLine` key is deserialised (a serde struct with that one
  field; the rest of the document is skipped, never kept, logged or sent). Its `command` goes into
  the relay file; its `padding` and `refreshInterval` are copied into the session's `statusLine`
  block. The relay runs the command through the shell Claude Code itself would use: `sh -c` on
  Unix; on Windows Git Bash (`CLAUDE_CODE_GIT_BASH_PATH`, else `bash.exe` beside `git.exe` on
  `PATH`), else `powershell -NoProfile -Command`. The child gets `MICOLD_STATUS_RELAY=1`; a relay
  that starts with it set runs only the pass-through, so a chained command can never recurse.
- **Rationale**: `--settings` replaces the user's `statusLine` rather than merging, so without
  chaining the user loses theirs. Resolution at prepare time is one read per session start, not
  one per status-line run; a user who edits their `statusLine` key mid-session gets it in sessions
  started afterwards (editing their script's content works at once, as the statusline doc says).
  `ConfigLocations` (`crates/micold-core/src/mcp/binding.rs`) already resolves home and
  `CLAUDE_CONFIG_DIR` for 034; reuse it.
- **Credential note (FR-013)**: Claude Code's settings files are user configuration, not a
  credential store, but a user may put secrets in their `env` key. Deserialising only
  `statusLine` means no other value is held past the parse. The 034 tool server already reads
  `.claude.json` the same way. The Settings text and user guide say which files are read and that
  only `statusLine` is taken (FR-003).
- **Alternatives rejected**: resolving in the relay on every run (a file read per status-line
  update in every session); dropping the user's status line while the feature is on (violates
  FR-020); shelling out to `claude config get` (not a documented interface for `statusLine`).

## R5. Receiving readings — a second route on the hook receiver

- **Decision**: `POST /status/<session-uuid>` on the existing loopback receiver, authenticated by
  the same per-session token, body bounded at 16 KiB, never logged. A valid body becomes a
  `UsageReading` stamped with the daemon's wall clock and goes to `DaemonState::note_plan_usage`.
  Answers: 200 on a reading or an empty `rate_limits`, 400 malformed, 403 bad token, 413 too
  large, 404 other paths — the hooks contract's rules.
- **Rationale**: one listener, one token registry, one bounded-body path already reviewed
  (contracts/hooks.md of 026). The token limits a leaked credential to lying about one session's
  usage, as for activity.
- **Alternatives rejected**: a new listener (second port, second auth); the client socket
  protocol (the relay would need the client handshake and schema hash).

## R6. One reading for the account; newest wins (FR-019, Principle II)

- **Decision**: the daemon keeps one `Option<UsageReading>`, not one per session. Each accepted
  reading replaces it when its windows differ; an identical one only refreshes `obtained_at`
  (no broadcast when only the timestamp moves within 60 s, to keep ten idle sessions from
  broadcasting on every turn). The reading is in memory only (FR-014): not in the catalog, not in
  `settings.json`, not in terminal history; a daemon restart starts with none.
- **Rationale**: usage is the account's (spec Edge Cases, "Many sessions"); the source has no
  account identity (Decision 4). Per-session state would add Principle II scope for no shown value.
- **Alternatives rejected**: per-session readings with a max (the spec says newest wins, not
  highest); persisting the last reading (FR-014 and offline start would show a stale value).

## R7. Current vs stale (FR-009) — decided where it is drawn

- **Decision**: a window is current while `resets_at > now`. `UsageReading::current(now)` drops
  passed windows; the indicator shows the highest current window and hides when none remain. The
  client evaluates it at render with its wall clock, and holds a 30 s `iced::time::every`
  subscription **only while a reading exists**, so a passed reset hides the indicator within 30 s
  and an idle app with no reading has no timer (`tests/idle_subscriptions.rs`). The daemon drops a
  reading whose windows have all passed when the next one arrives or a client connects.
- **Rationale**: Claude Code itself drops a window at its `resets_at`; there is no polling to
  refresh a reading (Decision 3). A wrong local clock can only hide a window early, never show a
  reset as happened (spec Edge Cases).
- **Alternatives rejected**: an age limit (e.g. 10 minutes) — an idle user's last reading is still
  right until the reset.

## R8. Unavailability is silent (FR-015, FR-016)

- **Decision**: nothing about plan usage is logged at warning or above. The relay writes nothing
  (stderr would show in `claude --debug` only, but it stays empty anyway). The daemon logs a
  rejected `/status` request at `debug`, once per (session, reason) until that session's next
  accepted reading. Managed settings with `statusLine`, `allowManagedHooksOnly`,
  `disableAllHooks`, an untrusted folder, an API-key account and offline all simply produce no
  reading — the indicator stays hidden.
- **Alternatives rejected**: a "usage unavailable" placeholder (FR-015 says hide).

## R9. Settings — two service-owned settings, mirroring 613's threshold

- **Decision**: `plan_usage_enabled: bool` (default `true`, Decision 2, one constant
  `DEFAULT_PLAN_USAGE_ENABLED`) and `plan_usage_warn_percent: u8` (default 80, clamped 50–100 on
  read by `clamp_plan_usage_warn_percent`, refused on save by the client's field validation) in
  `micold_core::settings::Settings`, `DaemonSettings` and `ClientMsg::SettingsSet`, the same path
  as `long_task_threshold_secs` (feature 613: `settings.rs` clamp, `catalog.rs`,
  `server.rs` `SettingsSet`, `features/settings.rs` field check). Additive with serde defaults, so
  `settings_version` does not move.
- **Switch change**: the daemon rewrites the settings file of every session in the hook token
  registry (with or without the `statusLine` block) and writes or deletes its relay file; off also
  clears the stored reading and broadcasts `None` (FR-002, SC-007). Claude Code picks up a changed
  settings file in a running session when it reloads settings (Decision 5); new sessions always
  start under the current switch.
- **Placement**: Settings → Environment, a **Claude plan usage** subsection after **Desktop
  notifications** (a switch, then a threshold field, then the FR-003 statement). Environment
  already holds the other per-AI-CLI switches (default CLI, Pi activity, tool server).
- **Alternatives rejected**: client-owned settings (two writers, the 026 revert hazard noted on
  `DaemonSettings::default_ai_cli`); a new "Claude" section for two controls.

## R10. Where the indicator sits, and how it looks (FR-004, FR-006, FR-010, Principle VIII)

- **Decision**: a new shared component `UsageIndicator` in `crates/micold-client/src/ui/material/
  usage_indicator.rs`, placed in the top app bar as the first trailing action (before the project
  switcher). Normal look: `Icon::PlanUsage` (Material Symbols `data_usage`, U+E1AF) + label
  `42% · 15:30`. Warning look: `Icon::UsageWarning` (`warning`, U+E002) in the `error` role + the
  same label + the text role change; the glyph shape differs, so colour is not the only cue
  (FR-010). Details are the shared `Tooltip` (`ui/material/mod.rs`) with one line per window
  (`5-hour: 42%, resets 15:30`), a line naming the windows at or above the threshold, and
  `Updated 14:02`. Builder: `UsageIndicator::new(label, roles).warning(bool).details(lines)
  .into()`. The shipped icon font has full coverage (`assets/fonts/PROVENANCE.md`), so the two
  codepoints need no font change; `tests/icons.rs` locks them.
- **Rationale**: the app bar is the only always-visible bar (spec Assumptions). Not interactive
  beyond the tooltip, so no new press target in the bar.
- **Alternatives rejected**: a status bar at the window bottom (none exists; a new bar for one
  item); the sidebar footer (scrolls, hidden when the sidebar is collapsed).

## R11. Time formatting (FR-008)

- **Decision**: `plan_usage::format_reset(resets_at, now, offset)` in core: `HH:MM` when within 24 h,
  else `Mon HH:MM`; `offset` is a `chrono::FixedOffset` the client takes from `chrono::Local` at
  render. `chrono` is already a workspace dependency (daemon uses it); add it to `micold-core` and
  `micold-client` with the workspace's `default-features = false, features = ["clock"]`.
- **Alternatives rejected**: formatting in the daemon (its time zone may differ from the window's
  in the sandbox); a new time crate (Dependencies rule — chrono is already vetted).

## R12. Window names and which windows count

- **Decision**: `five_hour` → "5-hour", `seven_day` → "Weekly"; any other key under `rate_limits`
  holding `used_percentage` is shown with its key humanised (`seven_day_opus` → "Seven day opus"),
  so a plan that gains a window shows it (spec Edge Cases, "Plan changes"). `spend_limit` is
  excluded: it is a gateway spend limit, not plan usage (spec Out of Scope: spend reporting).
  Percentages are kept in tenths (`used_tenths: u32`) so wire types stay `Eq`; the label rounds to
  a whole percent (SC-002: within one point); the threshold compares tenths (≥ threshold × 10).
- **Alternatives rejected**: `f64` on the wire (breaks `Eq` derives on `DaemonMsg` payloads);
  only the two documented keys (a new plan window would be invisible).
