# Phase 0 Research: Switchable Regular Terminal Mode

All Technical-Context unknowns are resolved below. Evidence was gathered by reading the current
implementation (`src/main.rs`, `src/app.rs`, `src/session.rs`, `src/terminal.rs`,
`src/provider.rs`, `src/store.rs`, `src/icons.rs`, `src/ui/terminal.rs`,
`src/ui/material/icon_button.rs`, `src/ui/material/mod.rs`, `assets/fonts/PROVENANCE.md`) and
prior feature specs/plans (005, 006, 008).

## R1 — Two processes per session: `SessionTerminals`, not two `HashMap`s

**Decision**: Change `App.terminals` (`src/main.rs`) from `HashMap<SessionId, RuntimeTerminal>`
to `HashMap<SessionId, SessionTerminals>`, where:

```rust
pub struct SessionTerminals {
    pub ai_cli: Option<RuntimeTerminal>,
    pub shell: Option<RuntimeTerminal>,
}
```

with `attached(&self, mode: TerminalMode) -> Option<&RuntimeTerminal>` / `attached_mut` helpers
picking the right field. Every call site that currently does `app.terminals.get(&id)` /
`.get_mut(&id)` / `.insert(id, rt)` / `.remove(&id)` (the `TerminalTick` pump loop, `pane()`'s
`RuntimeTerminal` borrow, `TerminalBytes` write-through, `handle_process_exits`,
`SessionCloseRequested`) is updated to go through the session's `SessionTerminals` entry and the
relevant slot.

