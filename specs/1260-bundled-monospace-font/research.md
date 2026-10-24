# Research: Bundled monospace typeface

Feature 1260. Each decision lists what was chosen, why, and what was rejected. Sources read:
`fontdb` 0.23.0, `cosmic-text` 0.15.0 and `iced_graphics` 0.14.0 (the versions in `Cargo.lock`),
and the existing font code in `crates/micold-client`.

## R1 — The face and its files

**Decision**: Fira Mono 3.206, Regular and Bold, from
[mozilla/Fira](https://github.com/mozilla/Fira) at `fd8c8c0a3d353cd99e8ca1662942d165e6961407`,
committed as `assets/fonts/FiraMono-Regular.ttf` and `assets/fonts/FiraMono-Bold.ttf`.

| File | SHA-256 | name ID 1 / 2 | usWeightClass | unitsPerEm |
|---|---|---|---|---|
| `FiraMono-Regular.ttf` | `8c86f2963208a353c3435e28ecb38a99ab68f14d7433ba00c1822cef9a9c1b44` | Fira Mono / Regular | 400 | 1000 |
| `FiraMono-Bold.ttf` | `2a28efd740e8da1da75d40ac79f0db4f60a0f1aead0b15ca16b4694a11b45fc6` | Fira Mono / Bold | 700 | 1000 |

The Regular hash is the committed test fixture's: fetched again from the same commit on 2026-10-24,
byte-identical, so the fixture **moves** (`git mv`) rather than being re-added. The Bold file was
fetched from the same commit on the same day; the hashes above were computed then.

**Rationale**: the spec's assumption (Fira Mono already vetted under the OFL, provenance already
recorded). Its 0.600 em advance is the terminal grid's hard-coded cell width (`CellMetrics::new`,
`width: size * 0.6`). Bold is needed because code already asks for it: `link_text.rs::painted` sets
`Weight::Bold` on bold or strong code spans, and `terminal.rs::cell_font` on bold cells (FR-011,
FR-007). Upstream's `LICENSE` has no Reserved Font Name clause, so the file name and family may be
kept as they are.

**Alternatives rejected**:
- *Regular only, Bold synthesised*: cosmic-text does not embolden; `fontdb::Database::query` falls
  back to the nearest weight, so bold code would draw Regular. FR-011 asks for a real Bold.
- *JetBrains Mono, Source Code Pro, Noto Sans Mono*: a new licence review and new provenance for no
  gain; Fira Mono is already in the repository and measured by the layout gate.
- *A variable font*: iced 0.14's font matcher selects among faces, not axis instances (the reason
  Roboto ships two static instances, `material/text.rs`).
- *Italic*: Fira Mono has none. An italic request resolves to the upright face (R3).

## R2 — How `Font::MONOSPACE` reaches the bundled face

**Decision**: keep every call site on `Font::MONOSPACE` (`Family::Monospace`) and, at startup, before
the first text is shaped, run one shared function on iced's global font system that

