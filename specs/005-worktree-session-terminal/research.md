# Phase 0 Research: Worktree & Session Navigation with Embedded Terminal

**Date**: 2026-07-15 | **Feature**: 005-worktree-session-terminal

All versions verified via crates.io / official docs during planning (2026-07). Pin exact
versions where noted — the iced 0.13 ↔ terminal-stack compatibility window is narrow.

---

## R1 — PTY management

**Decision**: `portable-pty = "0.9"` (wezterm) behind the `TerminalBackend` trait.

**Rationale**: De-facto cross-platform PTY crate — Unix `openpty`/`forkpty` and Windows
ConPTY (WinPTY fallback) behind one runtime API; no GUI dependency; used by
`alacritty_terminal`/`iced_term` underneath, so the stack stays coherent (Principle VI).
Spawn with cwd + env (`CommandBuilder::new("claude").cwd(worktree).env("TERM","xterm-256color")`),
get `try_clone_reader()` / `take_writer()` (blocking `std::io`), `resize()`, `child.kill()/wait()`.
Read loop runs on a dedicated thread; drop the `slave` after spawn so EOF propagates.

**Alternatives considered**: `pty-process` (async but Unix-only — fails Windows parity);
raw `nix`/`openpty` (Unix-only, hand-roll ConPTY); `conpty`/`winpty` (single-platform —
already wrapped by portable-pty).

## R2 — ANSI/VT parsing + terminal grid

**Decision**: `alacritty_terminal = "0.25"` (transitive via `iced_term`; pin the 0.25 line).

**Rationale**: Provides a GUI-free emulation *grid model* (`Term<T>`: `Grid` of styled
`Cell`s, cursor, scrollback, modes) — not just a byte parser. Feed raw PTY bytes in, read a
renderable grid out. Actively maintained; depends on `vte` transitively for the raw state
machine. The `Term` model is pure, but it is kept gui-side (not a core dependency); pure-core tests cover
per-session byte routing, and gui-gated tests cover grid rendering by feeding canned VT sequences.

**Alternatives considered**: `vte` alone (parser events only — would reimplement the whole
emulator); `wezterm-term`+`termwiz` (capable but large, ecosystem-coupled); `termwiz` alone
(TUI toolkit, wrong altitude).

## R3 — Rendering the terminal grid in iced 0.13

**Decision**: `iced_term = "0.6.0"` — the last release targeting **iced 0.13.1**.

**Rationale**: Real terminal widget on exactly this stack (iced + alacritty_terminal +
portable-pty), tested on all three OSes; implements rendering (via iced `canvas`), keyboard/
mouse input, resize, scrolling, focus, selection, colors. Gets a working terminal in iced
0.13 without hand-writing a cell renderer. **Version trap**: `iced_term` 0.7/0.8 and
`alacritty_terminal` 0.26 target iced **0.14** — do NOT take latest; pin `0.6.0` / `0.25`.

Verified compatibility mapping:

| iced_term | iced | alacritty_terminal |
|-----------|-------|--------------------|
| 0.5.0 | ^0.13.1 | ^0.24 |
| **0.6.0** | **^0.13.1** | **^0.25** |
| 0.7.0 | ^0.14 | ^0.25 |

**Maturity caveat**: single maintainer, pre-1.0, "no stable API until iced 1.0." Mitigation:
pin exact version, keep the `TerminalBackend` trait as the seam, be ready to vendor/fork. It
exposes the `alacritty_terminal` backend so a custom `canvas` renderer is the escape hatch.

**Fallback (custom renderer)** if we outgrow `iced_term`: render the grid with
`iced::widget::canvas` (runs of same-style cells → text over colored rects, fixed monospace
cell metric), derive rows/cols from pane size, feed back to `master.resize()` + `Term::resize()`.
Use `canvas::Cache`; coalesce redraws to ≤1/frame; cap scrollback (avoids redraw storms).

**Alternatives considered**: per-cell `text` widgets (terrible perf); `iced_aw` (no terminal
widget); upgrading the whole app to iced 0.14 to use iced_term 0.8 (violates "iced 0.13 only").

## R4 — Streaming child output into iced (0.13)

**Decision**: Bridge the blocking PTY reader thread to iced with a `tokio::sync::mpsc`
channel surfaced as a long-running `Subscription::run_with_id(session_id, iced::stream::channel(...))`.
Hold the PTY writer (or a sender to a writer thread) in `State`; write from `update` on key input.

