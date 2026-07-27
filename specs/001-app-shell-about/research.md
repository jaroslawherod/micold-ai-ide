# Phase 0 Research: Application Shell with Help / About

All Technical Context unknowns are resolved below. No `NEEDS CLARIFICATION` markers remain.

## R1. GUI framework & architecture

- **Decision**: iced, using The Elm Architecture (TEA): a `State`, a `Message` enum, an
  `update(&mut State, Message)` reducer, and a `view(&State)` function. Pin the latest stable
  iced release (0.13+ line) in `Cargo.toml`.
- **Rationale**: The constitution mandates iced and forbids any other GUI framework
  (Principle V). TEA cleanly separates state transitions (`update`) from rendering (`view`),
  which is what makes the core unit-testable under `cargo test` and lets Test-First
  (Principle I) apply without a GUI-driving harness.
- **Alternatives considered**: None permitted — Principle V forecloses other frameworks.
- **Follow-up at implementation**: pin the exact current stable iced version and confirm the
  `application()` builder / `stack` overlay APIs against that release's docs.

## R2. Reading application identity (name, version, license, description)

- **Decision**: Read version, description, and license from **Cargo package metadata at
  compile time** via the `env!` macro; supply the display name as a `const`.
  - `version` → `env!("CARGO_PKG_VERSION")`
  - `description` → `env!("CARGO_PKG_DESCRIPTION")` (populated from `Cargo.toml` `description`)
  - `license` → `env!("CARGO_PKG_LICENSE")` (populated from `Cargo.toml` `license`)
  - `name` → a `const APP_NAME: &str = "Micold AI IDE"` (Cargo package names cannot contain
    spaces, so `CARGO_PKG_NAME` = `micold-ai-ide` is unsuitable for display; FR-006 requires
    the exact string "Micold AI IDE").
- **Rationale**: Cargo injects these env vars for every build, so the version is sourced from
  the single source of truth (`Cargo.toml`) and can never drift from the packaged release
  (FR-007, SC-003). No runtime file reads → stays fully offline (Principle IV).
- **Alternatives considered**:
  - Hardcoding the version → rejected, violates FR-007 and risks stale releases (SC-003).
  - A build script emitting a generated constants file → rejected as unnecessary; the `env!`
    macros already expose exactly what is needed.
  - Reading `Cargo.toml` at runtime → rejected; not present in a shipped binary and would add
    a filesystem dependency.

## R3. Fallback when metadata is empty (FR-016)

- **Decision**: `env!("CARGO_PKG_VERSION")` is always non-empty for a Cargo build, but
  `description` and `license` are the empty string when unset in `Cargo.toml`. The
  render-free core treats an empty metadata string as "unavailable" and substitutes a
  clearly-labeled fallback (e.g., `"unknown"`) before display.
- **Rationale**: Guarantees the dialog never shows a blank field or a raw placeholder
  (FR-016) while keeping the check trivial and testable (a pure function over `&str`).
- **Alternatives considered**: `option_env!` — rejected; `CARGO_PKG_*` are always defined
  (as possibly-empty strings), so an explicit is-empty check is clearer than `Option`.

## R4. Modal dialog as an in-window overlay (FR-013)

- **Decision**: Render the About dialog as an overlay layer stacked on top of the main
  content within the **same** window (iced `stack` widget with a dimmed backdrop), not a
  second OS/winit window. While open, the backdrop intercepts input so the toolbar and main
  content are non-interactive.
- **Rationale**: FR-013 requires a modal overlay inside the main window; a single-window
  overlay is also identical across platforms (Principle VI) and is the reusable dialog
  pattern later features will adopt.
- **Alternatives considered**: A separate OS window — rejected; violates FR-013 and
  introduces platform-specific window-management differences.

## R5. Keyboard (Esc) and focus handling (FR-011, FR-014)

- **Decision**: Subscribe to key presses via iced's keyboard subscription; map Esc to the
  `AboutClosed` message **only while the overlay is open**. No focus `Task`/operation is used
  on open or close.
- **Rationale**: Cross-platform keyboard handling comes from iced's abstraction (Principle
  VI); gating Esc on overlay state satisfies the "Esc with no dialog open has no effect" edge
  case. Focus movement was dropped 2026-07-27 (see spec.md alignment note): iced 0.13.4's
  `button` widget does not implement the framework's focusable operation (only `text_input`/
  `text_editor` do), so there is no supported way to move focus onto the Close button.
