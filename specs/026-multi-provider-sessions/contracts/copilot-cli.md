# Contract: `copilot` CLI Invocation

**Feature**: 026-multi-provider-sessions | the **GitHub Copilot CLI provider profile** of the AI CLI
provider seam (FR-020, `contracts/ai-cli-provider.md`).

The counterpart to `specs/005-worktree-session-terminal/contracts/claude-cli.md`. Each section below
is one seam capability. Nothing outside the seam may depend on anything in this file.

External dependency contract. Verified against **GitHub Copilot CLI 1.0.62** on Linux (research
R1–R6, R12), and re-verified unchanged against **1.0.80** when the fixture corpus was captured
(T001) — index filename, `schemaVersion`, `sessionIds`, `workspace.yaml`'s `name:` key and
`events.jsonl` all still as written here.

Every on-disk format here is *internal to Copilot* and may change without notice: all
reads are best-effort and a parse failure degrades the affected capability rather than failing the
session.

## Preconditions

- `copilot` is on `PATH`. Absence is surfaced when starting a session: the session enters `Failed`
  with a message naming the CLI (FR-010), and the application does not crash.
- No minimum version is gated on. `--session-id`, `--resume` and the on-disk layout below were all
  present in 1.0.62; if a future version drops `--session-id`, the fallback is the same shape as
  `claude`'s — launch without an id and read the new id from the per-cwd index below.

## Base directory

```
$COPILOT_HOME  if set and non-empty      # verified: relocates the entire store, no leakage
~/.copilot     otherwise                 # home-relative on every platform, Windows included
```

An empty `COPILOT_HOME` is treated as absent, and an unresolvable home directory yields "uncertain"
rather than "absent" — both matching `ClaudeProvider`'s existing convention.

**Windows is `%USERPROFILE%\.copilot`**, not `%APPDATA%` or `%LOCALAPPDATA%` (T081, against CLI
1.0.80). Copilot's own resolver is `resolveCopilotHome(configDir, $COPILOT_HOME, homedir())` — it is
handed the home directory and neither the platform nor `%LOCALAPPDATA%`, while `copilotCacheHome`
immediately beside it is handed all three, so the two are deliberately different in this respect.
Every `.copilot` literal the CLI ships joins it to `homedir()` unconditionally, and one of them is a
migration that *moves* state out of `$XDG_STATE_HOME`/`$XDG_CONFIG_HOME` into `homedir()/.copilot` —
the base directory is the destination those variables are being retired in favour of, not one of the
things they can override.

The one place this can diverge from what the application computes: Node's `os.homedir()` prefers
`%USERPROFILE%` when it is defined, while `directories::UserDirs` asks Windows for
`FOLDERID_Profile` and ignores the variable. They name the same directory unless something has
redefined `%USERPROFILE%` for the process tree — and if that happens, `ClaudeProvider` is wrong in
exactly the same way, since it resolves its own base the same way. `$COPILOT_HOME` overrides both.

## Launch — fresh session

```
cwd = <worktree path>                    # scopes the session to the worktree
env  TERM=xterm-256color
copilot --session-id <uuid> --no-remote  # app-generated UUID v4 → app owns the id
```

- `--session-id` with an id Copilot has never seen creates that session under that exact id
  (verified: `Registering foreground session: <uuid>`, and `session-state/<uuid>/` appears).
- `--no-remote` is deliberate, not incidental. Without it Copilot logs `Remote session access
  enabled` and the session is remotely steerable; a session this application spawned on the user's
  behalf must not be, absent an explicit opt-in (Principle IV). It is a per-launch flag, so no user
  configuration is modified (FR-011).
- `--allow-all-tools` is **not** passed. The session is interactive; the user answers Copilot's
  permission prompts in its own terminal, exactly as they answer `claude`'s.

## Launch — resume

```
cwd = <worktree path>
copilot --resume=<uuid> --no-remote
```

- `--continue` (most recent in cwd) and bare `--resume` (interactive picker) are **not** used — the
  application always targets a specific id, for the same reason feature 005 rejected `--continue`.
- An id Copilot no longer has must be surfaced and offered as a fresh session, not left as a blank
  terminal (spec edge case).

## Sessions recorded for a working directory (FR-014)

```
<base>/sidebar-sessions-state/<sha256_hex(cwd)>.json
```

```json
{ "schemaVersion": 1, "cwd": "<abs path>", "sessionIds": ["<uuid>", …] }
```

- `<sha256_hex(cwd)>` is the lowercase hex SHA-256 of the working-directory string as Copilot
  recorded it (verified byte-for-byte against a probe session; the recorded vector is in
  `crates/micold-core/tests/fixtures/copilot/README.md`). SHA-256 comes from the workspace's
  own dependency-free implementation (`protocol/hashing.rs`), not a new crate.
- Written at session start, before any prompt — **by the interactive TUI**. A `copilot -p …`
  non-interactive run writes `session-state/<uuid>/` and its `events.jsonl` but no index entry at
  all (re-verified on 1.0.80, T001). Immaterial to this application, which always spawns the
  interactive CLI in a PTY, but it is why a probe script written around `-p` concludes the file
  does not exist.