**Rationale**: `run_with_id` is the 0.13 way to declare a background stream keyed by a stable
id — started once, kept alive while returned from `subscription()`, torn down when dropped.
`stream::channel` is iced's "spawn a worker, hand back a Sink of Messages" helper. Reader
thread `read()`s 8 KiB chunks → `tx.blocking_send` → subscription forwards
`Message::PtyOutput { id, chunk }`; EOF → `Message::PtyExited { id }`. `Task`/`Command` is
wrong here (finite/one-shot, not an unbounded read loop).

**Redraw coalescing**: feed bytes into the session's `Term` on every chunk but mark a
per-session dirty flag / invalidate `canvas::Cache` at most once per frame (~16 ms).

**Alternatives considered**: `Task::run`/`Task::stream` (works but a subscription models a
long-lived source with lifecycle tied to app state); an async PTY crate (rejected — Windows
parity, R1). The blocking-thread + channel bridge is cheap and standard.

## R5 — Multiple concurrent terminals

**Decision**: One PTY + one `Term` + one reader thread **per session**, keyed by `SessionId`;
`Subscription::batch(sessions.map(|id| terminal_subscription(id)))`. `HashMap<SessionId,
Session>` in `State`; `view()` renders only the active session, but every session's
subscription/reader stays alive (FR-015b).

**Pitfalls captured**: always tag messages + subscriptions with the session id (shared ids
→ iced dedupes → a terminal silently stops); background sessions update their `Term` but must
not force UI repaints (per-session dirty flag, repaint on switch); bounded `mpsc` (cap ~100)
gives natural backpressure; on close remove from the map (→ iced tears down the subscription),
`child.kill()+wait()`, join the reader thread; kill all children on app shutdown (`Drop`) to
avoid zombies.

## R6 — Claude Code CLI session mechanics

Verified against Claude Code **v2.1.210** (`claude --version`). JSONL on-disk format is
documented as *internal* — treat any file parsing as best-effort and degrade gracefully.

**Decision — session id (app-controlled)**: pre-generate a UUID v4 and launch with
`claude --session-id <uuid>` (available **v2.1.210+**) so the app owns the id up front — no
filesystem watching needed. Fallback for older CLIs: launch with `-p --output-format json`
once and read the `session_id` field, or watch `~/.claude/projects/<encoded-cwd>/` for the new
`<uuid>.jsonl`. The app MUST detect the CLI version (`claude --version`) and gate `--session-id`.

**Decision — working directory**: launch `claude` with **process cwd = the worktree path**.
`claude` scopes the session to its cwd's project; different worktrees → separate sessions. No
`--project-dir` flag; setting cwd is the mechanism.

**Decision — resume**: restore/reopen with `claude --resume <uuid>` (accepts a specific id
non-interactively; `-r <uuid>` equivalent). Must run from (or within) the session's original
worktree cwd. Missing id → `No conversation found` error (surface it; offer a fresh session).
`--continue`/`-c` resumes the most recent session in the cwd (not used — we target a specific id).

**Decision — session name/label**: best-effort read the latest `"type":"ai-title"` record
(field `aiTitle`) from `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`; the title is
appended/updated as the conversation grows (**v2.1.205+**). The encoded dir name is the full
cwd with non-alphanumerics replaced by `-` (respect `CLAUDE_CONFIG_DIR` override). Until a
title exists, show a placeholder label; update the label when the title appears (FR-011a).
Because the JSONL format is internal, wrap extraction behind the `Git`-analogous boundary and
never fail a session if the title can't be read.

**Notes**: `-p`/`--print` is non-interactive (not used — we want the interactive TUI);
`--model`/`ANTHROPIC_MODEL` select the model; version-gated features: `--session-id`
(2.1.210+), `ai-title` (2.1.205+), default session names (2.1.196+).

## R7 — Git worktree + branch creation, discovery, sanitization

**Decision — mechanism**: shell out to the user's `git` binary via `std::process::Command`
behind a `Git` trait (mirrors `FolderScanner`). No git crate.

**Rationale**: `git worktree add -b <branch> <path> HEAD` does branch-at-HEAD + worktree
registration + checkout in one native, cross-platform command matching the requirement
exactly. `git2`/libgit2 1.9 has no `-b` equivalent (must create branch then
`worktree_add` with `WorktreeAddOptions::reference`, runs no hooks) **and rejects
`extensions.relativeworktrees`** (git ≥ 2.48) — a live incompatibility. `gitoxide` has no
worktree-*add* at all. The app already depends on git; shelling out adds zero deps and is
byte-identical to user expectations. Porcelain formats are stable, documented contracts.

