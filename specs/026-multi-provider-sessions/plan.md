# Implementation Plan: Choose which AI CLI a session runs on

**Branch**: `feat/add-support-for-other-ai-cli` | **Date**: 2026-08-16 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/026-multi-provider-sessions/spec.md`

## Summary

The seam already exists and has never been used. `micold_core::provider::AiCliProvider` was written
by feature 005 to be the one place an AI CLI's specifics live, and
`crates/micold-core/tests/ai_cli_provider_seam.rs` says in its own module docs that it is not
substitutable yet: *"Every consumer in the workspace names `ClaudeProvider` concretely."* Seven
call sites do, across all three crates. So this feature is not "add Copilot" — it is *make the seam
true, and land a second CLI as the proof*.

Five changes, in that order:

1. **Reshape the seam** so it has no Claude-shaped defaults. `transcript_dir` +
   `discover_transcript_session_ids` encode one layout — a per-cwd *directory* whose `*.jsonl`
   filenames are session ids. Copilot's is a per-cwd *index file* listing ids
   (`sidebar-sessions-state/<sha256(cwd)>.json`, research R3). Both are pure derivations from
   `(config_dir, cwd)`; only the return shape was wrong. It becomes
   `recorded_session_ids(...) -> Vec<Uuid>`, and the trait gains `id()`, `display_name()`,
   `is_available()` and `activity_source()`. See [contracts/ai-cli-provider.md](./contracts/ai-cli-provider.md).
2. **Name which CLI a session runs on.** A closed `AiCli` enum on `Session`, set at construction and
   never mutated, persisted as one `#[serde(default)]` field on `StoredSession` — no
   `schema_version` bump, by the same argument `mode` and `archived` already carry (research R8).
   `Settings` gains `default_ai_cli`; `ClientMsg::SessionCreate` gains `provider`, which *is* a wire
   change and does move the protocol hash.
3. **Implement `CopilotProvider`.** `copilot --session-id <uuid> --no-remote` for a fresh session,
   `--resume=<uuid>` for an existing one — Copilot accepts an application-chosen UUID, which is the
   single assumption the whole seam rests on and the first thing research verified (R1). Title from
   `name:` in `workspace.yaml`; activity by tailing `session-state/<uuid>/events.jsonl`. Most of its event
   types map onto the `HookKind` vocabulary the daemon's `Activity` state machine already consumes,
   so that machine is not touched — but the mapping is typed at `ActivityEvent`, because
   `session.shutdown`/`session.error` land on `Ended { reason }`, a sibling variant `HookKind` has
   no way to express. See [contracts/copilot-cli.md](./contracts/copilot-cli.md).
4. **Choose one, in one place — and put that place where every crate can reach it.** The lookup is
   `AiCli::provider(self) -> &'static dyn AiCliProvider` in `micold-core/src/provider.rs`, an
   exhaustive match, total by construction. It is *not* on `Capabilities`, which an earlier draft
   assumed: `micold-daemon` depends on `micold-client` only as a dev-dependency and `micold-core`
   cannot depend on it at all, yet `catalog.rs`, `state.rs`, `supervisor.rs` and
   `core/terminal.rs::claude_args` all need to resolve a provider from a session's `AiCli`.
   `Capabilities` stays the client's assembly point and delegates to it.
   `no_concrete_implementations.rs` is extended past the client to the daemon and `core/terminal.rs`
   — where four of the seven concrete mentions live — so a third CLI cannot be wired in by naming it
   in the supervisor (FR-022), with core's definition site as an explicitly listed exemption.
