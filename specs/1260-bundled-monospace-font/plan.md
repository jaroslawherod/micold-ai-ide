# Implementation Plan: Bundled monospace typeface

**Branch**: `claude/project-thread-c0bgc0` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1260-bundled-monospace-font/spec.md`

## Summary

Ship Fira Mono 3.206 (Regular and Bold) in `assets/fonts/`, moving the test fixture there rather
than adding a second copy. At startup, one shared function loads both faces into iced's global font
system, removes any other face named `Fira Mono` (a host copy) and points the database's monospace
family at the bundled face, so every `Font::MONOSPACE` call site — code blocks, inline code, diffs,
tool output and the terminal — draws it on every host without a call-site change. The layout tests
call the same function, so they measure what the application registers. Every release bundle gains
the font licence files, which today none of them carry.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`advanced` feature: `iced::advanced::graphics::text::{font_system,
FontSystem}` and the re-exported `cosmic_text::fontdb` 0.23); `ttf-parser` (already a client
dependency) for the bytes guard; `tempfile` (already a dev-dependency) for the same-name test. No new
crate.

**Storage**: none (compile-time assets; one in-process registration)

**Testing**: `cargo test` via `mise run gate`: in-crate unit tests in `ui/material/mono.rs` and
`ui/terminal.rs`; client integration tests in `crates/micold-client/tests/` (layout apparatus guard,
layout snapshot, one-copy test, packaging test); `scripts/tests/macos-bundle.test.sh`; quickstart §B
visual pass and §C for the platform bundles

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace); touches `micold-client`, packaging, assets,
docs

**Performance Goals**: no measurable startup cost: two in-memory faces parsed once (≈380 KB), one
linear pass over the font database

**Constraints**: registration before the first text is shaped (cosmic-text caches font matches,
research R2); interface text unchanged (FR-009, SC-005); system fallback kept (FR-004)

**Scale/Scope**: 1 new module, 4 one-line call sites of the installer, 1 font file added and 1 moved,
4 packaging manifests, 2 provenance files, 2 user-guide/dev-doc pages

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | The decision logic is `pin_monospace` (which faces go, what the family becomes), unit-tested red first on a bare `fontdb::Database` holding a same-name file face. The bytes guard, the layout guard (now through the shipped installer), the one-copy test, the terminal cell tests and the packaging test are written failing first. `install_monospace` and its four call sites are glue (three calls, no branch), covered by the layout guard, which goes through it, and quickstart §B. The application call site, `shell/startup.rs`, is the body of the binary's `main` (moved there by feature 021, FR-019a; `mod shell` is declared in `src/main.rs`), so it is binary wiring under the exception, beside the Roboto `.font(..)` registration it sits next to. |
| II. Multi-Session | PASS | One process-wide registration, no per-session font state; every session view and window shares it (spec edge case; quickstart B6). |
| III. Worktree Integration | PASS | No file or VCS operation. |
| IV. Local-First | PASS | Fonts are compiled in; nothing is fetched at run time. The upstream download happened once at planning (research R1) and is pinned by hash. |
| V. Rust + iced | PASS | iced's own font system and its `fontdb`; no new GUI or font stack. |
| VI. Cross-Platform | PASS | The same bytes and the same database steps on all three platforms, no OS branch; the layout snapshot runs on all three CI runners with no per-platform fixture (SC-002); every platform's bundle gains the licences (§C). |
| VII. Documentation | PASS | `docs/user-guide/appearance-theming.md` *The typeface* (FR-010), `assets/fonts/PROVENANCE.md`, `tests/fixtures/FONT-PROVENANCE.md`, `docs/development/layout-snapshot.md`, all in the same PR. |
| VIII. Components | PASS | No widget is added or changed; the face is reached through the existing `Font::MONOSPACE` the shared components (`DiffView`, `CodeBlock`, `Text::monospace`, `LinkText`) already use. |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### D1 — The face and its provenance — research R1, R5; FR-001, FR-005, FR-008

- `git mv crates/micold-client/tests/fixtures/FiraMono-Regular.ttf assets/fonts/`, and
  `git mv` its `LICENSE-FiraMono-OFL.txt` beside it. Add `assets/fonts/FiraMono-Bold.ttf` from the
  pinned upstream commit; verify both hashes against research R1.
- `assets/fonts/PROVENANCE.md`: the table grows to three typefaces (Fira Mono, two files, OFL 1.1,
  `LICENSE-FiraMono-OFL.txt`), and a *Fira Mono* section records upstream, commit, version and each
  file's SHA-256 (moved from `tests/fixtures/FONT-PROVENANCE.md`).
- `tests/fixtures/FONT-PROVENANCE.md`: the monospace face joins the "faces the application ships"
  table; the "for the tests only" table and framing go; the feature-940 section is kept as history
  and ends by pointing at feature 1260.

### D2 — Registration — research R2, R3, R6, R8; FR-001–FR-004, FR-011; contract C-1–C-3

- New `ui/material/mono.rs`: `MONO_FAMILY`, `FIRA_MONO_REGULAR_BYTES`, `FIRA_MONO_BOLD_BYTES`,
  `pin_monospace`, `install_monospace` (contract). Re-exported from `ui::material` and `ui`.
- Call `install_monospace(&mut font_system().write()…)` once in `shell/startup.rs` before
  `iced::application(...)`, and in `showcase/main.rs`. The comment at the Roboto registration notes
  why the monospace face is registered differently (research R2, alternative 2).
- `material/test_support.rs::renderer`: add the call inside its `Once`, so in-crate component tests
  measure the bundled face too.

### D3 — Tests measure the shipped face — research R4; FR-005, FR-006, SC-002, SC-003

- `tests/support/layout.rs`: `renderer()` replaces its fixture `load_font` and
  `set_monospace_family` with `micold_client::ui::install_monospace`; `REFERENCE_MONO_FONT_BYTES`
  and `REFERENCE_MONO_FONT_FAMILY` become the shipped constants (doc comment rewritten: shipped,
  not test-only).
- `tests/layout_apparatus.rs`: `the_monospace_face_parses_and_is_the_face_that_measures` keeps its
  checks and adds the Bold face (`Weight::Bold` resolves to the Bold bytes, its probe width equals
  the Bold face's advances), with failure messages naming `Fira Mono`.
- New one-copy test (in `tests/layout_apparatus.rs`, beside the guard): walk the repository from
  `CARGO_MANIFEST_DIR/../..`, skipping `target*`, `.git` and `.claude/worktrees`, and fail naming each
  `FiraMono*` font file outside `assets/fonts/`.
- Run the layout snapshot; expect it unchanged (research R4). If not, regenerate once and review.

### D4 — Terminal (isolated; FR-007, pending the user's confirmation) — research R7

Under the clarified spec (adopt) this step is **tests and one doc sentence**: `cell_font` already
starts from `Font::MONOSPACE`, which D2 points at the bundled face.

- No change to `terminal.rs::cell_font` or `CellMetrics`.
- In-crate tests in `ui/terminal.rs` (using `test_support::renderer`): `cell_font(Flags::empty())`
  and `cell_font(Flags::BOLD)` resolve to the bundled Regular and Bold faces; `CellMetrics::new(s)
  .width` equals the bundled face's advance at `s` for 12, 13 and 14 px.
- D6's terminal sentence and quickstart B7.

**If the user declines FR-007** (the terminal keeps the host's monospaced font), D4 is replaced by
the list below. D1–D3 and D5 do not change, and neither do `mono.rs` or its contract C-1–C-3.

1. `terminal.rs`: a constant `TERMINAL_FONT: Font = Font::with_name("Noto Sans Mono")` (the family
   `Family::Monospace` named before this feature: cosmic-text's default, `font/system.rs:158`), and
   `cell_font` starts from it instead of `Font::MONOSPACE`. Test first: `cell_font(Flags::empty())`
   is not the bundled family, and bold still sets `Weight::Bold`.
2. The D4 tests above are not written; the cell-width test is dropped (the host face only
   approximates 0.6 em, as today).
3. D6: the user guide keeps "the terminal keeps its monospaced font" and says code elsewhere uses
   Fira Mono.
4. Quickstart B7 expects the host's monospaced font in the terminal.
5. Spec: FR-007 and FR-010 revert to the host font for the terminal; the clarification, the
   *terminal pane* edge case and the *Code-like text* entity drop the terminal.
6. Contract C-4 and data-model *Code-like text* drop `terminal.rs::cell_font` from the call sites.

### D5 — Licences in every bundle — research R5; FR-008, SC-004

- `crates/micold-client/Cargo.toml`: `[package.metadata.deb] assets` and
  `[package.metadata.generate-rpm] assets` gain the four files of the font licence set.
- `scripts/macos-bundle.sh`: copy them to `Contents/Resources/licenses/fonts/`, failing by name when
  one is missing (its existing `missing icon:` pattern).
- `packaging/windows/micold-ai-ide.iss` `[Files]`: four explicit lines to `{app}\licenses\fonts`
  (no wildcard, as the file's comment requires).
- `.gitattributes`: `/assets/fonts/LICENSE*` and `/assets/fonts/PROVENANCE.md` join
  `micold-packaging`.
- `tests/packaging_excludes_showcase.rs`: its Windows rule rejects any `[Files]` `Source:` whose
  basename is outside `WINDOWS_SHIPPED` (`windows_violations`, line 512), so the four licence lines
  would turn `the_windows_installer_contains_no_showcase` red. Add a separate allow-set
  `WINDOWS_SHIPPED_LICENCES` (the licence-set basenames, accepted only with a `DestDir` under
  `{app}\licenses\`; new helper `iss_file_entries` returning `(source, dest_dir)` pairs, since
  `iss_file_sources` reads `Source:` only, and `windows_violations` gains the licence allow-set as a
  parameter), keep the wildcard and showcase rules as they are, and add the red-first
  synthetic case beside `a_windows_installer_that_ships_an_unlisted_file_fails` (an unlisted file
  still fails; a licence file outside `{app}\licenses\` fails). `macos-bundle.sh` names each licence
  file (no wildcard), so the macOS rule (`macos_violations`: wildcards into `Contents/MacOS/`) is
  unaffected.
- Tests: extract `deb_assets`/`rpm_assets`/`repo_root` from `tests/packaging_excludes_showcase.rs`
  into `tests/support/packaging.rs`; new `tests/packaging_ships_font_licences.rs` asserts each
  licence-set file is in the deb assets, the rpm assets, the `.iss` `[Files]` and `macos-bundle.sh`,
  and exists on disk. `scripts/tests/macos-bundle.test.sh` asserts the staged bundle has the files and
  adds an `expect_failure` for a missing licence.

### D6 — Docs — FR-010

- `docs/user-guide/appearance-theming.md` *The typeface*: the application also ships **Fira Mono**
  for code, diffs, tool output and the terminal; the terminal is exempt from Roboto and the type
  scale but no longer uses the host's monospaced font; uncovered characters still fall back.
- `docs/development/layout-snapshot.md` (line 183): code measures the shipped Fira Mono.
- `docs/user-guide/worktrees-and-sessions.md` line 1125 ("a monospace font"): name the bundled face.

## Requirement map

| Requirement | Design | Test |
|---|---|---|
| FR-001 | D1, D2 | bytes guard (`mono.rs`) |
| FR-002 | D2 | layout guard (Regular), quickstart B1–B2 |
| FR-003 | D2 (`pin_monospace`) | `mono.rs` same-name test; layout guard source check; B4 |
| FR-004 | D2 (only `Fira Mono` faces removed) | `mono.rs` other-faces-kept test; B5 |
| FR-005 | D1, D3 | one-copy test; layout guard through the shipped installer |
| FR-006 | D3 | layout guard messages name `Fira Mono` |
| FR-007 | D4 | terminal cell tests; B7 |
| FR-008 | D1, D5 | packaging licence test; Windows allow-set cases in `packaging_excludes_showcase.rs`; `macos-bundle.test.sh`; §A hashes; §C |
| FR-009 | D2 (default font untouched) | layout snapshot (non-code nodes unchanged); B8 |
| FR-010 | D6 | `check-user-guide-updated.sh` in the gate; review |
| FR-011 | D1 (Bold face), D2 | `mono.rs` Bold query test; layout guard Bold; B3 |
| SC-001, SC-002 | D2, D3 | layout snapshot on three CI runners |
| SC-003 | D1, D3 | one-copy test |
| SC-004 | D5 | packaging test; §C |
| SC-005 | — | layout snapshot diff |

## Test strategy by layer

| Layer | Tests |
|---|---|
| Client in-crate unit (`ui/material/mono.rs`) | bytes guard: both faces parse, family `Fira Mono`, weights 400/700, fixed-pitch, 600-unit ASCII advance; `pin_monospace` on a bare database: a same-name `Source::File` face (a temp copy of the Regular bytes) is removed and `Family::Monospace` resolves to the `Binary` Regular; weight 700 → Bold; italic → upright; faces of other families kept; second call returns 0 |
| Client in-crate unit (`ui/terminal.rs`) | D4: cell fonts resolve to bundled Regular/Bold; cell width equals the face's advance |
| Client integration (`tests/`) | layout apparatus guard (Regular and Bold, through `install_monospace`); one-copy test; layout snapshot (geometry gate, all three runners); packaging licence test; `packaging_excludes_showcase.rs` Windows licence allow-set cases |
| Script tests | `scripts/tests/macos-bundle.test.sh` (licences staged; missing licence named) |
| Quickstart §B visual pass | B1–B8 on Linux: the glue (`startup.rs`, `showcase/main.rs`) and the look |
| Quickstart §C | macOS and Windows look, and bundle contents per platform |

## Project Structure

### Documentation (this feature)

```text
specs/1260-bundled-monospace-font/
├── spec.md
├── plan.md              # this file
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/fonts.md
├── autopilot.md         # ledger
└── tasks.md             # next unit
```

### Source Code (repository root)

```text
assets/fonts/
├── FiraMono-Regular.ttf        # moved from crates/micold-client/tests/fixtures/
├── FiraMono-Bold.ttf           # new
├── LICENSE-FiraMono-OFL.txt    # moved
└── PROVENANCE.md               # Fira Mono section
crates/micold-client/
├── Cargo.toml                  # deb/rpm assets: font licence set
├── src/ui/material/mono.rs     # new (contract)
├── src/ui/material/mod.rs      # re-export
├── src/ui/mod.rs               # re-export
├── src/ui/material/test_support.rs
├── src/ui/terminal.rs          # D4 tests only
├── src/shell/startup.rs
├── src/showcase/main.rs
└── tests/
    ├── support/layout.rs
    ├── support/packaging.rs    # new, extracted
    ├── layout_apparatus.rs
    ├── packaging_excludes_showcase.rs
    ├── packaging_ships_font_licences.rs   # new
    └── fixtures/FONT-PROVENANCE.md
packaging/windows/micold-ai-ide.iss
scripts/macos-bundle.sh
scripts/tests/macos-bundle.test.sh
.gitattributes
docs/user-guide/appearance-theming.md
docs/user-guide/worktrees-and-sessions.md
docs/development/layout-snapshot.md
```

**Structure Decision**: the existing workspace; client-only code, beside the other embedded faces in
`ui/material/`.

## Complexity Tracking

None.