**Decision — create + rollback sequence**:
1. `std::fs::create_dir_all(.claude/worktrees/)` (remember if we created it).
2. `git -C <project> worktree add -b <type>/<ticket>-<name> .claude/worktrees/<dir> HEAD`.
3. On failure, unwind in order: `worktree remove --force <path>` → `worktree prune` →
   `branch -D <branch>` (ignore "not found") → remove the target dir if still present.
   Order matters: remove the worktree registration **before** deleting the branch (git
   refuses to delete a branch checked out in a worktree). The rollback is modeled as an
   ordered `Vec<CleanupStep>` in the pure core so it is unit-testable without a real repo.

**Decision — validation** (pure gates before any mutation): repo check via
`git -C <dir> rev-parse --show-toplevel` (compare to opened path — stricter than `.git`
existence, correct for `.git`-file worktrees); branch-name validity via git
`check-ref-format` rules (our slugify output `[a-z0-9-]` passes by construction, but validate
anyway); duplicate detection via `show-ref --verify refs/heads/<branch>` and the porcelain
worktree list, plus a non-empty-target-dir check.

**Decision — discovery + health**: `git -C <project> worktree list --porcelain -z`, parsed by
a **pure** function, then cross-checked against the filesystem to classify each worktree under
`.claude/worktrees/` as **Valid** (listed + dir exists), **Missing** (listed + dir gone; git
flags `prunable`), or **Invalid/Orphan** (dir exists but not registered). `prune` clears stale
admin records only; `remove` deletes a live worktree; an orphan dir git never registered needs
a direct `fs` delete. (Removal itself is deferred scope; discovery/flagging is in scope,
FR-018a.)

**Decision — slugify**: hand-rolled, zero-dependency (research): lowercase → replace non
`[a-z0-9]` with `-` → collapse/trim `-` → reject empty → guard git tails (`.lock`, leading
`.`/`-`, `..`, `@`) and Windows reserved device names (`con`, `nul`, …). Output `[a-z0-9-]`
is valid as both a git ref component and a cross-OS directory name. Slugify is a normalizer;
the validity gates above remain the authority.

**Alternatives considered**: `git2 0.21`/libgit2 (heavier C build, no `-b`, relative-worktree
incompatibility — kept as a *possible* alternate trait impl only); `gix 0.85` (great for
read/discovery, but no worktree-add — possible future for the read half); `slug` 0.1.6 crate
(nice Unicode transliteration, but the hand-rolled version honors the minimal-deps principle).

## R8 — Testability boundary (carried across R1–R7)

**Decision**: keep the pure core free of `portable-pty`, `git`, and spawned processes. Two
I/O traits (`Git`, `TerminalBackend`) with fake in-memory impls for tests; the real impls
live at the binary boundary (`GitCli` via `std::process` in `src/git.rs`; the `portable-pty`
terminal impl gui-gated in `src/ui/terminal.rs`). `update` consumes `PtyOutput`/`PtyExited`
messages it is *given*, so grid/lifecycle behavior is deterministic under
`cargo test --no-default-features` with canned VT bytes and a `FakeGit` primed to fail at a
chosen step (exercises rollback). This matches the existing `FolderScanner`/`ProjectStore`
pattern.

---

## Resolved unknowns

| Unknown (from Technical Context) | Resolution |
|----------------------------------|------------|
| Embedded terminal widget for iced 0.13 | `iced_term 0.6.0` (+ portable-pty 0.9, alacritty_terminal 0.25); pin exact (R1–R3) |
| Stream PTY → iced runtime | `Subscription::run_with_id` + tokio mpsc; coalesce redraws (R4) |
| Concurrent background sessions | per-session PTY/Term/thread/subscription keyed by SessionId (R5) |
| Get + control `claude` session id | `--session-id <uuid>` (2.1.210+), version-gated; fallbacks documented (R6) |
| Resume after restart | `claude --resume <uuid>` from worktree cwd (R6) |
| Session label source | best-effort `ai-title` from session JSONL, placeholder until present (R6) |
| Create worktree + branch | `git worktree add -b <branch> <path> HEAD` via CLI; ordered rollback (R7) |
| Repo gate for "open project" | `git rev-parse --show-toplevel` (R7) |
| Slugify ticket/name | zero-dep normalizer to `[a-z0-9-]`; validate against check-ref-format (R7) |
| MSRV / new deps vetting | bump rust-version for iced_term/alacritty; uuid v4 core dep; all gui deps gated (Principle V) |
| Keep core testable | `Git` + `TerminalBackend` traits + fakes; PTY impl gui-gated (R8) |

No `NEEDS CLARIFICATION` markers remain.