5. **Write the discovery pass FR-014 asks for**, which does not exist. This was found late and is
   recorded plainly because the earlier drafts of this plan and of `tasks.md` both assumed it was
   there to be generalised. It is not: `discover_transcript_session_ids`, `transcript_dir` and
   `is_archived` are called from **no `src/` file in the workspace** — only from
   `micold-core/src/provider.rs`, where they are defined, and from tests.
   `micold-core/tests/session_reconciliation.rs` states in its own module doc that it mirrors a
   `reconcile_sessions_from_transcripts` in the client's `main.rs`; that function is gone. So
   FR-014/FR-015 are net-new behaviour with a real gate to write, and the process that owns the pass
   is the **daemon**, in the existing `ClientMsg::AttachProject` arm, in the same `spawn_blocking`
   hop that already refreshes the project's worktrees (R15). The question looked like a trade-off
   only because this plan asserted the daemon "cannot enumerate worktrees"; it can, and has since
   US3/T053 — `State::refresh_worktrees` runs `git worktree list --porcelain` and caches the result
   the catalog snapshot reads. So the pass sits on the one process that may write `projects.json`,
   with the location list already in hand and a snapshot about to be sent.

**The launch path is the other half of change 1, and it is not a substitution.**
`micold-core::terminal::LaunchSpec` has no provider field, the function that reads it is called
`claude_args` and ignores the spec, and the daemon decides the spawn at two `LaunchSpec { … }` sites
in `state.rs` that call `hook_settings_file(id)` unconditionally and hand everything to
`spawn_claude`. So the seam does not reach the spawn at all: a struct has to gain a field and a
function has to lose its name before "take the provider from the record" means anything there. The
same discovery settles the `ActivitySource::Hooks` shape — the settings path is written by the
daemon from a port and a token chosen at runtime, so the variant carries **no payload** and only
names which mechanism applies.

**Two smaller shapes were also wrong and are corrected in the artifacts**, both found by opening the
type rather than re-reading the task: the default-CLI preference is **service-owned** (the daemon
persists its whole boot-time `Settings` struct on every set, so a client-owned field is reverted by
an unrelated change — a defect `theme` already carries), and the missing-CLI message travels as
`WireLifecycle::Failed { reason, attempts }`, because the domain `SessionLifecycle::Failed` is a
payload-free "auto-restart gave up".

**Five clarifications on 2026-08-18 settle what the earlier drafts left implicit**, and three of
them narrow the work rather than widen it. Discovery runs on **every** project open, with cost
per *location* rather than per conversation. The activity badge covers only sessions this
application supervises — a discovered one reads unknown and is never watched, which is what keeps
SC-006 true on a worktree holding hundreds. A conversation another process is already attached to is
resumed like any other and reported if the CLI refuses, with **no liveness detection of our own**,
because neither CLI offers a marker to test against. The two remaining answers are naming: rows and
the terminal bar carry the **command name** (`claude`, `copilot`) while menus and failure messages
carry the human-readable one, and the large-history claim becomes SC-009, structural in the same way
SC-006 is.

**Two set-wide provider decisions are the risk in change 1**, not the trait edit. Both judge a *set*
of sessions with one hoisted provider, and both are reached by paths that name nothing concrete, so
neither shows up in the seam audit: `micold-daemon/src/state.rs`'s `prune_empty_sessions` /
`present_interrupted_resumable_at_startup`, and the client's own `main.rs::prune_empty_sessions`
called from `shell/startup.rs`. Left as they are, every Copilot session looks empty to
`ClaudeProvider` and is archived or dropped without a word. They are the only silent-data-loss paths
in this feature and each has a dedicated failing test before its fix.

**A third structural gap turned up the same way, and it is the availability set.** FR-006 says an
uninstalled CLI is never offered; three places have to know which are installed — the Settings
select, the per-session override, and the reducer that resolves a start. None of them can reach
`Capabilities`. `crates/micold-client/src/features/` imports nothing from `shell::` and should not
start; `ui/settings_form.rs`'s view is dispatched through `crate::ui::DialogView`, a single
fn-pointer type shared by all nine registered surfaces, so it sees `&State` and nothing else; and
the sidebar's `row_actions_cluster` takes four narrow arguments. So the set is carried as **state**
— one field on `State`, filled at the I/O boundary from `Capabilities::available_providers()` and
refreshed when the choice is offered. Not per frame: a `PATH` probe per render is exactly the
scheduled work SC-006 forbids, and research R11's rule is "never *persisted*", which an in-memory
snapshot does not break.

