# Research: Choose which AI CLI a session runs on

**Feature**: 026-multi-provider-sessions | **Date**: 2026-08-14

All Copilot findings below were obtained by inspecting **GitHub Copilot CLI 1.0.62** as installed on
the development machine, and by two throwaway launches with an application-chosen session id (one
against the real `~/.copilot`, one against a relocated store). Both probe sessions were removed
afterwards. No prompt was ever sent, so nothing here depends on model output.

Everything Copilot stores is a plain file under the user's own home directory. Nothing in this
feature needs the network, and nothing needs a database engine — see R4, which is the finding that
removed a `rusqlite` dependency from the plan.

---

## R1 — Does Copilot CLI accept an application-owned session id?

**Decision**: Yes, and the existing app-owns-the-id model carries over unchanged.

- Fresh: `copilot --session-id <uuid>`. Verified: launching with
  `--session-id 11111111-2222-4333-8444-555555555555` produced
  `[INFO] Registering foreground session: 11111111-2222-4333-8444-555555555555` and a
  `session-state/11111111-…/` directory under that exact id.
- Resume: `copilot --resume=<uuid>`. `--session-id <id>` also resumes an existing id, and
  `--continue` resumes the most recent — neither is used, for the same reason feature 005 rejected
  `claude --continue`: the application always targets a specific id.

**Rationale**: This is the single assumption the whole seam rests on (`launch_args(session_id,
mode)`). Had Copilot allocated its own ids, the seam would have needed a discover-the-id-after-launch
path and this feature would be much larger.

**Alternatives considered**: Launching without an id and reading it back from the per-cwd index
(R3) — the documented fallback if a future Copilot drops the flag, mirroring `claude-cli.md`'s own
`< 2.1.210` fallback. Not implemented.

---

## R2 — Where does Copilot keep its config, and can it be relocated for tests?

**Decision**: Base directory `~/.copilot`, overridden by the **`COPILOT_HOME`** environment
variable — the direct analogue of `CLAUDE_CONFIG_DIR`.

Verified by launching with `COPILOT_HOME` pointed at a scratch directory: the whole store
(`config.json`, `session-state/`, `session-store.db`, `logs/`, `sidebar-sessions-state/`) was
created there and **nothing leaked into `~/.copilot`**.

**Rationale**: This is what makes the provider testable the way `ClaudeProvider` already is —
integration tests point `COPILOT_HOME` at a `TempDir` and assert against real files, with no risk to
the developer's own Copilot history.

**Alternatives considered**: `COPILOT_CACHE_HOME` (cache only, not the session store) and
XDG variables (not honoured for the session store). Both rejected. As with `ClaudeProvider`, an
unresolvable base directory yields `None` — "uncertain", not "absent".

---

## R3 — How are a working directory's sessions discovered?

**Decision**: `<config_dir>/sidebar-sessions-state/<sha256(cwd)>.json` — a per-working-directory
index file, listing that directory's session ids.

```json
{
  "schemaVersion": 1,
  "cwd": "/abs/path/to/worktree",
  "sessionIds": ["22222222-3333-4444-8555-666666666666"]
}
```

Verified: the filename is the lowercase hex SHA-256 of the **cwd string exactly as Copilot recorded
it**, and the file is written at session start, before any prompt.

**Re-verified on 1.0.80** while capturing the fixture corpus (T001): filename, `schemaVersion`,
`cwd` and `sessionIds` all unchanged, with the recorded vector now pinned in
`crates/micold-core/tests/fixtures/copilot/README.md`. One qualification the original probe did not
surface, added because it briefly looked like the whole mechanism had been withdrawn: the index is
written by the **interactive TUI**, not by `copilot -p …`. A non-interactive run creates
`session-state/<uuid>/` and its `events.jsonl` and no index entry at all — so a store built entirely
by `-p` runs has no `sidebar-sessions-state/` directory. This application always spawns the
interactive CLI in a PTY, so the index is always written for the sessions it starts; the note is
here for the next person who probes with `-p` and concludes R3 was wrong.

