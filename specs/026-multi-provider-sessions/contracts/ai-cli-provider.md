# Contract: the AI CLI provider seam

**Feature**: 026-multi-provider-sessions | supersedes the single-implementation shape of
`micold-core/src/provider.rs` (FR-020, FR-021, FR-022).

This is the boundary FR-020 promises: every CLI-specific detail is reached through it, and adding a
third CLI means adding one implementation and touching nothing else. Two implementations exist after
this feature — `ClaudeProvider` (profile:
`specs/005-worktree-session-terminal/contracts/claude-cli.md`) and `CopilotProvider` (profile:
`copilot-cli.md`).

## What changes, and why

The trait today is honest about its own state — `tests/ai_cli_provider_seam.rs` says so in its own
module docs: *"It is not, yet. Every consumer in the workspace names `ClaudeProvider`
concretely."* Two things follow.

**1. The layout assumption comes out.** `transcript_path` / `transcript_dir` /
`discover_transcript_session_ids` encode one specific shape: a per-cwd *directory* whose `*.jsonl`
filenames are session ids. Copilot's is a per-cwd *index file* listing ids, with each conversation
in its own directory. Both are pure derivations from `(config_dir, cwd)` — the parameters are right,
the return shape is not — so the fix is to move the listing behaviour into the implementations
rather than provide it as a default that hardcodes `claude`'s layout.

**2. The seam gains identity, availability and an activity source.** A session must be able to name
its provider on disk (`AiCli`), the application must be able to ask whether a CLI is installed, and
the busy/idle signal is now provider-specific (HTTP push for one, file tail for the other).

## The seam

```rust
pub trait AiCliProvider {
    // --- identity & availability (new) ---

    /// The persisted, looked-up name of this provider.
    fn id(&self) -> AiCli;

    /// The user-facing name ("Claude Code", "GitHub Copilot"). Used where the application
    /// **offers a choice or names a failure** — the Settings default, the per-session override
    /// list, the missing-CLI message. Those are menus and sentences.
    fn display_name(&self) -> &'static str;

    /// The executable to spawn, resolved on `PATH` — and also the string the sidebar row and the
    /// terminal bar carry as their label (`claude`, `copilot`), per FR-016/FR-016a as clarified
    /// 2026-08-18. Two registers, one provider: a label in a width budget is not a menu entry, and
    /// letting either leak into the other is the likeliest drift in this seam.
    fn command(&self) -> &'static str;

    /// Whether `command()` resolves on `PATH` right now. Never cached, never persisted (R11).
    fn is_available(&self) -> bool;

    // --- launching (unchanged) ---

    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String>;

    /// The provider's base config directory, or `None` when it cannot be determined —
    /// "uncertain", not "absent".
    fn config_dir(&self) -> Option<PathBuf>;

    // --- conversation storage (reshaped) ---

    /// Every session id this provider has recorded for `cwd`. Best-effort: an unreadable or
    /// missing source contributes nothing, never an error, so discovery never fails a project
    /// open. Replaces the `transcript_dir` + listing default.
    fn recorded_session_ids(&self, config_dir: &Path, cwd: &Path) -> Vec<Uuid>;

    /// Whether this provider has recorded a conversation for this session.
    fn has_recorded_conversation(&self, config_dir: &Path, cwd: &Path, id: Uuid) -> bool;

    /// The latest title this provider has recorded, read from disk. NEVER errors: any missing
    /// file / read / parse failure yields `None` (FR-017).
    fn read_title(&self, config_dir: &Path, cwd: &Path, id: Uuid) -> Option<String>;

    // --- durable close/remove suppression (unchanged in contract, per-provider in location) ---

    fn mark_archived(&self, config_dir: &Path, cwd: &Path, id: Uuid) -> io::Result<()>;
    fn is_archived(&self, config_dir: &Path, cwd: &Path, id: Uuid) -> bool;

    // --- activity (new) ---

    /// How this provider's busy/idle events reach the daemon for one session. Pure, like every
    /// other derivation here — which is why the `Hooks` arm carries nothing (see below).
    ///
    /// Answering this does **not** commit the daemon to observing anything. A source is opened only
    /// for a session the application is supervising; a session merely discovered under FR-014 is
    /// never watched, however many of them a project holds (FR-018, SC-006).
    fn activity_source(&self, config_dir: &Path, cwd: &Path, id: Uuid) -> ActivitySource;
}

/// Where a session's `ActivityEvent`s come from. The daemon's `Activity` state machine consumes
/// the same events either way and is **not** changed by this feature.
pub enum ActivitySource {
    /// The provider pushes to the loopback hook receiver, and its launch needs the per-session
    /// settings file the daemon already writes. (`claude` — feature 010's mechanism, unchanged.)
    ///
    /// **Deliberately payload-free.** An earlier draft of this contract wrote
    /// `Hooks { settings: PathBuf }`, which no provider can honour: that path is chosen and
    /// written by the daemon, not derived. `State::hook_settings_file` calls
    /// `receiver.prepare_settings(id)`, which **writes a file** containing a port picked at
    /// daemon start and a per-session bearer token (`micold-daemon/src/hooks.rs`), and hands the
    /// path to `spawn_claude` as an argument. A pure `(config_dir, cwd, id)` derivation in
    /// `micold-core` cannot see any of that. The variant's job is to say *which mechanism*, and
    /// the daemon supplies the file exactly as it does today.
    Hooks,
    /// The daemon tails an append-only event log the provider writes, at a path the provider
    /// *can* derive. (`copilot`.)
    EventLog { path: PathBuf },
    /// This provider reports nothing; the signal stays `Unknown`.
    None,
}
```