**Reshaping the trait to have no defaults has a cost, and it moves `CopilotProvider` earlier than
its user story.** Twelve required methods means the type must have all twelve the moment anything
resolves to it — and with the lookup an exhaustive match, that is the moment the seam reshape
compiles, in Phase 2, not US1. Its discovery, title and activity bodies belong to US2 and US3, so
what lands foundationally is six deliberately conservative ones (empty, `false`, `None`, `Ok(())`,
`ActivitySource::None`) that later stories replace, plus `is_available`, which is real from the
start because the split affordance branches on it. `CopilotProvider` therefore moves into Phase 2
with the three tests that gate it — launch args, the FR-011 no-config-writes assertion, and
`config_dir` — keeping their IDs, so Phase 2 runs four of them out of numeric order. The phase now
ends with both provider types present and only Claude's behaviour reachable from the UI; US1 is what
connects the second one to a user.

**One new dependency, and three avoided.** SHA-256 for the Copilot index comes from the workspace's
own `protocol/hashing.rs`, deliberately dependency-free; the one YAML key ever read gets a
purpose-built scalar reader rather than a YAML crate; Copilot's SQLite store is rejected outright —
it is *less* complete than the filesystem (142 rows against 253 session directories on the
development machine) and would put a database in a workspace that has none.

The one addition is a **cross-platform filesystem-watch crate** (`notify`). FR-019 as clarified on
2026-08-16 forbids any polling timer, and this workspace has no watch facility of any kind — so the
choice was one vetted crate, three hand-written platform backends, or a poll. The crate wins on
Principle VI (one abstraction rather than three code paths on the least-exercised platform) and on
the Dependencies constraint, which asks that additions be justified rather than avoided. This
supersedes the "no new dependency" claim the plan carried before that clarification.

## Technical Context

**Language/Version**: Rust, stable (pinned by `rust-toolchain.toml`)

**Primary Dependencies**: One new — a cross-platform filesystem-watch crate (`notify`), required by
FR-019's no-polling rule and vetted against the Dependencies constraint. Otherwise existing:
`serde` for the added fields, `uuid` for the app-owned session id.

**Storage**: Three local locations, all already in use. The application's own per-project state file
(`StoredSession`, one added defaulted field) and settings file (one added defaulted preference);
Copilot's own store under `~/.copilot` or `$COPILOT_HOME`, which the application reads best-effort
and writes exactly one byte-less sentinel into (`session-state/<uuid>/micold.archived`), mirroring
what `ClaudeProvider` already does with `<uuid>.archived`.

**Testing**: `mise run test` (whole workspace, matching CI); `mise run test-core` while iterating on
the provider, store and settings, which is where most of this lands. Every path derivation is pure,
so the Copilot provider is fully unit-testable **without `copilot` installed** — the property
`ClaudeProvider` has today and the one that keeps CI green on all three platforms. The gate table is
[quickstart.md](./quickstart.md) §A; §B is the manual pass that needs the real CLI.

**Target Platform**: Linux, macOS, Windows desktop (CI covers all three). Copilot CLI 1.0.62 was
verified on Linux only; its Windows base directory is unverified (research R2) and is the one
platform risk, recorded below.

**Project Type**: Desktop application — existing three-crate workspace, no new crate.

**Performance Goals**: No regression, and one explicit bound made structural by the 2026-08-16
clarification: activity observation is **purely event-driven in this application's own
scheduling** — no polling timer of our own, no periodic wakeup, no work scheduled per idle session,
and no debouncer in the path (FR-019, SC-006). Where a platform or filesystem offers no native
change notification, the watch crate's own internal fallback is explicitly out of that scope — the
rule is about what we schedule, not what the crate does underneath. `events.jsonl` is tailed for running
sessions only, woken by the platform's change notification and nothing else, which is the same
standard feature 010's hook receiver meets. The test asserts the absence of a timer rather than
measuring one.