1. loads both bundled faces (`FontSystem::load_font`, which also bumps iced's font-system version),
2. **removes every other face whose family is `Fira Mono`** from the `fontdb` database (a host copy,
   any version), and
3. sets the database's monospace family to `Fira Mono` (`Database::set_monospace_family`).

**Rationale**: `iced_graphics::text::font_system()` builds its `cosmic_text::FontSystem` with
`load_system_fonts()`, and cosmic-text sets the monospace family to `Noto Sans Mono` by default
(`font/system.rs:158`), which is why each host draws code differently today. Pointing the family at
the bundled face fixes FR-002 for every call site at once, including any written later.

Step 2 is what FR-003 needs. `Database::query` (fontdb `lib.rs:661`) collects every face whose
family matches the name and picks the best match by stretch, style and weight; between a host
`Fira Mono Regular` and the bundled one the tie goes to the face found first, and host faces are
loaded first. The existing test guard (`the_monospace_face_parses_and_is_the_face_that_measures`)
checks that the resolved face is a `Source::Binary`, but on a host *with* Fira Mono installed nothing
made that true. Removing the other same-name faces makes the bundled copy the only candidate. Only
faces of that one family are removed, so the system fallback for uncovered characters (FR-004) keeps
every other face.

Step order matters: cosmic-text caches font matches per attribute set
(`FontSystem::get_font_matches`), so the function must run before any text is shaped. Startup calls
it before `iced::application(...).run()`; the test renderers call it in their existing `Once`.

**Alternatives rejected**:
- *A named font constant (`Font::with_name("Fira Mono")`) at each of the eight call sites*
  (`diff_view.rs` ×3, `text.rs`, `link_text.rs`, `code_block.rs`, `terminal.rs`, tests): more churn,
  a later `Font::MONOSPACE` silently reaches the host again, and a host font of the same name still
  wins the lookup.
- *Registering through `iced::application(...).font(bytes)` like Roboto*: that loads through a boot
  `Task`, after the caller can touch the database, so neither step 2 nor step 3 can follow it; and
  it would be a second registration path beside the tests'.
- *Renaming the family inside the font file* (e.g. `Micold Mono`) so no host font can share it:
  modifies an upstream binary, needs a new provenance procedure and touches OFL font-name rules,
  where removing the host copy from the in-process database does not.
- *Building a private `FontSystem` without system fonts*: iced's renderers use the global one; and
  dropping system fonts would break the fallback FR-004 requires.

## R3 — Weights, italic, and coverage

**Decision**: no extra code. With both faces registered, `Weight::Bold` resolves to the Bold face
and `Weight::Normal` to Regular. `Style::Italic` resolves to the upright face of the requested weight
(fontdb's style step falls back to `Normal` when no italic exists in the family). Characters Fira
Mono lacks fall through cosmic-text's per-glyph fallback to host faces, as Roboto's do today.

**Rationale**: the spec's edge cases allow upright or synthesised italic; fontdb gives upright.
`get_font_matches` keeps host faces in the fallback list, and R2 removes only `Fira Mono` faces.

**Alternatives rejected**: bundling a CJK or symbol monospace face (out of scope in the spec).

## R4 — One copy, and the tests measure it

**Decision**: `git mv` the fixture to `assets/fonts/`, delete `tests/fixtures/FONT-PROVENANCE.md`'s
monospace section's "test-only" framing (the file keeps documenting why the snapshot uses the shipped
faces), and make `tests/support/layout.rs` and the in-crate `material/test_support.rs` call the same
installer the application calls (R2). `REFERENCE_MONO_FONT_BYTES` becomes the shipped constant, not
an `include_bytes!` of a second path. A test walks the repository and fails if a `FiraMono*` font
file exists anywhere but `assets/fonts/` (FR-005, SC-003).

**Rationale**: the defect class `FONT-PROVENANCE.md` records twice already (a second Roboto, a
missing icon face). Calling the application's own installer means the layout tests measure exactly
what the application registers, removal step included.

**Alternatives rejected**: keeping the fixture and asserting its bytes equal the shipped file: still
two copies, against SC-003.

**Snapshot expectation**: the tests already measure Fira Mono Regular, so `layout_snapshot.txt`
should not change. Bold code may now resolve to the Bold face instead of Regular; both advance 0.600
em, so widths should not change either. Any difference is regenerated once
(`UPDATE_LAYOUT_SNAPSHOT=1`) and reviewed in the same commit (spec Assumptions).

## R5 — Licence in every release bundle

**Finding that corrects a spec assumption**: the spec assumes "release bundling already carries the
interface fonts' licence files". It does not. The fonts are compiled into the binary with
`include_bytes!`, and no package copies `assets/fonts/LICENSE*`: the `.deb` and `.rpm` asset lists
(`crates/micold-client/Cargo.toml`), `scripts/macos-bundle.sh` and `packaging/windows/micold-ai-ide.iss`
`[Files]` ship binaries, the desktop entry and icons only. The Windows installer shows the repository
`LICENSE` (Apache-2.0) as its licence page.

**Decision**: ship all three font licence files — `LICENSE-FiraMono-OFL.txt` (FR-008), and with it
`LICENSE-Roboto-OFL.txt` and `LICENSE` (Material Symbols, Apache-2.0) because they take the same
path — plus `PROVENANCE.md` (the mapping of file to licence), into a `fonts` licence directory of
each bundle:

| Bundle | Destination |
|---|---|
| `.deb` | `usr/share/doc/micold-ai-ide/fonts/` |
| `.rpm` | `/usr/share/doc/micold-ai-ide/fonts/` |
| macOS `.app` | `Contents/Resources/licenses/fonts/` |
| Windows installer | `{app}\licenses\fonts\` |

The licence files are added to the `micold-packaging` set in `.gitattributes`, since the installer
build now reads them.

**Rationale**: FR-008 and SC-004 require the Fira Mono licence in 100% of bundles. The OFL requires
the licence to travel with the font, and the binary carries the font, so the package must carry the
licence. Roboto has the same obligation and is unmet today; shipping it on the same path costs one
line per bundle and is recorded as a decision in the ledger.

**Alternatives rejected**:
- *Fira Mono's licence only*: leaves the bundles carrying one OFL font's licence and not the other's,
  which FR-008's "in the same way the other bundled faces' licences are" cannot describe.
- *An in-app licences screen*: new UI, out of the spec's scope; the files in the bundle are what an
  auditor inspects (US3).
- *Concatenating the licences into one file*: `assets/fonts/PROVENANCE.md` explains why the licences
  are kept separate; the bundle keeps the same files.

## R6 — Where the shared code lives

**Decision**: a new module `crates/micold-client/src/ui/material/mono.rs` beside `text.rs` (Roboto)
and `glyph.rs` (icons), exporting the two byte constants, the family name, `MONO_FAMILY`, and two
functions:

- `pin_monospace(db: &mut fontdb::Database) -> usize` — R2 steps 2 and 3 on a bare database; returns
  how many other same-name faces it removed. Pure over the database, so unit-tested on a fresh
  `fontdb::Database` without iced's global state.
- `install_monospace(fonts: &mut iced::advanced::graphics::text::FontSystem)` — loads both faces,
  then calls `pin_monospace` on `fonts.raw().db_mut()`. The one entry point for `shell/startup.rs`,
  `showcase/main.rs`, `material/test_support.rs` and `tests/support/layout.rs`.

**Rationale**: the decision logic (which faces are removed, what the family is set to) lives in a
function a test can drive with a database that contains a same-name host face (a temp-file copy of
the font loaded as `Source::File`). The installer is three calls of glue. `micold-core` is iced-free
and cannot name `fontdb` types; the client's `ui::material` already owns the other two embedded faces.

**Alternatives rejected**: adding the bytes to `text.rs` (it is the Roboto type-scale module, already
long); a function in `shell/startup.rs` (not reachable from the showcase or the tests).

## R7 — The terminal (FR-007, pending the user's confirmation)

**Decision**: the terminal adopts the bundled face with **no terminal code change**: `cell_font`
starts from `Font::MONOSPACE`, which R2 points at Fira Mono, and sets `Weight::Bold` for bold cells,
which resolves to the Bold face. The terminal step adds tests only (cell fonts resolve to the bundled
Regular and Bold faces; `CellMetrics::new(size).width` equals the bundled face's advance at `size`)
and the user-guide sentence (FR-010). It is planned as its own step (D4) so it can be dropped.

**If the user answers "no" (keep the host font in the terminal)**: the terminal names the family
`Family::Monospace` resolved to before this feature as a constant,
`Font::with_name("Noto Sans Mono")` (cosmic-text's default, `font/system.rs:158`), and `cell_font`
starts from it instead of `Font::MONOSPACE`. A constant rather than a value captured at run time, so
`pin_monospace` and `install_monospace` stay as contracted. Everything that changes in that case is
listed in plan D4; the registration, the tests' face, the one-copy rule and the licences do not.

**Rationale**: as recorded in the spec's clarification. The 0.6 em cell width now equals the face
the grid draws with on every host, instead of approximating it.

**Alternatives rejected**: a terminal-only font path kept on the host face (the "no" branch above) —
it leaves the one fixed-width surface that differs per OS (Principle VI).

## R8 — Registration failure

**Decision**: no special handling in the application. If the bundled bytes did not parse,
`FontSystem::load_font` logs and skips them; `Fira Mono` then has no face, `Family::Monospace`
resolves to nothing by name and cosmic-text falls back to a host face, so the application starts
and draws code. The test suite fails: the bytes guard (both faces parse, report `Fira Mono`, weights
400/700, are monospaced, advance 600 units) and the layout guard both name the face.

**Rationale**: the spec's edge case asks for exactly this split (start in the app, fail in tests).

**Alternatives rejected**: panicking at startup on a bad font (the spec says the app must start).