**This corrects the premise behind FR-021.** The spec assumed Copilot has no
working-directory-partitioned storage and that discovery would need a scan of every session
directory. It does have one — it is a hashed *file* rather than a slugified *directory*, but it is
still a pure derivation from `(config_dir, cwd)`, exactly the shape `transcript_dir` already has.
The seam therefore needs a smaller change than the spec anticipated (R7), and FR-021 is satisfied by
generalising "a directory whose filenames are session ids" into "a per-cwd listing the provider
knows how to read" — not by removing the cwd parameter.

**Rationale**: One file read per project open, versus a scan of 253 session directories on this
machine (a number that only grows). It is also the same information Copilot's own session picker
uses, so it is as authoritative as anything on disk.

**Alternatives considered**:

- *Scan `session-state/*/workspace.yaml` and filter on its `cwd` field.* Correct but O(all sessions
  ever) per project open. Kept as the recovery path if an index file is missing or unparseable.
- *Query `session-store.db`.* Its `sessions` table has `id` and `cwd`, and a row is inserted at
  session start (verified). Rejected: it would add a SQLite dependency to a workspace that
  deliberately has none, and it is *less* complete than the filesystem — 253 session directories
  exist against 142 database rows, with every database row present on disk and 111 directories
  absent from it (older CLI versions predating the database).

**No new dependency**: SHA-256 already exists in the workspace, dependency-free, at
`micold-core/src/protocol/hashing.rs` (written that way so `build.rs` can `include!` it). The plan
promotes it to a reusable core primitive rather than adding `sha2`.

---

## R4 — Where does a Copilot session's title come from?

**Decision**: the `name:` key in `<config_dir>/session-state/<uuid>/workspace.yaml`.

```yaml
id: ce5df141-…
cwd: /home/jaro/workspaces/…/ci-local-staging-instance
branch: ci/local-staging-instance
name: Prepare Local Staging PRD      # ← absent until Copilot generates a summary
user_named: false
summary_count: 1
created_at: 2026-06-13T20:00:47.196Z
updated_at: 2026-06-13T20:48:30.570Z
```

The key is absent on a fresh session and appears once Copilot has summarised the conversation —
which is precisely `SessionLabel::Pending` → `SessionLabel::Named`, the behaviour `claude`'s
`ai-title` record already drives. `git_root`, `repository`, `branch` are present only when the cwd is
a repository; nothing here reads them.

**Rationale**: Same lifecycle, same fallback, same best-effort posture as the existing title reader —
so `read_title` keeps its contract (never errors, `None` until present) and only its parsing changes.

**Alternatives considered**: `sessions.summary` in the SQLite store holds the same string, but see
R3 for why the database is not used. **No YAML crate is added**: `cwd` is already available from the
R3 index, so the only key ever read out of this file is `name`, and a purpose-built reader for one
scalar (handling plain and quoted forms) is smaller and safer than a new dependency — consistent
with the workspace's own choice to hand-roll SHA-256 rather than take `sha2`.

---

## R5 — Is there a busy/idle signal for Copilot? *(the spec's one droppable requirement)*

**Decision**: Yes — `<config_dir>/session-state/<uuid>/events.jsonl`, a per-session, append-only,
structured event log. FR-018 is achievable, and better than the spec assumed.

Event types observed in a real 1507-line log, with their counts:

| Copilot event | Count | Maps to existing `HookKind` | Resulting `ActivitySignal` |
|---|---|---|---|
| `user.message` | 10 | `UserPromptSubmit` | `Working` |
| `assistant.turn_start` | 183 | `PreToolUse` | `Working` |
| `tool.execution_start` | 229 | `PreToolUse` | `Working` |
| `tool.execution_complete` | 228 | `PostToolUse` | *(no change)* |
| `assistant.turn_end` | 182 | `Stop` | `AwaitingInput` |
| `permission.requested` | 9 | `Notification` | `AwaitingInput` |
| `session.shutdown` / `session.error` | 3 | *(termination)* | `Ended { reason }` |