**Constraints**: Everything the application reads in another vendor's store is *their* internal
format and may change without notice, so every read is best-effort: a missing, unreadable or
unparseable file degrades exactly one capability (no title, no discovery, `Unknown` activity) and
never fails a session or a project open. Unknown fields and unknown event types are ignored, never
rejected. Against that, one deliberate exception: an **unknown provider string in our own store is a
load error**, not a silent fallback — quietly loading a future `Codex` session as Claude Code would
start the wrong CLI in the user's worktree.

**Scale/Scope**: One new value type, one field on `Session`, one on `StoredSession`, one setting, one
wire field, one new provider implementation, and a registry where a single `Arc` used to be. Roughly
a dozen files across the three crates, two user-guide pages, and one guard test whose reach grows.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Almost all of this is render-free and directly
  testable: the reshaped seam, both providers' path arithmetic, the index and title parsers, the
  store and settings round-trips, the Copilot event→signal mapping. Because the derivations are pure
  and the parsers take bytes, the tests run against fixtures with **no CLI installed** — quickstart
  §A is the plan, written before the code. The glue covered by the exception is the launch call and
  the Settings/override widgets in `src/ui/`; every decision they invoke (which provider, is it
  available, what does the row say) lands in tested logic first.
- [x] **II. Multi-Session Support**: PASS, and the feature strengthens it. `provider` is per-session
  state fixed at construction with no setter and no message that changes it, which is what makes
  FR-005 ("changing the default affects nothing already open") true by shape rather than by
  discipline. Two sessions in the same worktree may run different CLIs; their conversation records,
  titles, activity sources and archived markers live in different stores, so there is nowhere for
  one to leak into the other.
- [x] **III. Worktree Integration**: PASS. No new session location. A session's cwd is its worktree
  or the sanctioned Default project root exactly as before; the provider only decides *which binary*
  runs there. Copilot is launched with cwd set to the worktree, which is also what scopes its own
  per-cwd index — so discovery stays worktree-aware for the second CLI on the same terms as the
  first.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS, with one thing worth stating plainly.
  Everything *this application* reads and writes is on the local filesystem and works offline; a
  provider being unreachable degrades to `Unknown`, never to a blocked launch. The CLIs themselves
  talk to their vendors — that is the user's own installed tool, the same as Claude Code has been
  since feature 005, and no different in kind. Where this feature *can* act, it does:
  `--no-remote` is passed on every Copilot launch, because without it Copilot logs `Remote session
  access enabled` and a session this application spawned on the user's behalf becomes remotely
  steerable. That is precisely the "nothing leaves the device without explicit opt-in" clause, and
  it is a per-launch flag, so no user configuration is modified (FR-011).
- [x] **V. Rust + iced Stack**: PASS. `AiCli` is a closed enum, not a string: a session cannot name
  a provider that does not exist, and the registry lookup is total. Availability is *computed*,
  never stored, so "installed" cannot go stale in a file. No new GUI framework; one new runtime
  dependency (the watch crate), justified above and vetted against the Dependencies constraint.