**Rationale**: A single `HashMap<(SessionId, TerminalMode), RuntimeTerminal>` was considered and
rejected — it makes "does this session have a process in the other mode running right now"
(needed by FR-004/FR-005's "reattach if already running" check) an extra lookup with a
synthesized key, and it would let two entries for the same session disagree about which mode is
"current" (that's tracked once, on `Session.mode`, not derivable from map keys). A struct with
two named `Option` slots keeps "at most one shell process and one AI CLI process per session"
(spec Assumptions) true by construction and keeps every existing single-process call site's
change to "add `.attached(mode)` /` .ai_cli` / `.shell`" mechanical and easy to review.

**Alternatives considered**: A `Vec<(TerminalMode, RuntimeTerminal)>` per session — strictly less
type-safe than two named fields for a fixed set of exactly two kinds. Rejected.

## R2 — Shell lifecycle is simpler than `SessionLifecycle`, not a copy of it

**Decision**: Add a new enum, `ShellLifecycle`, independent of `SessionLifecycle`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellLifecycle {
    #[default]
    NotStarted,
    Starting,
    Running,
    Exited,
}
```

No `Restarting { attempts }` and no `Failed` variant, and therefore no `on_unexpected_exit`-style
crash-loop method — per the clarification recorded in spec.md (Session 2026-07-18), an exited
shell (intentional `exit` or a crash — indistinguishable at the process-exit level) always just
becomes `Exited`, with restart only ever a manual, user-triggered action (a new
`Message::TerminalRestartRequested`, contracts/terminal-mode-lifecycle.md).

**Rationale**: Mirroring `SessionLifecycle`'s five states (including `Restarting`/`Failed`) would
either (a) silently apply the AI CLI's 3-attempt crash-loop guard to the shell too — contradicting
the clarification — or (b) require the crash-loop fields to sit unused on every shell, an
unrepresentable-state smell the type system should reject (Principle V). A four-state enum with
no retry-count field makes "the shell never auto-restarts" true by construction: there is no
field to increment.

**Alternatives considered**: Reusing `SessionLifecycle` directly for the shell and just never
calling `on_unexpected_exit` on it — rejected because it leaves `Restarting`/`Failed` reachable
in type but never in practice, which is exactly the kind of invalid-state-should-be-unrepresentable
case Principle V calls out.

## R3 — Resolving the platform's default shell, testably

**Decision**: A pure function resolves the shell command from already-read environment values
(no direct `std::env::var` call inside the pure function itself, so it is unit-testable under
`--no-default-features` without mutating process env):

```rust
// src/terminal.rs (pure core), alongside the existing claude_args() seam
pub fn default_shell_command(shell_env: Option<&str>, comspec_env: Option<&str>) -> String {
    if cfg!(windows) {
        comspec_env.filter(|s| !s.is_empty()).unwrap_or("cmd.exe").to_string()
    } else {
        shell_env.filter(|s| !s.is_empty()).unwrap_or("/bin/sh").to_string()
    }
}
```

The impure `std::env::var("SHELL")` / `std::env::var("COMSPEC")` reads happen once at the
call site in `src/main.rs` (or `src/ui/terminal.rs`'s `spawn_shell_pty`), mirroring how
`ClaudeProvider::config_dir` already reads `CLAUDE_CONFIG_DIR` at the edge and passes a value in.

**Rationale**: Matches the spec Assumption ("the platform's standard interactive default shell
— the same shell a standalone terminal would launch") and Constitution Principle VI (platform
branching isolated behind one small abstraction, not scattered `cfg!` checks). Keeping the
`cfg!(windows)` branch inside a pure, argument-driven function — rather than reading env vars
directly — keeps it testable on any host OS: a test can assert Windows-branch behavior on a
Linux CI runner by calling the function with `cfg!(windows)`'s *logical* branch exercised via a
second helper split by platform, or (simpler, chosen) by testing both branches' argument-handling
logic (empty-string / `None` fallback) independent of which `cfg!` branch is compiled — see
`tests/shell_command.rs` in Project Structure.

**Alternatives considered**: A `ShellProvider` trait mirroring `AiCliProvider` — rejected as
over-engineering for a single command string with no argument shape, transcript, or title
parsing to abstract; `AiCliProvider`'s seam exists because *multiple* AI CLIs are anticipated
(FR-024, feature 005), not multiple shells.

## R4 — Spawning the shell: factor `spawn_pty`, don't duplicate it

**Decision**: Factor the PTY-open + `Term`-construction body of `src/ui/terminal.rs::spawn_pty`
(currently: open PTY, build `Term` with `Config { scrolling_history }`, spawn a reader thread,
assemble `RuntimeTerminal`) out into a private helper that takes an already-built
`portable_pty::CommandBuilder` and `cwd`. `spawn_pty` (claude) becomes a thin wrapper that builds
its `CommandBuilder` from `ClaudeProvider.command()` + `claude_args(spec)` (unchanged); a new
`spawn_shell_pty(cwd: &Path, env: &[(String, String)], scrollback_lines: usize) -> io::Result<RuntimeTerminal>`
builds its `CommandBuilder` from `default_shell_command(...)` with no extra args.

**Rationale**: The two functions would otherwise be near-identical copies (same PTY size, same
`Term`/`Config`, same reader-thread spawn) differing only in how the `CommandBuilder` is built —
duplicating that is exactly the kind of one-off fork Principle VIII's reuse posture argues
against, even though `IconButton`/component reuse is the principle's literal UI-widget target;
the same reasoning applies to this process-spawning seam.

**Alternatives considered**: Giving `spawn_pty` a `command_and_args: (String, Vec<String>)`
parameter directly instead of a `CommandBuilder` — equivalent in effect, slightly less
idiomatic against `portable-pty`'s own builder API. Either is acceptable; the private-helper
split is chosen for clearer call sites (`spawn_pty(spec, ...)` keeps its existing signature,
`spawn_shell_pty` gets one shaped for what it actually needs — no `LaunchMode`/`session_id`,
which don't apply to a shell).

## R5 — Persisting the mode: extend `StoredSession`, backward compatible

**Decision**: Add `#[serde(default)] mode: StoredTerminalMode` to `StoredSession`
(`src/store.rs`), where `StoredTerminalMode` is a small serde-mapped mirror of `TerminalMode`
(`AiCli` default). `StoredCatalog::from_workspace` writes `s.mode` from the runtime `Session`;
`into_workspace` reads it back into `Session::restored(..., mode)`. No `schema_version` bump —
the exact pattern feature 008 used for `worktree_display_names` (`#[serde(default)]` keeps files
written before this feature loading unchanged, per `store.rs`'s existing comment convention).

**Rationale**: Matches FR-011 (mode persisted and restored across app restarts) and the
project's established backward-compatible persistence idiom; avoids a schema-version gate for
an additive, defaultable field.

**Alternatives considered**: A separate `terminal_modes: BTreeMap<Uuid, TerminalMode>` sidecar
map — rejected, no other per-session field uses a sidecar map (`title` lives inline on
`StoredSession`); inline keeps one record per session, consistent with the existing shape.

## R6 — Background pumping: extend the existing tick loop, don't add a new one

**Decision**: `Message::TerminalTick`'s handler (`for rt in app.terminals.values_mut() { rt.pump()
}`) becomes `for st in app.terminals.values_mut() { if let Some(rt) = &mut st.ai_cli { rt.pump()
}; if let Some(rt) = &mut st.shell { rt.pump() } }` — both slots are pumped every tick regardless
of which is attached/visible. `handle_process_exits` similarly scans both slots per session.

**Rationale**: Feature 008 already established that a session's process keeps running and being
serviced in the background when its project isn't even the active one ("switched-away sessions
keep running in the background", `session.rs` doc comment; `restarted_while_inactive` /
`note_background_restart`). Extending that same tick to a second slot on the *same, currently
displayed* session is strictly less surprising than introducing an "only pump the attached
process" special case, and it's what keeps a backgrounded `claude` process's crash-loop
auto-restart working even while Regular Terminal mode is displayed (spec User Story 2, Scenario
2 — the turn "continues uninterrupted in the background"). It also keeps `Term` state for the
detached process current, so re-attaching renders instantly correct instead of needing to
fast-forward a backlog.

**Alternatives considered**: Only pumping the attached process, and bulk-draining the detached
one's raw output buffer on re-attach — rejected: bulk-draining a potentially large backlog into
`Term::advance` on the main update tick risks a visible hitch exactly at the moment SC-001
(<500ms, no perceptible delay) is measured, and it does nothing for the AI CLI background-crash-
restart requirement above, which needs the exit to be *detected* while backgrounded, not just
the output drained.

## R7 — Two new icons: sourced from the already-vendored full-coverage font, not a font rebuild

**Decision**: Add two `Icon` variants (e.g. `Icon::AiCli`, `Icon::RegularTerminal` — final names
chosen during task breakdown) to `src/icons.rs`, each mapped to a Material Symbols codepoint
looked up in the vendored font's own codepoints reference (the font ships with **full** Unicode
coverage as of feature 009 — `assets/fonts/PROVENANCE.md`: "adding a new `Icon` variant never
again requires regenerating this binary — only `src/icons.rs` + `tests/icons.rs` change"). The
exact codepoints are a task-level lookup against the vendored
`MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints` reference (e.g. glyphs named `terminal`
and `smart_toy` or `robot_2` are the natural fits), not guessed here — `tests/icons.rs`
regression-locks whatever is chosen, exactly as it does for `Filter`'s `E152`.

**Rationale**: This is precisely the workflow feature 009's font-coverage change was built for;
no PROVENANCE.md/font regeneration step is needed, only the two-file change the file's own doc
comment describes.

**Alternatives considered**: Reusing an existing `Icon` variant (e.g. `Settings`) for one of the
two states — rejected: FR-009 requires the toggle's icon to be an unambiguous, at-a-glance
mode indicator, which fails if it's shared with an unrelated action's icon elsewhere in the UI.

## R8 — The shell's "manual restart affordance" (FR-013) reuses one small UI addition, not a new pattern

**Decision**: Add one `Message::TerminalRestartRequested` and a small text/icon button shown in
the terminal's bottom status bar whenever the *currently attached* process (AI CLI or shell,
whichever `Session.mode` selects) is not running — i.e. `SessionLifecycle` is `Idle`/`Failed`
while in AI CLI mode, or `ShellLifecycle` is `Exited`/`NotStarted` while in Regular mode. The
binary interprets the message per current mode: AI CLI → the same spawn-if-absent `Resume` path
`SessionSelected` already uses; Regular → `spawn_shell_pty`.

**Rationale**: Today there is no explicit "restart" UI at all — an `Idle`/`Failed` AI CLI
session only recovers via re-selecting it in the sidebar (which triggers main.rs's existing
spawn-if-absent check). That implicit path doesn't extend to a shell (there's nothing to
"re-select"), so FR-013's explicit requirement for a restart affordance needs one small new
control. Making it mode-generic (same button, dispatches on current mode) avoids adding two
near-duplicate buttons and gives the AI CLI path an explicit affordance it was previously
missing, for free.

**Alternatives considered**: Requiring the user to toggle away and back to implicitly respawn a
dead shell — technically satisfies "kept alive and resumable" but is a confusing two-click
workaround, and does not read as a "restart affordance" (FR-013's explicit wording). Rejected.