- **Alternatives considered**: Global OS hotkey — rejected; unnecessary and platform-specific.
  A custom `Operation`-based focusable wrapper around the Close button — rejected for this
  single-button dialog as disproportionate complexity; revisit if iced adds native button
  focus support or the dialog grows more interactive controls.

## R6. Testing strategy under Principle I

- **Decision**: Put all decision logic in a render-free core: `metadata.rs` (pure functions
  over metadata strings) and the `update` reducer + `Overlay` state in `app.rs`. Write failing
  `cargo test` tests first: unit tests for metadata resolution/fallback, integration tests in
  `tests/` that drive `update` through Help→About→Close and Help→About→Esc and assert the
  resulting `Overlay` state. Rendering (`view`) is validated manually via `quickstart.md` and
  proven to compile/run on all platforms by CI.
- **Rationale**: iced has no first-class UI-driving test harness; isolating logic from
  rendering is the established way to keep GUI apps genuinely test-driven. Meets Principle I
  without over-investing in brittle pixel/e2e automation.
- **Alternatives considered**: Full GUI automation (e.g., driving rendered widgets) —
  rejected as brittle and not portable across the three CI platforms for a bootstrap feature.

## R7. Cross-platform build & CI (Principle VI)

- **Decision**: Add a GitHub Actions matrix (`ubuntu-latest`, `macos-latest`,
  `windows-latest`) running `cargo build` + `cargo test`, plus a docs check. Keep all core
  logic OS-agnostic; no `cfg(target_os = ...)` branching is needed for this feature.
- **Rationale**: Operationalizes the constitution's cross-platform gate (Principle VI) and
  TDD gate (Principle I) on every change.
- **Alternatives considered**: Single-platform CI — rejected; violates Principle VI's
  "CI MUST build and test on all three platforms".

## R8. Rust toolchain in mise (constitution follow-up)

- **Decision**: Add the Rust stable toolchain to `mise.toml` (currently declares only `uv`).
- **Rationale**: Directly closes the constitution's recorded follow-up TODO ("mise.toml
  declares uv only; add the Rust stable toolchain") and satisfies the Technology Constraints
  ("Rust, stable toolchain, managed via mise").
- **Alternatives considered**: Leaving toolchain unmanaged — rejected; violates the
  constraint that the toolchain is managed via `mise`.

## R9. License value shown in the About dialog (prerequisite)

- **Decision (mechanism)**: The dialog displays whatever the `Cargo.toml` `license` field
  resolves to (via `env!("CARGO_PKG_LICENSE")`), and a matching `LICENSE` file lives at the
  repo root. This is a **data/governance prerequisite**, not a technical unknown: the plan is
  complete regardless of which OSI license is chosen.
- **Decision (value)**: **`Apache-2.0`** — confirmed by the project owner. OSI-approved,
  includes an explicit patent grant, widely used across the Rust ecosystem. Set
  `Cargo.toml` `license = "Apache-2.0"` (SPDX) and ship the full Apache License 2.0 text as
  the root `LICENSE` file; the About dialog reads it via `env!("CARGO_PKG_LICENSE")`.
- **Alternatives considered**: dual `MIT OR Apache-2.0` (Rust-idiomatic) and `MIT` alone
  (simplest permissive) — not chosen; owner selected single `Apache-2.0`.
- **Status**: RESOLVED. Closes the constitution follow-up TODO ("choose and add the
  OSI-approved LICENSE file"). No longer a prerequisite blocker.

## Resolved unknowns summary

| Topic | Resolution |
|-------|------------|
| GUI framework / architecture | iced + TEA, latest stable pinned (R1) |
| Identity source | Compile-time `CARGO_PKG_*` env vars + `APP_NAME` const (R2) |
| Empty-metadata fallback | Is-empty check → `"unknown"` label (R3) |
| Modal mechanism | In-window `stack` overlay, single window (R4) |
| Esc / focus | Overlay-gated keyboard subscription + focus Task (R5) |
| Testing | Render-free core; `cargo test` unit + integration; manual render validation (R6) |
| CI | GitHub Actions matrix on Linux/macOS/Windows + docs check (R7) |
| Toolchain | Add Rust stable to `mise.toml` (R8) |
| License value | Apache-2.0 (SPDX), owner-confirmed; `Cargo.toml` license + root LICENSE (R9) |
