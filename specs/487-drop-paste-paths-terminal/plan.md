# Implementation Plan: Drop Files or Paste Images into a Terminal to Insert Their Paths

**Branch**: `claude/project-thread-wysm57` (spec dir `487-drop-paste-paths-terminal`) | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/487-drop-paste-paths-terminal/spec.md` (clarified, CLEAN)

## Summary

Dropped OS files and pasted clipboard images become shell-quoted paths typed (never submitted) at a
terminal's input. All decisions live in render-free `micold-core` (`path_insert`: shell quoting, the
sandbox translate-or-refuse plan, pasted-image locations and orphan selection). The client adds the
impure edges: an iced file-drop subscription routed to the pane under the pointer (new pointer state), a clipboard image
read when the clipboard holds no text, the PNG write, and a notice for each refusal or failure. The
daemon removes a session's pasted images in `delete_session` and sweeps orphans at start. Insertion
reuses the existing `TerminalBytes` path with `keymap::paste_bytes`, so no newline is ever produced
(bracketed when the program asked for it). See [research.md](research.md) for the decisions.

## Technical Context

**Language/Version**: Rust (workspace edition), iced 0.14

**Primary Dependencies**: existing `iced`, `directories`; new in `micold-client` only: `arboard`
(clipboard image read; `iced::clipboard` is text-only) and `png` (encode RGBA; already in the tree
through tiny-skia's `png-format`). No new core dependency.

**Storage**: local files only. Pasted images: `<worktree>/.micold-pasted/<session-id>/<n>.png`, or
`<data_dir>/pasted/<session-id>/` when the session has no writable worktree. Sandboxed sessions use
the sandbox state mount (`MountSet.state`, already container-visible) in place of `<data_dir>`.

**Testing**: `cargo test` via `mise run gate`; core unit tests, client reducer tests, a real-shell
round-trip test, geometry gate, quickstart Part B visual pass.

**Target Platform**: Linux, macOS, Windows (Principle VI).

**Project Type**: desktop app (core / client / daemon crates).

**Performance Goals**: path visible within 1 s of drop (SC-001); quoting and planning are O(path length).

**Constraints**: offline; no remote service; no change to tracked files of the user's project.

**Scale/Scope**: a drop of tens of files; one image of screenshot size per paste.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 (unchanged).*

- [x] **I. Test-First**: every unit in the Test Strategy below has a failing test written first; tasks.md orders tests before code (`speckit-tdd-plan`).
- [x] **II. Multi-Session**: pasted images are keyed by session id, one directory each; drops go to the pane under the pointer; nothing is shared between sessions (FR-005, spec edge "two sessions pasting").
- [x] **III. Worktree Integration**: images live in the session's worktree (hidden from VCS status by the repository's local exclude file, no tracked file touched) or, for the Default session, in the session temp dir; no other non-worktree location.
- [x] **IV. Local-First**: only local files and the OS clipboard; works offline.
- [x] **V. Rust + iced**: `ShellKind`, `InsertedPath` and `Refusal` are enums/structs; an unrepresentable path is an `Err`, not a garbled string.
- [x] **VI. Cross-Platform**: quoting is a pure function per `ShellKind`, tested for all kinds on every OS (as `pathmap` does with `windows_host`); only the clipboard read and temp-root lookup are platform edges, behind `shell/`.
- [x] **VII. Documentation**: `docs/user-guide/terminal-panes.md` gains a "Drop files and paste screenshots" section and a sandbox note in `sandboxed-daemon.md`, in the same change.
- [x] **VIII. UI Components**: no new widget. Feedback reuses the existing notice banner (`notify_error`/`notify_info`); drop hit-testing uses pane rectangles from the existing split layout.

## Project Structure

### Documentation (this feature)

```text
specs/487-drop-paste-paths-terminal/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/terminal-insertion.md
└── tasks.md             # /speckit-tasks, not created here
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── path_insert/mod.rs        # NEW  plan_insertion, InsertionPlan, Refusal, Insertion target
├── path_insert/quote.rs      # NEW  ShellKind, quote() per shell, Unrepresentable
├── path_insert/pasted.rs     # NEW  PastedLayout: dirs, file names, exclude line, orphan selection
├── lib.rs                    # add `pub mod path_insert`
├── terminal.rs               # path_insert::ShellKind::detect(shell command), beside default_shell_command (:62)
└── sandbox/pathmap.rs        # reused: map_for / reverse for host->container; MountSet (mod.rs:488)
crates/micold-client/src/
├── features/mod.rs           # `Outcome` gains Insert{terminal,text} (defined here)
├── features/session.rs       # NEW msgs FilesDropped / ImagePasted / InsertionFailed; reducer emits Outcome::Insert
├── shell/subscriptions.rs    # NEW file-drop subscription (iced window::Event::FileDropped, :149 neighbour)
├── shell/clipboard.rs        # on_paste_requested (:94): empty text + AI session -> image path
├── shell/pasted_image.rs     # NEW  arboard read, png encode, write, exclude-file append (impure)
├── keymap.rs                 # paste_bytes (:286) reused unchanged
├── app.rs / shell/subscriptions.rs  # NEW pointer position state (CursorMoved subscription)
└── ui/material/split_view.rs # publish pane rects for drop hit-testing (FR-005)
crates/micold-daemon/src/state.rs # delete_session (:3295) removes pasted dir; startup sweep
docs/user-guide/terminal-panes.md, docs/user-guide/sandboxed-daemon.md
```

**Structure Decision**: logic in core (render-free, cross-platform tests), effects in client `shell/`,
cleanup in the daemon that owns session lifetime (existing core/client/daemon split).

## Requirement Map

| FR | Where |
|---|---|
| 001, 003, 004, 005 | `plan_insertion` joins with single spaces; client turns the text into `TerminalBytes` via `paste_bytes` (no newline, appended at the cursor); drop routed by pointer pane |
| 002 | `path_insert::quote` per `ShellKind` |
| 006, 007, 008 | `shell/pasted_image.rs` + `PastedLayout` (worktree dir, fallback temp dir, unique `<n>-<nanos>.png`, exclude line) |
| 009 | image branch only when `clipboard::read()` text is empty |
| 010, 011 | `plan_insertion` with `MountSet`: `pathmap::map`, refusal outside `projects` (canonicalised first) |
| 012 | `Outcome::Notify` on every `Err` / dead process / empty pane |
| 013, 014 | `PastedLayout::orphans` + daemon `delete_session` and start sweep |

## Test Strategy

- **Core unit** (`mise run test-core`): quoting tables per shell with ≥20 awkward names (SC-003); `plan_insertion` (order, refusals, symlink resolution, unrepresentable); layout/name uniqueness; `orphans`.
- **Real-shell round trip** (core integration test, unix `sh`/`bash`, skipped when absent; PowerShell on Windows CI): quoted text is run with `printf %s` / `echo` and compared to the file name (SC-003).
- **Client reducer** (`tests/features_session.rs` style): drop → `Outcome::Insert` bytes without `\r`/`\n` outside bracket (SC-004); target pane; exited process and empty pane → notice.
- **Client shell** (tempdir): image write, fallback on read-only worktree, exclude line idempotent, text-wins-over-image.
- **Daemon**: `delete_session` removes only the session's pasted dir, never a dropped file; start sweep.
- **Geometry gate**: drop target resolves to the pane under the pointer in a split layout (484).
- **Quickstart §B** visual pass: real drop and paste on a private Xvfb (see [quickstart.md](quickstart.md)).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| New `arboard` dependency in the client | iced 0.14 exposes only text clipboard reads | Shelling out to `wl-paste`/`xclip`/`osascript` is per-platform and fragile (research R3) |
