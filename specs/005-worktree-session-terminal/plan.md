# Implementation Plan: Worktree & Session Navigation with Embedded Terminal

**Branch**: `005-worktree-session-terminal` | **Date**: 2026-07-15 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-worktree-session-terminal/spec.md`

## Summary

Add project-scoped worktree and session management to the existing Material Design app shell. The user opens a git repository as a project, creates worktrees through a form (Conventional-Commits `type` + optional `ticket` + `name`), each becoming a new git branch (`${type}/${ticket}-${name}`) bound to a git worktree at `.claude/worktrees/${type}-${ticket}-${name}`. A left navigation sidebar shows worktrees (top level) → sessions (sub-items) as a reusable **TreeView**. Selecting a worktree lets the user start a session; an active session shows an embedded terminal on the right running the `claude` CLI in that worktree. Sessions run concurrently in the background, persist (id + `claude` session name + worktree binding), auto-restart on crash, resume via `claude --resume <id>` after restart, and stop when the project is closed/switched.

**Technical approach**: extend the render-free core (pure `State`/`Message`/`update`) with new pure domain modules (naming derivation, worktree model + porcelain parsing, session lifecycle state machine) and two new I/O traits mirroring the existing `FolderScanner` pattern — a `Git` boundary (shells out to the `git` binary) and a `TerminalBackend` boundary (wraps `portable-pty` + `alacritty_terminal`, gui-gated). The iced binary supplies the real implementations and streams PTY output into the runtime via `Subscription::run_with_id` + a tokio channel. Two shared UI primitives — `TreeView` and `IconButton` — are added to a new `src/ui/components/` library (Constitution Principle VIII).

## Technical Context

**Language/Version**: Rust, edition 2021. Current `rust-version = 1.80`. Determine the exact MSRV from the terminal crates' stated Rust versions and pin `rust-version` to max(1.80, that value) in Cargo.toml (do not leave unbounded); verify in CI.

**Primary Dependencies**:
- Existing: `iced 0.13` (add features `canvas`, `advanced`, `lazy`; already has `tokio`), `serde`/`serde_json`, `directories`, `dark-light` (gui).
- New (gui-gated, Principle V dependency vetting required):
  - `iced_term = "0.6.0"` — the last release targeting iced 0.13.1 (0.7+ moved to iced 0.14). Embedded terminal widget over `portable-pty` + `alacritty_terminal`. Pin exactly; be prepared to vendor/fork (single-maintainer, pre-1.0).
  - `portable-pty = "0.9"` — cross-platform PTY (openpty / ConPTY). Transitive via `iced_term`; declared explicitly for the `TerminalBackend` impl.
  - `alacritty_terminal = "0.25"` — GUI-free VT grid model (transitive via `iced_term`; pin the 0.25 line for iced 0.13). Lives gui-side only; the pure core routes PTY bytes to a per-session sink and does NOT depend on `alacritty_terminal` (keeps `--no-default-features` lean). Grid rendering is validated by gui tests.
- New (core, non-gui): `uuid = { version = "1", features = ["v4"] }` — app pre-generates the session UUID passed to `claude --session-id`. Small, ubiquitous, justified.
- **Git**: no crate — shell out to the user's `git` binary via `std::process::Command` behind the `Git` trait. Rationale in research.md R3 (libgit2 has no `worktree add -b` equivalent and rejects `extensions.relativeworktrees`; gitoxide has no worktree-add). Zero new dependency, byte-identical to user expectations.
- **Slugify**: hand-rolled zero-dependency normalizer in the pure core (research R7); output alphabet `[a-z0-9-]` satisfies both git `check-ref-format` and cross-OS directory rules.

**Storage**: Local-first (Principle IV). Extend the existing per-user JSON store pattern. Worktrees are discovered from git (`worktree list --porcelain`) — not persisted as the source of truth. **Sessions** are persisted per project (session id, `claude`-provided name, worktree binding) so they restore across restarts. New schema documented in `contracts/storage-schema.md`.

**Testing**: `cargo test --no-default-features` exercises the entire pure core (naming, worktree model + porcelain parsing, session lifecycle, rollback-plan) against fake `Git`/`TerminalBackend` impls — no real git, no spawned processes, no GUI. GUI-gated integration tests cover the iced adaptation.

**Target Platform**: Desktop — Linux, macOS, Windows (Principle VI, CI on all three).

**Project Type**: Desktop application (single Rust project; render-free lib core + gui binary).

**Performance Goals**: 60 fps UI; terminal output coalesced to ≤1 redraw/frame (~16 ms) with `canvas::Cache` (research R3); open project + list worktrees < 3 s (SC-001); session start to interactive terminal < 5 s (SC-004); bounded PTY channel + capped scrollback per session.

**Constraints**: The app's own functionality is fully offline/local-first; the `claude` process's network use is the tool's concern, not app state (see Constitution Check IV). Invalid worktree/session states made unrepresentable via enums (Principle V).

**Scale/Scope**: A handful of worktrees per project and a handful of concurrent sessions each; N background PTYs (one reader thread + one bounded channel + one subscription each). Scrollback trimmed to a fixed cap per session.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: All new logic lands as pure core modules (naming, worktree model, porcelain parsing, session lifecycle, rollback plan) tested first via `cargo test --no-default-features`. `Git` and `TerminalBackend` traits get fake impls so orchestration (create+rollback, crash-restart, project close) is unit-tested with no real git/processes.
- [x] **II. Multi-Session Support**: Sessions are first-class, independently addressable (`SessionId` UUID), persisted (id + name + worktree binding) and restorable (`claude --resume <id>`); each has its own PTY/`Term`/reader task and leaks no state into another (per-session routing keyed by id). Concurrent background sessions supported (FR-015b).
- [x] **III. Worktree Integration**: The app owns worktree create + switch natively (no manual git); every file/VCS op is worktree-aware (branch bound at creation, cwd set per session). *Note*: full worktree **deletion** is a deferred scope increment (spec clarification); session close/stop is owned here. This is a scope boundary, not a manual-git regression — no unjustified deviation.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: All app state (projects, sessions) lives on the local filesystem; the app opens projects, creates worktrees, and manages sessions offline. The embedded terminal is a generic PTY; the `claude` process's own network use is external tool behavior, not app-persisted state, and nothing is transmitted off-device by the app without user action.
- [x] **V. Rust + iced Stack**: Rust + iced 0.13 only. New crates are gui-gated and justified above (Principle V dependency vetting). Enums make invalid states unrepresentable (`WorktreeStatus`, `SessionLifecycle`, `Overlay` extension).
- [x] **VI. Cross-Platform Parity**: `portable-pty` (ConPTY/openpty), the `git` CLI, and `iced_term` all support Linux/macOS/Windows; platform specifics stay inside those crates/behind the traits. CI builds + tests all three.
- [x] **VII. Documentation First-Class**: User-guide docs added in the same change — opening a project (git-only), creating worktrees, starting/switching/closing sessions, the embedded terminal. Verified in CI docs build.
- [x] **VIII. Reusable UI Component Foundation**: The sidebar tree and icon-only actions are built as shared primitives `TreeView` and `IconButton` in `src/ui/components/`, reused (not forked); both honor light/dark theming and cross-platform parity.

**Result: PASS — no violations. Complexity Tracking left empty.**

## Project Structure

### Documentation (this feature)

```text
specs/005-worktree-session-terminal/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── git-trait.md
│   ├── terminal-backend-trait.md
│   ├── claude-cli.md
│   ├── naming.md
│   └── storage-schema.md
├── checklists/
│   └── requirements.md  # (from /speckit-specify + /speckit-clarify)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── lib.rs                  # add: pub mod naming, worktree, session, git;
│                           #      (terminal backend trait lives here too)
├── app.rs                  # extend State/Message/update: worktree tree, add-worktree
│                           #   form, session lifecycle, active session/terminal routing
├── naming.rs               # NEW (pure): slugify + derive dir/branch + validation
├── worktree.rs             # NEW (pure): Worktree, WorktreeStatus, porcelain parsing,
│                           #   create-orchestration + rollback plan (as data)
├── session.rs              # NEW (pure): Session, SessionLifecycle state machine,
│                           #   crash-restart policy, persisted session shape
├── git.rs                  # NEW: `Git` trait (I/O boundary) + `GitCli` (std::process)
│                           #   + fake for tests (cfg(test))
├── terminal.rs             # NEW (pure): `TerminalBackend` trait + Session terminal
│                           #   state; real portable-pty impl is gui-gated (see ui/)
├── store.rs                # extend: persist sessions per project (new schema section)
├── fs_scan.rs, project.rs, workspace.rs, selector.rs, settings.rs,
│   theme.rs, tokens.rs, icons.rs, metadata.rs   # existing (reused)
├── main.rs                 # wire GitCli + real TerminalBackend; PTY subscriptions
└── ui/                     # gui-gated iced layer
    ├── mod.rs              # add sidebar + session/terminal panes to view()
    ├── components/         # NEW shared primitives (Principle VIII)
    │   ├── mod.rs
    │   ├── tree_view.rs    # TreeView<Node> primitive
    │   └── icon_button.rs  # IconButton primitive
    ├── sidebar.rs          # NEW: worktree→session tree, add-worktree affordance
    ├── worktree_form.rs    # NEW: add-worktree form (type/ticket/name + derived preview)
    ├── terminal.rs         # NEW (gui): iced_term/portable-pty-backed TerminalBackend
    │                       #   impl + terminal pane rendering
    ├── shell.rs            # extend: two-pane layout (sidebar | session/terminal)
    ├── style.rs, toolbar.rs, about.rs, project_selector.rs, rename.rs, theme_menu.rs
    └── (existing modules reused)

tests/                      # pure-core integration tests (--no-default-features)
├── naming.rs               # NEW: slugify/derive/validation
├── worktree_create.rs      # NEW: create + rollback vs FakeGit
├── worktree_discovery.rs   # NEW: porcelain parse + status classification
├── session_lifecycle.rs    # NEW: start/switch/close/crash-restart/project-close
├── session_store.rs        # NEW: persist + restore sessions roundtrip
└── (existing tests reused)

docs/user-guide/
├── worktrees-and-sessions.md   # NEW (Principle VII)
└── (existing guides)
```

**Structure Decision**: Extend the established render-free-core + gui-binary layout (no new crates/workspaces). New pure logic goes in `src/{naming,worktree,session}.rs` and stays testable under `--no-default-features`. I/O sits behind `Git` (in `src/git.rs`, `std::process`) and `TerminalBackend` (trait in `src/terminal.rs`; real `portable-pty` impl gui-gated in `src/ui/terminal.rs`), mirroring the existing `FolderScanner`/`ProjectStore` boundary. Shared UI primitives live in `src/ui/components/` per Principle VIII.

## Complexity Tracking

*No constitution violations — no entries.*
