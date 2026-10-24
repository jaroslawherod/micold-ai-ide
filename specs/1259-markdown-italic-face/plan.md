# Implementation Plan: Markdown italic draws in a true italic face

**Branch**: `claude/project-thread-wxdcto` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1259-markdown-italic-face/spec.md`

## Summary

Ship three static Roboto italic instances (400, 500, 600) cut from the same Google Fonts upstream
as the upright faces. A new `ui/material/roboto.rs`, the twin of feature 1260's `mono.rs`, lists the
six Roboto faces once, loads them into iced's global font system and removes any host italic
`Roboto` face, so an italic query can only land on the bundled bytes; the application, the showcase
and both test renderers call it instead of keeping their own lists. `MarkdownView`'s existing italic
mapping (`link_text::painted`) is kept and now finds a face; code runs inside emphasis stop asking
for a slant. The shared markdown fixture gains a bold-italic run, so the showcase shows it and core
asserts its style.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`advanced`: `iced::advanced::graphics::text::{font_system,
FontSystem}`, re-exported `cosmic_text::fontdb`, cosmic-text 0.15); `ttf-parser` (client dependency)
for the face guard; `tempfile` (dev-dependency) for the host-copy test. No new crate. Font cutting
at implementation uses `uv run --with fonttools` (not a build dependency).

**Storage**: none (compile-time assets)

**Testing**: `mise run gate`: in-crate unit tests in `ui/material/roboto.rs` and
`ui/material/link_text.rs`; client integration `tests/roboto_font.rs`, `tests/layout_snapshot.rs`,
`tests/layout_apparatus.rs`; core `tests/markdown_subset.rs`; quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace); touches `micold-client`, one core test
fixture, assets, docs

**Performance Goals**: no measurable startup cost: three more in-memory faces (~0.47 MB) parsed once,
one linear pass over the font database

**Constraints**: registration before the first text is shaped (cosmic-text caches matches); upright
text unchanged (FR-004); host fallback for uncovered characters kept

**Scale/Scope**: 1 new module (absorbing 4 constants from `text.rs`), 4 call sites switched to one
installer, 1 line in `painted`, 3 font files, 1 fixture line, 2 provenance files, 1 user-guide page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Decision logic is `pin_roboto_italics` (which faces go) and the one-line slant rule in `painted`; both unit-tested red first (a bare `fontdb::Database` with a file-loaded Roboto Italic; a code span inside emphasis). The face guard in `tests/roboto_font.rs` and the core bold-italic assertion are written failing first (they fail on the missing files and fixture run). `install_roboto` and its four call sites are glue (a loop and a call, no branch), covered by the resolution tests through `test_support::renderer`, the layout tests through `tests/support/layout.rs`, and quickstart §B for `startup.rs` and `showcase/main.rs` (binary wiring, as in feature 1260). |
| II. Multi-Session | PASS | One process-wide registration, no per-session state; every session view, the Overview and the showcase share it (quickstart B7). |
| III. Worktree Integration | PASS | No file or VCS operation. |
| IV. Local-First | PASS | Faces are compiled in; nothing is fetched at run time. The upstream is downloaded once at implementation and pinned by sha256 (research R1). |
| V. Rust + iced | PASS | iced's font system and its `fontdb`; no new stack. |
| VI. Cross-Platform | PASS | Same bytes and same database step on all three platforms, no OS branch; host copies cannot win (FR-006); layout snapshot on three CI runners, one fixture. |
| VII. Documentation | PASS | `docs/user-guide/appearance-theming.md` *The typeface* (emphasis is italic), `assets/fonts/PROVENANCE.md`, `crates/micold-client/tests/fixtures/FONT-PROVENANCE.md`, in the same PR. |
| VIII. Components | PASS | No widget added; `MarkdownView`/`LinkText` keep their builder API; the only component change is inside `painted`. The registration moves into the shared `ui::material` library beside `install_monospace`. |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### D1 — The faces and their provenance — research R1, R8; FR-002, FR-007, SC-004

- `assets/fonts/Roboto-Italic.ttf`, `Roboto-MediumItalic.ttf`, `Roboto-SemiBoldItalic.ttf`: cut
  from `ofl/roboto/Roboto-Italic[wdth,wght].ttf` with the `PROVENANCE.md` instancer script, list
  `[(400, "Roboto-Italic.ttf"), (500, "Roboto-MediumItalic.ttf"), (600, "Roboto-SemiBoldItalic.ttf")]`.
- `assets/fonts/PROVENANCE.md`: the overview table's Roboto row lists the six files; the Roboto
  section (retitled from "Regular 400, Medium 500" to the six faces) gains the italic upstream file
  and its sha256, the cut command, the date and fonttools version, and a row per italic file in the
  weight and checksum tables (sha256, name IDs 1/16); `LICENSE-Roboto-OFL.txt` covers them (same
  project, same OFL).
- No packaging manifest changes: the faces are `include_bytes!` into the binary, and every bundle
  already ships `LICENSE-Roboto-OFL.txt` (feature 1260 D5).

### D2 — Registration and the pin — research R2, R3; FR-005, FR-006; contract C-1–C-3

- New `crates/micold-client/src/ui/material/roboto.rs`: `ROBOTO`, `ROBOTO_FAMILY`, the six
  `ROBOTO_*_BYTES`, `ROBOTO_FACES`, `pin_roboto_italics`, `install_roboto`. The upright constants
  and `ROBOTO` move from `text.rs`; `material/mod.rs` and `ui/mod.rs` re-export the new names
  beside the moved ones, so existing `ui::ROBOTO_*` paths keep compiling.
- Callers (contract C-3): `shell/startup.rs` and `showcase/main.rs` call `install_roboto` beside
  `install_monospace` and drop their three `.font(ROBOTO_*_BYTES)` lines (comments updated: the
  italic faces, why the pin); `test_support.rs::renderer` and `tests/support/layout.rs::renderer`
  call it inside their `Once` instead of the three Roboto `load_font`s.
- Order: host faces are loaded when the global `FontSystem` is built; `install_roboto` loads the
  bundled faces as `Source::Binary` and then pins, so the pin sees both.

### D3 — The markdown span — research R4; FR-001, FR-003, FR-010, Edge: inline code

- `link_text::painted`: `if s.style.italic && !s.style.code { font.style = Slant::Italic }`. The
  mapping stays (FR-010); bold keeps `Weight::Bold`, which resolves to the 600 italic (contract C-2).
- Links inside emphasis are unchanged: colour and underline are set independently of the font.

### D4 — The bold-italic sample — research R5; FR-009

- `crates/micold-core/tests/fixtures/stream-json/markdown-subset.md` line 11: `**bold**, *italic*,
  ***bold italic***, ~~struck~~`. The showcase's `MarkdownView` section draws this fixture.
- `crates/micold-core/tests/markdown_subset.rs`: `style("bold italic") == Style { bold: true,
  italic: true, ..Style::default() }`.
- `crates/micold-core/tests/fixtures/stream-json/README.md`: mention the run if it lists the
  paragraph's contents.

### D5 — Geometry — research R7; FR-004, SC-001

- Run `tests/layout_snapshot.rs`; if `session-transcript-rich` changes (its `*leaves*` now has
  italic advances), re-record with `UPDATE_LAYOUT_SNAPSHOT=1` and confirm the diff is confined to
  that paragraph's nodes or its block's wrap. Any other state's change is a regression to fix, not
  to record.
- `tests/layout_apparatus.rs` keeps guarding the upright reference face; no change expected.

### D6 — Docs — Principle VII

- `docs/user-guide/appearance-theming.md` *The typeface*: emphasis in agent replies is drawn in
  Roboto's own italic, at the weight of the text around it; code inside emphasis stays upright.
- `crates/micold-client/tests/fixtures/FONT-PROVENANCE.md`: the italic faces join the list of
  faces the layout tests measure with.

## Requirement map

| Requirement | Design | Test |
|---|---|---|
| FR-001 | D2, D3 | `roboto.rs` resolution test (italic 400 → bundled); B1, B3 |
| FR-002 | D1 | `roboto_font.rs` weights 400/500/600 italic; `roboto.rs` resolution per weight |
| FR-003 | D3, contract C-2 | `roboto.rs`: weight 700 italic → SemiBoldItalic; `link_text.rs`: bold+italic span asks 700 italic; B1, B3 |
| FR-004 | D2 (pin keeps upright faces), D5 | `roboto.rs`: upright queries unchanged after pin, host upright kept; layout snapshot diff; B2, B7 |
| FR-005 | D2 (four callers) | resolution through `test_support::renderer`; layout tests through `tests/support/layout.rs`; B1 (showcase), B3 (app) |
| FR-006 | D2 (`pin_roboto_italics`) | `roboto.rs` host-copy test (file-loaded Roboto Italic removed, query → bundled); B5 |
| FR-007 | D1 | review of `PROVENANCE.md`; quickstart §A hashes |
| FR-008 | D1 | `roboto_font.rs`: parse, weight, `is_italic`, family, static, distinct bytes |
| FR-009 | D4 | `markdown_subset.rs` bold-italic style; B1 |
| FR-010 | D3 | `link_text.rs` italic span test (mapping kept) |
| SC-001 | D3, D5 | layout snapshot (upright unchanged); B1 |
| SC-002 | contract C-2 | `roboto.rs` weight-700 italic and weight-700 upright both resolve to weight-600 faces |
| SC-003 | D2 | host-copy test; CI on three runners |
| SC-004 | D1 | quickstart §A file sizes |

## Test strategy by layer

| Layer | Tests |
|---|---|
| Core (`micold-core/tests/markdown_subset.rs`) | `***bold italic***` parses to a span with `bold` and `italic` (FR-009) |
| Client in-crate unit (`ui/material/roboto.rs`) | on a bare `fontdb::Database`: a file-loaded Roboto Italic (temp copy of `ROBOTO_ITALIC_BYTES`) and a file-loaded upright Roboto, then the six bundled faces; `pin_roboto_italics` returns 1, the upright host face stays, italic 400/500/600/700 resolve to the bundled italic twins, upright 400/500/600 resolve as before the pin, other families kept, second call returns 0. Through the global font system (`test_support::renderer`): `ROBOTO` italic at 400/500/600 and bold resolves to the bundled bytes |
| Client in-crate unit (`ui/material/link_text.rs`) | `painted`: an italic span is `Slant::Italic`; bold+italic is `Weight::Bold` + italic; a code span inside emphasis is `Font::MONOSPACE` upright |
| Client integration (`tests/`) | `roboto_font.rs` face guard for six faces (FR-008); `layout_snapshot.rs` geometry gate on three runners (FR-004, SC-001); `layout_apparatus.rs` unchanged |
| Quickstart §B visual pass | B1–B7 on Linux: the glue in `startup.rs`/`showcase/main.rs`, the look, the host copy |

## Project Structure

### Documentation (this feature)

```text
specs/1259-markdown-italic-face/
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
├── Roboto-Italic.ttf            # new
├── Roboto-MediumItalic.ttf      # new
├── Roboto-SemiBoldItalic.ttf    # new
└── PROVENANCE.md                # Roboto section: italic faces
crates/micold-client/
├── src/ui/material/roboto.rs    # new (contract); absorbs ROBOTO + upright constants from text.rs
├── src/ui/material/text.rs      # constants moved out
├── src/ui/material/mod.rs       # re-exports
├── src/ui/mod.rs                # re-exports
├── src/ui/material/link_text.rs # painted: no slant on code
├── src/ui/material/test_support.rs
├── src/shell/startup.rs
├── src/showcase/main.rs
└── tests/
    ├── roboto_font.rs
    ├── support/layout.rs
    └── fixtures/{layout_snapshot.txt (if R7), FONT-PROVENANCE.md}
crates/micold-core/tests/
├── markdown_subset.rs
└── fixtures/stream-json/markdown-subset.md
docs/user-guide/appearance-theming.md
```

**Structure Decision**: the existing workspace; client-only code beside the other embedded faces in
`ui/material/`, one core test fixture line.

## Complexity Tracking

None.