Every line is `{"type": …, "data": {…}, "id": …, "parentId": …, "timestamp": …}`.

**This means the activity state machine is reused unchanged.** `daemon/src/activity.rs` already
implements exactly this transition table for `claude`'s hooks; Copilot only needs a different
*source* of the same events. The seam gains "how do I observe activity for this session"; `Activity`
itself is not touched.

**Rationale**: It is pulled rather than pushed, so it lags by the observation interval — which is
what FR-018's "conservative, may lag" allows. It requires no configuration at all, which means it
also works for sessions started outside the application (FR-014), something a hook-based mechanism
could not do.

**Two properties worth stating**:

- The file is created lazily, on the **first user message** — not at session start. Verified: both
  probe sessions, which never received a prompt, have no `events.jsonl`. So "no file" means "no
  conversation yet", which serves double duty as `has_recorded_conversation` (R6) and leaves the
  badge at `Unknown` — the state the enum already documents as "no signal yet".
- `turn_start` outnumbered `turn_end` by one in the sample: a session killed mid-turn leaves a
  dangling `Working`. The daemon already knows whether the process is alive, so the guard is the one
  it already applies to `claude` — a dead process is not working.

**Alternatives considered**:

- *Copilot's own hooks.* It has them (`hook.start`/`hook.end` with `hookType: postToolUse`, 224
  occurrences). Rejected: they need user configuration this application may not modify (FR-011), and
  they cannot see sessions started outside it.
- *`--log-dir` + `--log-level debug`.* Rejected on inspection: 194 debug lines in five seconds of an
  *idle* session, almost all HTTP connection-pool churn, with no turn-level events.
- *The SQLite `turns` table.* Rejected: 139 of 155 empty-response rows are mid-history, not the
  latest turn, so "empty response = in flight" is simply false. `forge_trajectory_events` is empty
  (0 rows). See also R3 on the database generally.
- *`--acp` (Agent Client Protocol server mode).* A structured, pushed, bidirectional session stream —
  strictly the best signal available. Rejected for this feature because it replaces the terminal
  session model entirely: the application would become Copilot's UI rather than hosting its TUI in a
  PTY. Recorded as the likely shape of a future feature, not this one.

---

## R6 — Recorded-conversation detection and the durable closed marker

**Decision**:

- *Recorded conversation*: `session-state/<uuid>/events.jsonl` exists (R5).
- *Closed marker*: `session-state/<uuid>/micold.archived`, an empty app-owned sentinel inside
  Copilot's own per-session directory.

**Rationale**: Identical reasoning to bugfix BUG-003's `<uuid>.archived` for `claude` — the marker
must outlive the application's own store, so it lives in the provider's storage, addressed purely by
session id, with no shape to evolve beyond present/absent. Placing it *inside* the session directory
rather than beside it is safe here because discovery reads the R3 index, not a directory listing, so
the marker can never be mistaken for a session.

**Alternatives considered**: Removing the id from the R3 index file. Rejected — that file is
Copilot's, the application does not write to another tool's data, and Copilot would rewrite it
anyway.

---

## R7 — What has to change in the `AiCliProvider` seam?

**Decision**: Four changes, all smaller than the spec's FR-021 feared once R3 landed.

1. **`command()` returns `&'static str` today** and every other method is fine as-is. Keep.
2. **`transcript_path` / `transcript_dir` are Claude-shaped names for a Claude-shaped layout.**
   Rename to intent — "where this session's conversation is recorded" and "how to list the sessions
   recorded for this cwd" — and let `discover_transcript_session_ids` become a provider method
   rather than a default implementation that hardcodes "list a directory, parse `*.jsonl` stems".
   `ClaudeProvider` keeps today's behaviour as its own implementation; `CopilotProvider` reads the
   R3 index.
