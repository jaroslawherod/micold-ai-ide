# Implementation Plan: macOS Package

**Branch**: `feat/package-for-macos` (spec dir `028-macos-package`) | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/028-macos-package/spec.md`

## Summary

Every release currently ships two `.deb` files and nothing else, while the constitution's
Distribution constraint requires builds for Linux, macOS, and Windows. macOS already builds and
tests green on every pull request — this feature is packaging, not porting.

The approach: compose a `micold-ai-ide.app` bundle from binaries cargo already produces, ship it
inside a drag-to-Applications `.dmg`, sign it ad-hoc, and attach it to the same draft release the
`.deb`s go to — so the existing `publish` job, which only runs once every artifact job succeeded,
becomes the mechanism that holds an incomplete release as a draft (FR-011). Two behaviours land in
tested Rust: refusing to run from the mounted image or a translocated path (FR-019), and attributing
a file-access failure to the macOS permission that caused it (FR-028). Everything else is a shell
script, a plist, two workflow edits, and documentation — each gated by a text-scan or shell test in
the pattern this repository already uses for the Debian package.

Windows packaging stays out of scope; this closes one of the two open platforms.

## Technical Context

**Language/Version**: Rust stable (`rust-toolchain.toml`), MSRV 1.97; POSIX `sh`/`bash` for the
packaging scripts; XML plist for the bundle's identity.

**Primary Dependencies**: No new Rust crates. Packaging uses only tools present on a stock macOS and
on the `macos-latest` runner: `lipo`, `codesign`, `hdiutil`, `plutil`, `sips`/`iconutil` (not needed
— `assets/icon/icon.icns` already exists). Deliberately **no** `cargo-bundle`, `cargo-packager`, or
`create-dmg` (research R1, R4).

**Storage**: Unchanged. `$HOME/.micold/` for state, `$HOME/.micold/run/d.sock` for the endpoint
(`micold-core::endpoint`). The bundle stores nothing inside itself.

**Testing**: `cargo test` (core unit tests for the two new render-free modules; a client text-scan
gate extending `tests/packaging_excludes_showcase.rs`), plus `scripts/tests/*.test.sh` for the
bundle script's composition rules — the same shell-suite slot `classify-change.sh` and
`check-assertions-frozen.sh` already occupy.

**Target Platform**: macOS 15 (Sequoia) and newer — the two most recent releases, a floor that moves
(FR-004). Both architectures: Apple silicon and Intel, in one universal binary.

**Project Type**: Desktop application, delivered as an OS-native package. This feature adds a
packaging pipeline, not an application subsystem.

**Performance Goals**: Not applicable to the artifact itself. The one budget that matters is CI
cost: the per-PR packaging gate (FR-036) must add no new build, only bundle composition and a launch
of binaries the macOS leg already compiled.

**Constraints**:
- The client finds the daemon as a **sibling of `current_exe()`** (`micold_core::spawn::daemon_binary`),
  so both binaries must sit in `Contents/MacOS/`. Getting this wrong makes every session fail to
  start while the app looks healthy.
- The showcase binary must never ship (FR-006); `cargo build` produces it in the same directory as
  the two that do, so the bundle script must never glob.
- The published release is immutable; the draft is not. Ordering — attach, then publish — is the
  whole of FR-011.
- No Apple Developer account, certificate, or repository secret may be required (FR-016).

**Scale/Scope**: ~2 new scripts, 1 plist template, 2 workflow edits, 2 new Rust modules (~200 LOC
with tests), 1 new user-guide page, 1 new developer page, 2 mise tasks.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design; the entries below are the
post-design state, and no gate changed verdict between the two passes.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. The two behavioural additions (`install_location`,
  `permission_failure`) are render-free `micold-core` modules with unit tests written first. The
  packaging *definition* is covered the way the Debian one is: a text-scan test that fails a manifest
  which would ship the showcase, driven by deliberately-broken synthetic inputs. The bundle script's
  layout rules are driven by `scripts/tests/macos-bundle.test.sh`, which runs on Linux against dummy
  binaries (research R3). The only code claiming the GUI/process-spawn exception is the iced view for
  the install-location screen, whose decision logic lives in the reducer.
- [x] **II. Multi-Session Support**: PASS. No new session state; sessions are unchanged. The bundle
  changes where the daemon binary lives, not how sessions are scoped.
- [x] **III. Worktree Integration**: PASS. Untouched.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing new leaves the device. The app
  performs no update check and contacts nothing; the user downloads the `.dmg` themselves.
- [x] **V. Rust + iced Stack**: PASS. New logic is Rust in `micold-core`; the new screen is iced,
  built from shared components. `InstallLocation` is an enum, so "translocated" and "installed"
  cannot both be true.
- [x] **VI. Cross-Platform Parity**: PASS, with a note. The two new core modules compile and are
  tested on all three platforms and return the benign answer off macOS, so no platform branches in
  application logic beyond the existing `cfg` boundary in `endpoint`/`keymap`. CI keeps building and
  testing all three. This feature *narrows* the standing parity gap (releases ship Linux-only)
  rather than widening it; Windows packaging remains open and is named as out of scope in the spec's
  Assumptions, not silently dropped.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/install-macos.md` and
  `docs/development/macos-packaging.md` ship in this change, both added to the `docs` job's existence
  checks, and README's Debian-only claim is corrected (FR-030..FR-033).
- [x] **VIII. Reusable UI Component Foundation**: PASS. The install-location screen reuses existing
  shared primitives; if it needs a new one, that primitive is promoted into the shared library with
  the chainable builder-into-`Element` API. No feature-local widget.

No entries in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/028-macos-package/
├── spec.md              # Feature specification (/speckit-specify, /speckit-clarify)
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── bundle-layout.md
│   ├── release-artifacts.md
│   └── install-location.md
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
packaging/
├── macos/
│   ├── Info.plist.in            # NEW — identity, version, floor, TCC purpose strings
│   └── entitlements.plist       # NEW — empty dict today; the seam FR-017 needs
├── micold-ai-ide.desktop        # unchanged (Linux)
└── sandbox/                     # unchanged

scripts/
├── macos-bundle.sh              # NEW — compose + (on macOS) ad-hoc sign the .app
├── macos-dmg.sh                 # NEW — stage, hdiutil, verify; release-only
└── tests/
    └── macos-bundle.test.sh     # NEW — the script's composition rules, runs on Linux

crates/micold-core/src/
├── install_location.rs          # NEW — where am I running from? (FR-019)
├── permission_failure.rs        # NEW — classify a denied file access (FR-028)
└── lib.rs                       # + two `pub mod` lines

crates/micold-client/
├── src/features/window.rs       # + the install-location verdict on the boot path
├── src/ui/                      # + the "install me first" screen (glue; logic in the reducer)
└── tests/packaging_excludes_showcase.rs   # widened to cover the macOS manifest

.github/workflows/
├── ci.yml                       # + a macOS-only step in the existing `test` job (FR-036)
└── release.yml                  # + a `macos` job; `publish` gains it in `needs:` (FR-011, FR-037)

docs/
├── user-guide/install-macos.md  # NEW (FR-030)
└── development/macos-packaging.md # NEW (FR-033)

mise.toml                        # + `app` and `dmg` tasks (FR-035)
README.md                        # releases are no longer Debian-only (FR-031)
```

**Structure Decision**: The feature follows the repository's existing packaging shape rather than
introducing one. Declarative packaging inputs live in `packaging/<platform>/` beside the Debian and
sandbox ones; the executable steps live in `scripts/` beside `build-lock.sh` and `classify-change.sh`
and are exposed through `mise` tasks, mirroring `mise run deb`. The two behavioural additions go in
`micold-core` because they are render-free decisions the client merely displays — the same split
that lets `spawn.rs` and `endpoint.rs` be tested without iced.

## Complexity Tracking

No constitutional violations to justify. The one judgement call worth recording is the deliberate
absence of a packaging *crate* or third-party bundler: see research R1.