- **Pure path derivation, no I/O** — so it is unit-testable, like `claude`'s `<slug(cwd)>`.
- Recovery path when the file is missing or unparseable: scan `session-state/*/workspace.yaml` and
  filter on its `cwd` key. Correct but O(all sessions ever); never the primary route.
- `schemaVersion` is Copilot's, not ours. A value other than `1` is treated as unreadable —
  contributing no sessions — rather than parsed hopefully.

## Recorded-conversation detection

```
<base>/session-state/<uuid>/events.jsonl        exists ⇒ a conversation was recorded
```

Created lazily on the **first user message**, not at session start (verified: probe sessions that
received no prompt have no such file). A session directory without it is a session that was opened
and never used.

## Session label extraction (best-effort, FR-017)

```
<base>/session-state/<uuid>/workspace.yaml   →   name: <title>
```

- The `name` key is absent until Copilot has summarised the conversation, then present and updated
  as the conversation grows — the same lifecycle as `claude`'s `ai-title`, so the same
  `SessionLabel::Pending → Named` transition applies, re-read opportunistically.
- Only this one key is ever read from this file: `cwd` is already known from the index above, and
  `git_root`/`repository`/`branch` (present only inside a repository) are not used. A purpose-built
  reader for one scalar — plain and quoted forms — avoids adding a YAML dependency.
- A missing file, missing key, or unreadable value NEVER fails the session; the label stays
  `Pending`.

## Activity signal (FR-018)

```
<base>/session-state/<uuid>/events.jsonl        append-only JSONL
```

Each line: `{"type": "<event>", "data": {…}, "id": "<uuid>", "parentId": "<uuid>", "timestamp": "<rfc3339>"}`.

Observed by tailing the file for a **running** session and mapping to the `ActivityEvent` vocabulary
the daemon's `Activity` state machine already consumes — the state machine itself is unchanged:

| Copilot `type` | `HookKind` | Signal |
|---|---|---|
| `user.message` | `UserPromptSubmit` | `Working` |
| `assistant.turn_start` | `PreToolUse` | `Working` |
| `tool.execution_start` | `PreToolUse` | `Working` |
| `tool.execution_complete` | `PostToolUse` | *(no change — turn continues)* |
| `assistant.turn_end` | `Stop` | `AwaitingInput` |
| `permission.requested` | `Notification` | `AwaitingInput` |
| `session.shutdown`, `session.error` | *(termination)* | `Ended { reason }` |

Everything else observed (`assistant.message`, `hook.start`/`hook.end`, `session.plan_changed`,
`session.mode_changed`, `session.model_change`, `skill.invoked`, `system.message`, `session.info`,
`session.resume`, `permission.completed`, and — seen in the 1.0.80 capture — `session.start`,
`session.auto_mode_resolved`, `session.usage_checkpoint`) is ignored. **Unknown types must be ignored, not
rejected** — this log gains event types between CLI versions.

Bounds:

- No `events.jsonl` ⇒ `Unknown`. This is the honest state for a session with no conversation, and it
  is what the enum already documents.
- A dangling `assistant.turn_start` (process killed mid-turn) must not leave the badge `Working`
  forever; the daemon already knows the process is dead and that guard applies here unchanged.
- Observation must cost nothing while nothing happens (FR-019) — a running session only, and no
  work per tick beyond what the platform's change notification delivers.

## Durable close/remove suppression marker (FR-015)

```
<base>/session-state/<uuid>/micold.archived     empty sentinel, app-owned
```

- Written best-effort when the user closes or removes a session; a failure never fails the caller.
- Never read or written by `copilot` — it is our artifact living in Copilot's storage so it survives
  independently of the application's own store, exactly as `<uuid>.archived` does for `claude`.
- Checked by reconciliation: an id in the per-cwd index with a matching marker is never
  reconstructed as a session.
- Safe *inside* the session directory because discovery reads the index file, not a directory
  listing — so the marker can never be misread as a session, and the `.jsonl`-extension filter
  `claude` needs has no counterpart here.
- Not versioned: it is not part of the application's store and has no shape beyond present/absent.

## Not used

- `--acp` (Agent Client Protocol server). The best available signal, and the wrong shape for this
  feature — it would replace the PTY session model rather than observe it. A candidate for a later
  feature (research R5).
- `session-store.db` (SQLite). Holds the same `cwd` and title, and is *less* complete than the
  filesystem (142 rows against 253 session directories on the development machine). Using it would
  add a database dependency to a workspace that has none.
- `--log-dir` / `--log-level`. Debug output is connection-pool noise with no turn-level events.
- Copilot's own hook system. It exists, but needs user configuration this application may not modify
  and cannot see sessions started outside the application.

## Open, to be settled by quickstart §B

Copilot maintains a `trustedFolders` list in its `config.json`, and every worktree this application
creates is a new folder. A probe reached `cli_ready` in an untrusted directory with no evident
prompt, but the probe could not read the TUI, so this is **unverified**. Expected correct behaviour
if a prompt does appear: it is Copilot's prompt, it appears in the session's own terminal, the user
answers it there, and the sidebar must not meanwhile report the session as failed.