3. **Add "how do I observe activity for this session"** (R5) — a provider-supplied source of
   `ActivityEvent`s. `claude`'s implementation is the existing HTTP receiver; Copilot's is the
   `events.jsonl` tail.
4. **Add identity**: a stable, serialisable discriminant so a session record can name its provider
   (R8), plus availability (`is `command()` on `PATH`?`).

**Rationale**: The seam's own test file (`ai_cli_provider_seam.rs`) already states the position
honestly — "It is not, yet… Every consumer in the workspace names `ClaudeProvider` concretely" — and
names `shell/capabilities.rs::Capabilities` as the place a provider would be chosen. That is the
design this feature completes; it is not being invented here.

**Alternatives considered**: A parallel `AiCliProvider2` trait with a shim. Rejected — there are
seven call sites, they are all in this workspace, and a second trait would leave the guard in
`no_concrete_implementations.rs` policing the wrong one.

---

## R8 — Persisting the choice, and migrating what exists

**Decision**: One `#[serde(default)]` field on `StoredSession`, **no `schema_version` bump**.

```rust
struct StoredSession {
    id: uuid::Uuid,
    #[serde(default)] worktree_dir: Option<String>,
    #[serde(default)] title: Option<String>,
    #[serde(default)] mode: StoredTerminalMode,
    #[serde(default)] archived: bool,
    #[serde(default)] provider: StoredAiCli,   // ← new; Default = ClaudeCode
}
```

`StoredAiCli` is a serde-mapped mirror of the core discriminant, exactly as `StoredTerminalMode`
mirrors `TerminalMode` — so the persisted shape can evolve independently of the enum.

**Rationale**: An absent field deserialises to `ClaudeCode`, which is precisely FR-013: every
session written before this feature loads as a Claude Code session with no migration step and no
version bump. This is the pattern `mode` and `archived` both established in this file, and the file's
own comments state the precedent.

**Alternatives considered**: A bump to `schema_version: 2` with an explicit migration. Rejected as
strictly more code for the same result — an old reader already tolerates unknown fields, which is
the reason the previous two additions did not bump either.

---

## R9 — Where the provider is chosen, and how it reaches the daemon

**Decision**: the registry is `AiCli::provider(self) -> &'static dyn AiCliProvider` in
`micold-core/src/provider.rs`; the client sends the choice with the create request; the daemon stores
it on the session and resolves through that lookup thereafter.

- **Amended.** This note first put the registry on `Capabilities` and then said "the daemon …
  consults the registry thereafter", which cannot happen: `micold-daemon` depends on `micold-client`
  only as a **dev-dependency**, and `micold-core` — where `terminal::claude_args` needs the same
  answer — cannot depend on it at all. `micold-core` is the only crate all three see.
- An **exhaustive match**, not a `BTreeMap`. A map is partial by type while every caller wants an
  infallible answer, and a lookup that can be absent is what invites an `unwrap()` on the set-wide
  paths where a wrong answer silently archives sessions.
- `Capabilities` keeps `provider(which)` and gains `available_providers()`, both delegating, and
  `real()` stops naming provider types. `no_concrete_implementations`'s FR-018 property survives with
  its "one place" moved from the shell to core's definition site, listed as an explicit exemption.
- Protocol: `ClientMsg::SessionCreate { req, project, worktree_dir }` gains `provider`. This is a
  wire-format change, so it trips the protocol schema-hash guard and needs a protocol version bump —
  the mechanism in `protocol/version.rs` and `contracts/protocol.md` §4 handles it; the plan must not
  skip it.
- The daemon's `catalog.rs`, `supervisor.rs`, `state.rs` and `core/terminal.rs` take the provider
  from the session record instead of naming `ClaudeProvider`.