The asymmetry is the point: `EventLog` carries a path because
`config_dir/session-state/<id>/events.jsonl` is arithmetic, and `Hooks` carries none because the
equivalent is a runtime secret. A variant that looks uniform at the cost of being unimplementable is
worse than one that admits the two mechanisms differ.

### Rules

- **Path derivations are pure.** Everything that turns `(config_dir, cwd, id)` into a path does no
  I/O, so every layout is unit-testable without the CLI installed. This is how `ClaudeProvider` is
  tested today and the property must survive.
- **Reads are best-effort and never fail a session.** Missing, unreadable, or unparseable provider
  data degrades one capability (no title, no discovery, `Unknown` activity) and nothing more.
- **Unknown data is ignored, not rejected.** These are other tools' internal formats; they gain
  fields and event types between releases.
- **Object safety is required.** Consumers hold `&dyn AiCliProvider`; the seam test would stop
  compiling otherwise, which is the point.
- **The trait provides no layout-specific defaults.** A default implementation that assumes one
  CLI's storage shape is how the current trait came to be un-substitutable; the third provider must
  inherit nothing it has to override.

## Choosing an implementation

**The lookup lives in `micold-core`, not in the client.** An earlier draft of this contract put the
registry on `Capabilities` (`micold-client/src/shell/capabilities.rs`) and then observed, one
paragraph later, that "the daemon needs the same lookup" without saying where from. It cannot come
from there: `micold-daemon` depends on `micold-client` only as a **dev-dependency**
(`micold-daemon/Cargo.toml`), so production daemon code cannot see it, and `micold-core` cannot see
it at all — the dependency runs the other way, and `micold-core::terminal::claude_args` is one of
the call sites that needs an answer.

```rust
// micold-core/src/provider.rs — the one crate all three can see
impl AiCli {
    pub fn provider(self) -> &'static dyn AiCliProvider;   // exhaustive match, total by construction
}
```

An exhaustive match rather than a `BTreeMap`: the map made the lookup partial by type while every
caller wanted it infallible, and a `provider(which)` that can be absent is exactly the shape that
invites `unwrap()` on the set-wide paths where a wrong answer silently archives someone's sessions.
Totality is structural here, and it costs a `match`.

`Capabilities` stays the client's assembly point and **delegates**: `provider(which)` forwards to
`AiCli::provider`, `available_providers() -> Vec<AiCli>` filters the variants by `is_available()`,
and `real()` names no provider type at all. That is what keeps "each implementation is chosen in
exactly one place" true once two crates need the answer — the one place becomes core's definition
site. `tests/no_concrete_implementations.rs` derives its list of real implementations *from*
`micold-core`, so that definition site must be an **explicitly listed** exemption, not one the scan
happens not to reach.

The launch path is where the lookup costs more than a substitution.
`micold-core::terminal::LaunchSpec` has no provider field today and the function that reads it is
called `claude_args`, so the seam does not reach the spawn at all: the decision is made at two
`LaunchSpec { … }` construction sites in `micold-daemon/src/state.rs` (~838 and ~1031), each
followed by `hook_settings_file(id)` and `PtySession::spawn_claude`. Generalising it means

- `LaunchSpec` gains `provider: AiCli`, and `claude_args` becomes provider-driven;
- `spawn_claude` loses its name, and its `settings_file` argument becomes what the `Hooks` arm
  above asks for — supplied for a `Hooks` provider, absent for an `EventLog` one, rather than
  produced unconditionally for every `TerminalMode::AiCli` session;
- `catalog.rs` and `supervisor.rs` take the provider from the session record, which for those two
  really is a substitution.

Left undone, a Copilot session is spawned by `spawn_claude` with `claude`'s argv and a `--settings`
file `copilot` has no flag for.

## The guard (FR-022)

`micold-client/tests/no_concrete_implementations.rs` derives the set of real implementations from
`impl <Port> for <Type>` in `micold-core` and asserts only the shell names them. It already finds
`ClaudeProvider`, and will find `CopilotProvider` with no change.

**It does not currently reach the daemon or `core/terminal.rs`**, which is where every concrete
mention outside `capabilities.rs` lives: `micold-daemon/src/{catalog,supervisor}.rs` once each,
`state.rs` twice, and `core/terminal.rs`. FR-022 is satisfied only when its reach includes them, so
that a future `CodexProvider` cannot be wired in by naming it in the supervisor.

**And note what this guard structurally cannot catch.** The client's boot prune
(`micold-client/src/main.rs::prune_empty_sessions`, reached from `shell/startup.rs`) names nothing
concrete — it takes one `&dyn AiCliProvider` from `Capabilities` and applies it to every session in
every project. It passes this check today and would pass it after the reshape, while dropping every
Copilot session at startup. A name-based guard finds names; a hoisted provider over a heterogeneous
set is invisible to it, and only a test that mixes providers will do (T008b).
