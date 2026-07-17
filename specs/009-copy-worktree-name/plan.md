# Implementation Plan: Copy Worktree Name to Clipboard

**Branch**: `fix/copy-paste-for-all-inputs` | **Date**: 2026-07-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/009-copy-worktree-name/spec.md`

**Note**: This plan was written after implementation, retrofitting the change into the spec-kit
process (see the process note in `spec.md`). It documents the design actually built, including
the investigation that ruled out a broader change.

## Summary

Investigation showed the reported "copy/paste for all inputs" gap was narrower than it first
sounded: iced's `text_input` already handles native OS copy/paste/cut/select-all
(Ctrl+C/V/X/A) for every editable field (rename dialogs, worktree-creation form, settings), and
the embedded terminal already has its own Ctrl+Shift+C/V + right-click Copy/Paste. The real gap
was **read-only labels** — worktree names — which iced's plain `text` widget cannot select or
copy. The fix adds a **Copy name** entry to the worktree's existing right-click context menu that
writes the row's displayed name to the system clipboard via a new, deliberately generic
`Message::TextCopyRequested(String)`, reusing the same clipboard-write path the terminal's own
Copy action already uses. Shipping the message as generic (not worktree-specific) means the same
one-line addition can cover session titles or project names later without a new message variant.
Because the app's icon font is a subsetted embed containing only the glyphs currently used
(`assets/fonts/PROVENANCE.md`), adding a "copy" icon required regenerating that subset with one
additional codepoint (`content_copy`, verified against the upstream Material Symbols manifest).

## Technical Context

**Language/Version**: Rust, edition 2021, rust-version 1.80 (stable, via `mise`)

**Primary Dependencies**: `iced` 0.13 (existing; `advanced::Clipboard` already used by the
terminal pane). No new Rust crate dependencies. `fonttools` (via `uvx`) was used one-time,
offline of the build, to regenerate the embedded icon font asset — it is not a build or runtime
dependency of the crate.

**Storage**: N/A — no persisted state added or changed.

**Testing**: `cargo test --no-default-features --all-targets` (icon/glyph regression tests,
matching CI) and `cargo test --features gui` (binary-only compile/test parity). `cargo clippy
--features gui --all-targets` for lints.

**Target Platform**: Desktop — Linux, macOS, Windows.

**Project Type**: Desktop application (Rust + iced), single project.

**Performance Goals**: N/A — a single discrete clipboard write on user action, not a
continuous/perf-sensitive path.

**Constraints**: The shipped icon font is a curated subset (research/PROVENANCE.md convention);
any new icon requires regenerating it with the additional codepoint, verified by the existing
font-integrity test so a missing glyph ("tofu") can never reach the running UI.

**Scale/Scope**: 1 new `Icon` variant, 1 new `Message` variant, 1 new context-menu entry, 1
regenerated font asset. 7 files touched (`src/icons.rs`, `src/app.rs`, `src/main.rs`,
`src/ui/mod.rs`, `tests/icons.rs`, `assets/fonts/MaterialSymbolsOutlined.ttf`,
`assets/fonts/PROVENANCE.md`) plus one user-guide doc.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: The pure reducer's no-op handling of
  `Message::TextCopyRequested` is covered by
  `tests/sidebar_state.rs::text_copy_requested_is_a_no_op_in_the_pure_reducer` (added during a
  convergence pass — see tasks.md T013/T014 — after an initial draft of this plan incorrectly
  grouped the reducer arm with the untestable GUI glue below; `src/app.rs` compiles under
  `--no-default-features` with no gui gate, so this arm was always headlessly testable, unlike
  the view-construction code). The font-glyph change genuinely went red→green (the
  `icons_font::every_icon_codepoint_has_a_glyph` test was observed failing against the
  unmodified font before the subset was regenerated, then passing after). What remains
  untested-before-implementation is narrower: the context-menu wiring and the binary's
  clipboard-write handler — like the sibling `TerminalCopyRequested`/`TerminalPasteRequested`
  messages and the existing Rename/Delete menu items, these are GUI-only presentation glue with
  no meaningful headless unit surface (private view-construction functions under `src/ui/` and
  binary-only orchestration in `src/main.rs`, which only compile under `cargo test --features
  gui`, not the `tests/` integration dir). This narrower gap matches established precedent in
  this codebase (see feature 007's plan.md) rather than a new exception; it remains a recorded,
  justified deviation — see Complexity Tracking.
- [x] **II. Multi-Session Support**: N/A — no session state is touched; the clipboard write is
  a stateless, app-global side effect.
- [x] **III. Worktree Integration**: N/A — no filesystem or VCS operation is added; the
  worktree's already-computed displayed name is read, not written.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS — the system clipboard is local to the
  machine; nothing is transmitted off-device; no persistence change.
- [x] **V. Rust + iced Stack**: PASS — uses iced's existing `advanced::Clipboard` /
  `iced::clipboard::write`; no alternative framework introduced.
- [x] **VI. Cross-Platform Parity**: PASS — `iced::clipboard::write` is the same cross-platform
  API the terminal's Copy action already relies on; no OS branching added.
- [x] **VII. Documentation First-Class**: PASS — `docs/user-guide/worktrees-and-sessions.md`
  documents **Copy name** in the same change.
- [x] **VIII. Reusable UI Component Foundation**: PASS — reuses the existing `MenuItem` builder
  (no one-off widget), the existing `Icon` enum/glyph mechanism, and the existing
  clipboard-write path already proven by `TerminalCopyRequested`; the new message is
  deliberately generic rather than worktree-specific so it is itself a reusable primitive for
  future labels (session titles, project names).

**Result**: All gates PASS. Complexity Tracking is empty.

**Post-Phase-1 re-check**: The design artifacts (research.md, data-model.md,
contracts/clipboard-copy.md, quickstart.md) introduce no further violations. The generic message
shape (R5 in research.md) and the honest documentation of its one side effect
(contracts/clipboard-copy.md) keep the reuse story (Principle VIII) intact, and the pure
reducer's no-op handling of `TextCopyRequested` is now test-covered (Principle I). Gates still
PASS.

## Project Structure

### Documentation (this feature)

```text
specs/009-copy-worktree-name/
├── plan.md                   # This file
├── research.md                # Phase 0 output
├── data-model.md              # Phase 1 output
├── quickstart.md              # Phase 1 output
├── contracts/
│   └── clipboard-copy.md      # Phase 1 output — the TextCopyRequested contract
├── checklists/
│   └── requirements.md        # From /speckit-specify
└── tasks.md                   # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── icons.rs                   # + Icon::Copy variant, glyph, ALL entry
├── app.rs                     # + Message::TextCopyRequested(String); no-op in the pure reducer
├── main.rs                    # Binary handler: iced::clipboard::write + dismiss the worktree menu
└── ui/
    └── mod.rs                 # worktree_menu_items(dir, display_name) gains a "Copy name" entry

tests/
├── icons.rs                    # Icon::ALL count + codepoint mapping updated for Icon::Copy
└── sidebar_state.rs            # + no-op reducer test for Message::TextCopyRequested

assets/fonts/
├── MaterialSymbolsOutlined.ttf  # Regenerated subset (+ content_copy / U+E14D)
└── PROVENANCE.md                # Subset command + mapping table updated

docs/user-guide/
└── worktrees-and-sessions.md    # + "Copy name" entry in the right-click menu section
```

**Structure Decision**: Single project, matching the existing layout. No new module or
directory is introduced; the change extends existing enums (`Icon`, `Message`) and an existing
view function (`worktree_menu_items`), consistent with how Rename/Delete were added in feature
008.

## Complexity Tracking

> No constitution violations. Section intentionally empty — see the Constitution Check's
> Principle I note for the (non-violating) genuinely-untestable GUI/binary glue this feature
> still leaves unit-test-free, matching feature 007's own precedent for the same category of
> code.