**Rationale**: The daemon owns session identity and the catalog; the client owns the user's
intent. The provider is part of the intent at creation and part of the record forever after, so it
travels the same path the location already does.

**Alternatives considered**: Deriving the provider daemon-side from the settings file. Rejected —
it would make a per-session override impossible (FR-004) and would read the user's settings from two
processes.

---

## R10 — The two UI surfaces

**Decision**:

- *Settings*: a "Default AI CLI" select in `ui/settings_form.rs`, alongside the existing theme and
  environment-include controls. Feature 022 already delivered a dedicated Select component; this
  reuses it (Principle VIII) rather than adding a bespoke control.
- *Per-session override*: the session-create affordance gains a menu of the available providers.

**Rationale**: `Settings` already carries five `#[serde(default)]` preferences and has a form that
renders them; a sixth is additive. The override belongs at the point of the action, per the
2026-08-14 clarification.

**Open for `/speckit-tasks`**: whether the override is a split button, a long-press/secondary menu,
or a menu on the existing control is a component-level decision that the showcase (feature 020) can
settle visually. It does not change any requirement.

---

## R11 — Availability, and what happens when a CLI is missing

**Decision**: Availability = `command()` resolves on `PATH`. Probed when the choice is offered
(FR-006) and again at launch (FR-010); never persisted.

**Rationale**: A persisted answer goes stale the moment the user installs or removes a CLI, and
`PATH` resolution is cheap. Feature 025's `claude-symlink-lost-on-reboot` note is the live
counter-example: a CLI that resolves in a login shell may not resolve for a daemon spawned at boot,
so the launch-time check is not redundant with the offer-time one.

A session whose provider is unavailable stays listed and correctly identified (FR-010, spec US4-3);
the existing failure path is the reporting mechanism, not a new one — but the message has exactly one
home, and it is not the one this note first named. `session::SessionLifecycle::Failed` is a unit
variant meaning "auto-restart gave up after repeated quick failures"; the reason travels as
`WireLifecycle::Failed { reason, attempts }`, whose own doc already covers a spawn that failed. A
missing binary should also not spend the crash-loop budget on the way there: three retries of a
`PATH` problem delay the one thing the user needs to read.

---

## R12 — Launch flags, and two defaults worth choosing deliberately

**Decision**: Launch as `copilot --session-id <uuid>` / `copilot --resume=<uuid>`, in the session's
cwd, with `TERM=xterm-256color`, plus **`--no-remote`**.

**Rationale for `--no-remote`**: an unqualified launch logs `[INFO] Remote session access enabled`
— by default a Copilot session is remotely steerable. Principle IV says nothing leaves the device
without the user's explicit, informed opt-in; a session this application spawned on the user's
behalf should not be remotely drivable because the user never asked. `--no-remote` is a per-launch
flag, so this respects FR-011 (no user configuration is modified).

**Not chosen**: `--allow-all-tools` / `--allow-all`. The session is interactive and the user answers
Copilot's permission prompts in its own terminal, exactly as they answer `claude`'s.

**Recorded, not resolved**: Copilot keeps a `trustedFolders` list in its `config.json` and every
worktree this application creates is a new folder. The probe reached `cli_ready` in an untrusted
directory without an evident prompt, but the probe could not read the TUI, so this is **unverified**.
The plan treats it as a quickstart §B check rather than an assumption: if a trust prompt does appear,
it appears in the session's own terminal and the user answers it there — which is correct behaviour,
but the sidebar must not meanwhile claim the session failed.

---

## R13 — Keeping the seam honest (FR-022)

**Decision**: Extend the existing guard rather than write a new one.
`micold-client/tests/no_concrete_implementations.rs` already derives the set of real implementations
from `impl <Port> for <Type>` in `micold-core` and asserts that only `shell/capabilities.rs` names
them. `CopilotProvider` is picked up by that derivation automatically.

