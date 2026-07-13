# Implementation Plan: Application Shell with Help / About

**Branch**: `001-app-shell-about` | **Date**: 2026-07-13 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-app-shell-about/spec.md`

## Summary

Bootstrap the Micold AI IDE UI shell: a single main window with a top toolbar whose only
entry is "Help", which reveals an "About" action that opens a modal About dialog showing
the app name, version, license, and one-line description, dismissible via a Close button or
Esc. This is also the project's first code, so it establishes the Rust + iced project
skeleton, the cross-platform CI, and the user-guide docs pipeline that all later features
inherit.

Technical approach: implement in Rust with iced using The Elm Architecture (State / Message
/ `update` / `view`). The About dialog is a modal **overlay layer within the single window**
(iced `stack`), not a separate OS window. Application identity is read from **Cargo package
metadata at compile time** (`env!("CARGO_PKG_VERSION")`, `..._DESCRIPTION`, `..._LICENSE`)
so the version is never hardcoded; the display name "Micold AI IDE" is a constant (Cargo
package names cannot contain spaces). All display logic and state transitions live in a
render-free core that is unit-testable with `cargo test`, keeping Principle I (Test-First)
enforceable without a GUI-driving harness.

## Technical Context

**Language/Version**: Rust, stable toolchain (latest stable at implementation time; added to `mise.toml`)

**Primary Dependencies**: iced (latest stable 0.13+ line, pinned in `Cargo.toml`) — the only GUI framework permitted by the constitution

**Storage**: N/A for this feature — no persistent state is introduced. Application identity is compile-time metadata; nothing is read from or written to disk at runtime.

**Testing**: `cargo test` — inline unit tests for the render-free core (metadata resolution, overlay state transitions via `update`) plus integration tests in `tests/` driving the `update` function

**Target Platform**: Desktop — Linux, macOS, Windows (feature parity required)

**Project Type**: Desktop application (GUI)

**Performance Goals**: Cold start to interactive main window in under 2 seconds on commodity hardware; UI interactions (open/close About) feel instant (<100 ms perceived)

**Constraints**: Fully offline / local-first — no network access of any kind; no OS-conditional branching in core logic; single window only

**Scale/Scope**: One main window, one toolbar entry ("Help"), one menu action ("About"), one modal dialog. Deliberately minimal — this is the shell bootstrap.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS — display/version/license/description resolution and overlay open/close transitions are implemented in a render-free core; failing `cargo test` unit + integration tests are written and reviewed before implementation. Rendering itself is validated manually via `quickstart.md` + CI build.
- [x] **II. Multi-Session Support**: PASS (not applicable) — this feature introduces **no session state**. Sessions are explicitly out of scope (spec Assumptions). The shell is structured so future session UI can mount inside the main window without reworking the overlay pattern; nothing here is per-single-session-global in a way that would block later isolation.
- [x] **III. Worktree Integration**: PASS (not applicable) — no file or version-control operations are performed by this feature. Nothing introduced conflicts with future worktree-aware operations.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS — zero network use; all displayed data comes from compile-time package metadata; the app is fully functional offline.
- [x] **V. Rust + iced Stack**: PASS — implemented in Rust with iced only. Overlay state is modeled as an `enum Overlay { None, About }` so "About dialog is open twice" is unrepresentable (satisfies FR-015 at the type level).
- [x] **VI. Cross-Platform Parity**: PASS — single window + in-window overlay with no OS-specific code paths; keyboard (Esc) handled via iced's cross-platform keyboard subscription. CI builds and tests on Linux, macOS, and Windows (new `.github/workflows/ci.yml`).
- [x] **VII. Documentation First-Class**: PASS — a user-guide page for the Help/About flow ships in the same change under `docs/user-guide/`, and the docs check runs in CI.

**Result**: All gates PASS. No violations → Complexity Tracking left empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-app-shell-about/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── ui-contract.md   # Toolbar/menu/dialog interaction + metadata display contract
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml               # package metadata (name, version, license, description) + iced dep
mise.toml                # add Rust stable toolchain (currently only declares uv)
LICENSE                  # Apache-2.0 license file (owner-confirmed; see research.md R9)

src/
├── main.rs              # entry point; launches the iced application
├── app.rs               # root State, Message, update(), view() — The Elm Architecture
├── metadata.rs          # AppMetadata: reads compile-time env vars + fallback rules (FR-016)
└── ui/
    ├── mod.rs
    ├── toolbar.rs       # top toolbar + "Help" menu exposing only "About"
    └── about.rs         # About modal overlay (name/version/license/description, Close)

tests/
├── metadata.rs          # unit: version/license/description resolution + empty-value fallback
└── about_flow.rs        # integration: Help→About→Close/Esc state transitions via update()

docs/
└── user-guide/
    └── help-about.md    # user-facing documentation for the Help/About flow (Principle VII)

.github/
└── workflows/
    └── ci.yml           # build + test matrix on Linux/macOS/Windows; docs check
```

**Structure Decision**: Single-project desktop application (Option 1). All code lives under
`src/` with a small `ui/` submodule; the render-free core (`metadata.rs`, the `update` logic
in `app.rs`) is separated from `view()` so it is unit-testable without a running GUI.
Because this is the repository's first feature, the plan also bootstraps `Cargo.toml`, the
Rust toolchain in `mise.toml`, the CI matrix, and the docs tree — these are one-time
foundations that subsequent features reuse rather than recreate.

## Complexity Tracking

> No constitution violations. Section intentionally empty.