- [x] **VI. Cross-Platform Parity**: PASS, with a recorded risk. Every path derivation is
  `PathBuf::join` on components — no separator assumptions, no `cfg(target_os)` in the providers.
  The environment override (`COPILOT_HOME`) and home-directory fallback follow the convention
  `ClaudeProvider` already uses for `CLAUDE_CONFIG_DIR`, so both providers behave the same way on
  all three platforms. **The risk**: Copilot's base directory on Windows was not verified (research
  R2) — it may not be `%USERPROFILE%\.copilot`. Mitigation: it is one function, `config_dir()`,
  behind the seam; CI stays green regardless because no test requires the CLI, and any test that
  would must skip when it is absent (FR-006's availability check is the same predicate). The watch
  crate is what keeps FR-019 from becoming three platform code paths, which is this principle's
  "isolated behind clear abstractions" clause applied to the one new mechanism.
- [x] **VII. Documentation First-Class**: PASS. Two pages, in the same change:
  `docs/user-guide/settings.md` gains the **Default AI CLI** preference (including that a default
  naming an uninstalled CLI is kept, not silently repaired), and
  `docs/user-guide/worktrees-and-sessions.md` gains choosing a CLI per session, what the sidebar
  shows, and that the choice survives a restart with the conversation.
- [x] **VIII. Reusable UI Component Foundation**: PASS. The Settings preference reuses feature 022's
  `ui/material/select.rs` — a shared component with the mandated builder API — rather than a new
  widget. The per-session override is a split affordance at session creation (Clarifications
  2026-08-16): the existing `icon_button.rs` starts the default in one press, and an adjacent
  secondary control opens a list `ui/material/picker.rs` and `ui/material/menu.rs` already provide.
  If the sidebar's per-row CLI
  identification (FR-016) needs anything beyond the existing `icon_label`/`tag`, it goes into
  `ui/material/` as a shared component with a chainable builder terminating in `.into()`, is
  theme-aware, and is added to the component showcase — not forked into `ui/sidebar.rs`.

Re-checked after Phase 1 design: unchanged, all PASS. Phase 0 in fact *removed* pressure on three
principles — finding `events.jsonl` (R5) meant the `Activity` state machine is reused byte-for-byte
instead of gaining a second polling path; finding the per-cwd index (R3) meant discovery is a pure
derivation like Claude's rather than a scan; and finding `protocol/hashing.rs` meant no new crate.
The one item Phase 1 *added* is the FR-022 gap: the existing guard reaches only the client, and
satisfying the requirement means extending it to the daemon and `core/terminal.rs`.

## Project Structure

### Documentation (this feature)

```text
specs/026-multi-provider-sessions/
├── plan.md              # This file
├── research.md          # Phase 0 output — R1–R13, verified against Copilot CLI 1.0.62
├── data-model.md        # Phase 1 output — AiCli, Session.provider, StoredSession, Settings, wire
├── quickstart.md        # Phase 1 output — §A automated gate table, §B manual pass B1–B8
├── contracts/
│   ├── ai-cli-provider.md   # The reshaped seam: what every provider must answer
│   └── copilot-cli.md       # The Copilot profile — launch, storage, events, marker
├── checklists/
│   └── requirements.md      # Spec quality checklist (from /speckit-specify)
└── tasks.md                 # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── provider.rs                     # the seam reshaped: id/display_name/is_available/
│                                   # recorded_session_ids/activity_source, no layout defaults;
│                                   # ClaudeProvider adapted; CopilotProvider added
├── session.rs                      # +AiCli, +Session::provider (construction-time, no setter)
├── settings.rs                     # +default_ai_cli (serde default → ClaudeCode)
├── store.rs                        # +StoredSession::provider (serde default, no schema bump)
├── terminal.rs                     # launch args take the session's provider, not ClaudeProvider
└── protocol/
    ├── messages.rs                 # SessionCreate gains provider; snapshots carry it outward
    └── version.rs                  # protocol version bump — the hash moves, once

crates/micold-daemon/src/
├── activity.rs                     # unchanged state machine; a second event source feeds it
├── catalog.rs                      # provider comes from the session record
├── state.rs                        # ditto
└── supervisor.rs                   # ditto; tail events.jsonl where ActivitySource::EventLog

crates/micold-client/src/
├── shell/capabilities.rs           # delegates to AiCli::provider; available_providers()
│                                   # names no concrete type any more
├── features/session.rs             # resolve default-or-override before SessionCreate
├── features/settings.rs            # the Default AI CLI preference
├── ui/settings_form.rs             # reuses ui/material/select.rs (feature 022)
├── ui/sidebar.rs                   # per-row CLI text label (FR-016) + the split start affordance
└── ui/terminal.rs                  # the AI-CLI mode toggle names the CLI (FR-016a)

crates/micold-core/tests/           # provider paths & parsers (no CLI needed), seam substitutability,
                                    # store/settings round-trip, schema hash, reconciliation
crates/micold-daemon/tests/         # copilot event → signal mapping against a fixture events.jsonl
crates/micold-client/tests/         # no_concrete_implementations extended to daemon + core/terminal
docs/user-guide/settings.md
docs/user-guide/worktrees-and-sessions.md
```

**Structure Decision**: The existing three-crate workspace, unchanged in shape — no new crate for
the second provider, because putting it anywhere but beside `ClaudeProvider` in `micold-core` would
concede that the seam does not hold. The one structural decision is **where the registry lives**:
`AiCli::provider` in `micold-core/src/provider.rs`, an exhaustive match, rather than a map on the
client's `Capabilities`. That is forced rather than preferred — the daemon depends on the client
only as a dev-dependency and core not at all, while `catalog.rs`, `state.rs`, `supervisor.rs` and
`core/terminal.rs` all need to resolve a provider from a session's `AiCli`. Core's definition site
therefore becomes the sole place a concrete implementation is named, which is the property
`no_concrete_implementations.rs` enforces (now with that site as an explicit exemption) and the
property that makes a third CLI a one-file change. Everything downstream — daemon and terminal alike
— takes the provider from the session record rather than reaching for a type.

## Complexity Tracking

> No constitution violations. The table records what was rejected, because in each case the obvious
> move was the worse one.

| Considered | Why it looked necessary | Why it was not taken |
|-----------|------------------------|---------------------|
| Read Copilot's `session-store.db` (SQLite) for discovery and titles | It is the "real" store, and it holds cwd and title in one queryable place | It is *less* complete than the filesystem — 142 rows against 253 session directories — and adding `rusqlite` puts a database dependency in a workspace with none, to read data that is already on disk as plain files. (research R3) |
| Add a YAML crate to read `workspace.yaml` | It is YAML; parse it as YAML | Exactly one scalar key is ever read (`name:`), and `cwd` is already known from the index. A purpose-built reader for the plain and quoted forms is a dozen lines against a new dependency tree. (R4) |
| Poll Copilot's store for busy/idle, as the clarification session chose | It was the only mechanism known when the question was asked | `events.jsonl` exists: an append-only per-session log whose event types map 1:1 onto the `HookKind` vocabulary the `Activity` state machine already consumes. Strictly better than what was chosen — structured events instead of inferred state, and the state machine is not touched. The first hypothesis (an empty `assistant_response` means "in flight") was disproved by the data: 139 of 155 such rows are mid-history. (R5) |
| Drive Copilot over `--acp` (Agent Client Protocol) | The richest signal available, by a distance | Wrong shape for this feature — it replaces the PTY session model rather than observing it, so it would rewrite what feature 005 built to gain a badge. Worth its own feature later. (R5) |
| Bump `schema_version` for the new `StoredSession` field | It is a persisted-format change | Additive and `#[serde(default)]`, exactly as `mode` and `archived` were before it: an older file loads with `ClaudeCode`, which is the correct answer for every session written before this feature. A bump would cost a migration path to express a default. (R8, FR-013) |
| `#[serde(other)]` on the persisted provider so unknown values fall back | Forward compatibility with a future third CLI | Falling back means starting the *wrong CLI* in the user's worktree. Declining to load is the safer failure, and the store's existing malformed-file recovery already covers it. (data-model round-trip table) |
| Poll the event log on a timer instead of adding a watch crate | It keeps the "no new dependency" claim the plan opened with, and SC-005's original 5-second budget permitted it | FR-019 as clarified on 2026-08-16 forbids a polling timer, and the reason is that "cheap enough" is not testable — the previous wording was an adjective, not a gate. Hand-writing three platform backends was the other way to avoid the crate, and it puts the most platform-specific code this codebase owns on its least-exercised path, against Principle VI. |
| Keep `no_concrete_implementations.rs` scoped to the client | It passes today, and it will pass with `CopilotProvider` too | It passes because it does not look where the problem is: four of the seven concrete mentions are in the daemon and `core/terminal.rs`. A guard that would not catch a `CodexProvider` named in the supervisor is not the guard FR-022 asks for. |