**Gap to close**: the guard covers the *client*. The daemon (`catalog.rs`, `supervisor.rs`,
`state.rs`) and `core/terminal.rs` name `ClaudeProvider` today and are not covered by it. FR-022 is
only satisfied when the guard's reach includes them.

**Rationale**: A derived guard that already found `ClaudeProvider` when a hand-written task list
omitted it is the right mechanism; it just needs to be pointed at the other two crates.

**One exemption, stated rather than stumbled into**: R9's `AiCli::provider` lives in
`micold-core/src/provider.rs`, which is where both types are defined, so it names them by necessity.
The guard's "one place" moves from `shell/capabilities.rs` to that definition site, and the
exemption must be listed by name — one the scan happens not to reach is a hole, not a decision.

---

## R14 — The one new dependency: `notify`

**Decision**: `notify` 8.2.0, as a dependency of **`micold-daemon` only**.

FR-019 as clarified on 2026-08-16 forbids a polling timer of ours, and this workspace has no
filesystem-watch facility of any kind. That leaves three ways to satisfy it, and only one that does
not put the most platform-specific code this codebase owns on its least-exercised path:

| | |
|---|---|
| One vetted crate | Chosen. One abstraction over inotify / FSEvents / ReadDirectoryChangesW. |
| Three hand-written platform backends | Rejected on Principle VI — three code paths, two of which CI exercises rarely and neither maintainer runs daily. |
| A polling timer | Forbidden by FR-019, and forbidden for a reason: "cheap enough" is an adjective, not a gate. |

**The vetting the Dependencies constraint asks for:**

- **Maintenance health**: `notify-rs/notify`, the de-facto standard for this job in the Rust
  ecosystem — ~141M all-time downloads, 8.2.0 published 2025-08-03, a 9.0 release candidate in
  progress. It is what `cargo-watch`, `mdbook` and `rust-analyzer` use, so its platform backends are
  exercised far harder than anything this repository would write.
- **License**: CC0-1.0 — a public-domain dedication, compatible with this project's Apache-2.0 and
  imposing no notice obligation.
- **MSRV**: 1.77, comfortably under this workspace's 1.97 pin.
- **Reach**: declared by `micold-daemon` alone. `micold-core` stays render-free *and* watch-free —
  the seam only *names* where the events are (`ActivitySource::EventLog { path }`); the daemon is
  what opens anything. So a third provider needs no new dependency, and the core's dependency list
  is unchanged.

**Two things this decision explicitly does not buy**, both recorded because they are the obvious
next assumptions:

1. **It does not make observation free.** A watch is opened only for a session the daemon is
   *supervising*; a session merely discovered under FR-014 gets none, however many a project holds
   (SC-006, T056a).
2. **It does not import the crate's debouncer.** `notify-debouncer-{mini,full}` are separate crates
   and are deliberately not taken: a debouncer is a timer, and adopting one would reintroduce
   exactly what FR-019 forbids, wearing someone else's name. The coalescing latency is capped at
   250 ms at the watcher itself (T064) so SC-005's one-second bound holds on macOS, where FSEvents'
   default latency is otherwise high enough to breach it.

**What FR-019 does *not* cover**: the crate's own internal fallback on a filesystem with no native
change notification. The rule is about what *this application* schedules, not what a platform
backend does underneath — stated here because it is the first thing a reader of T060 will ask.

---

## R15 — Which process owns the FR-014 discovery pass

**Decision**: the **daemon**, inside the existing `ClientMsg::AttachProject` arm
(`micold-daemon/src/server.rs:378-397`), in the same `spawn_blocking` hop that already refreshes the
project's worktrees. No new RPC, no protocol change, no client round trip.

**The question was framed on a false premise.** This plan and `tasks.md` both said the daemon
"cannot enumerate worktrees". It can, and already does: `State::refresh_worktrees`
(`micold-daemon/src/state.rs:587`) runs `worktree::discover(&GitCli::new(), &repo)` — a `git
worktree list --porcelain` subprocess plus a directory listing — and caches the result where the
catalog snapshot reads it. Feature US3/T053 gave it that capability. So the trilemma the task posed
(client sends the location list / daemon scans git itself / a new RPC) was already settled by an
earlier feature in favour of the second, and there is no trade-off left to make.

**Why attach is the hook.** `AttachProject` *is* "project open and reopen": it is per-client and
exclusive, and it already runs, in order — attach, prune this project's empty sessions
(`prune_empty_off_runtime`, itself a provider-consulting sweep), send `Attached { sessions }`, then
`refresh_worktrees_and_send`. Discovery is a fourth step between the enumeration and the snapshot,
which is precisely where the location list is in hand and the catalog is about to go out anyway. It
also puts the pass on the one process permitted to write `projects.json` — which is what T050
already assumed without being able to say why it was available.

**Cost, against FR-014's proportionality rule.** Per location, per provider: one
`recorded_session_ids` — a single index read for Copilot, a single directory listing for Claude. Two
providers × N locations, independent of how many conversations each store holds. The set difference
against the catalog's known ids for that location is in memory.

**The one place that rule can be broken, and the ordering that keeps it.** Deciding whether a
*discovered* id is archived is a per-id filesystem check (`session-state/<uuid>/micold.archived`),
which is proportional to conversations rather than locations. So the order is load-bearing: subtract
the catalog's own ids **first** — its in-memory `archived` flag already covers everything ever closed
through the application — and stat the provider marker only for ids that remain genuinely unknown.
In steady state that set is empty. The marker is the recovery path for a lost or corrupt
`projects.json`, where paying once per conversation is the right price for not resurrecting closed
sessions.

**Idempotence.** A discovered session's `SessionId` **is** the CLI's conversation uuid, so the next
open finds it already in the catalog and adds nothing. Minting a fresh id per open would grow
`projects.json` without bound and break `SetViewedSession`, which addresses sessions by id.

**Per-provider isolation.** Each provider's `config_dir()` resolves independently; one returning
`None` skips that provider's contribution and leaves the other's intact — the same rule T015a
applies to the prune, for the same reason.

**Rejected — the client sends a location list.** It adds a round trip per open for information the
daemon already holds, and it puts the decision to create a session record on the side of the socket
that must not write the store: the client would end up asking the daemon to write what the daemon
could have discovered itself.

**Follow-on**: T050 writes the pass as a new function in `micold-daemon`, called from the attach arm
off the runtime; T042a gates it from `crates/micold-daemon/tests/`, driving that function against a
fixture store rather than mirroring it.

---

## Cross-platform (Principle VI)

| | Linux | macOS | Windows |
|---|---|---|---|
| `copilot` on `PATH` | verified here | same mechanism | same mechanism, `.exe`/`.cmd` resolution |
| `COPILOT_HOME` / `~/.copilot` | verified | expected identical (home-relative) | verified (T081) — `%USERPROFILE%\.copilot` |

The path derivations are all `PathBuf::join` on a base directory, so they are platform-neutral by
construction. CI covers all three and the provider's path arithmetic is pure and unit-testable
without the CLI installed — which is how `ClaudeProvider` is already tested. Tests that require the
CLI itself must skip when it is absent rather than fail, as the existing suite does.

**The Windows row was closed in T081** without a Windows machine, by reading the CLI's own shipped
code: `resolveCopilotHome` is handed the home directory and neither the platform nor
`%LOCALAPPDATA%`, where `copilotCacheHome` beside it is handed all three; every `.copilot` literal
joins it to `homedir()` with no branch; and Node's `os.homedir()` is `%USERPROFILE%` on Windows.
`contracts/copilot-cli.md` carries the detail, including the one way the application's own
`directories`-based home can disagree with the CLI's.
