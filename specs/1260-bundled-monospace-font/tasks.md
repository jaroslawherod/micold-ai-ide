# Tasks: Bundled monospace typeface

**Input**: Design documents from `specs/1260-bundled-monospace-font/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/fonts.md](contracts/fonts.md), [quickstart.md](quickstart.md)

**Tests**: Mandatory (Constitution I). A test task comes before the implementation task it covers
and must fail first (the task says what makes it red). The glue (`install_monospace` and its call
sites in `shell/startup.rs` and `showcase/main.rs`) is covered by the tests that go through
`test_support::renderer` / `tests/support/layout.rs::renderer` and by quickstart §B.

**Documentation**: the user guide (FR-010) in M1, the milestone that makes the face visible;
`docs/development/layout-snapshot.md` and both provenance files in M1, the milestone that moves the
font (plan D1, D6).

**FR-007 is not yet confirmed by the user.** The terminal step (plan D4) is one task, T013, plus
the terminal sentence of T014 and quickstart B7 in T015. If the user declines FR-007, replace T013
with plan D4's *If the user declines* list 1–6 (a `TERMINAL_FONT` constant in `terminal.rs`, its
test first, and the spec, contract, data-model and quickstart edits); nothing else in this file
changes.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US3) the task serves; Setup, Foundational and Polish tasks have none

Paths: `client` = `crates/micold-client`.

---

## Phase 1: Setup — the face and its provenance (plan D1; FR-001, FR-005, FR-008)

- [ ] T001 Move the face into the shipped assets: `git mv crates/micold-client/tests/fixtures/FiraMono-Regular.ttf
  assets/fonts/FiraMono-Regular.ttf` and `git mv crates/micold-client/tests/fixtures/LICENSE-FiraMono-OFL.txt
  assets/fonts/LICENSE-FiraMono-OFL.txt`; add `assets/fonts/FiraMono-Bold.ttf` fetched from
  `mozilla/Fira` at commit `fd8c8c0a3d353cd99e8ca1662942d165e6961407` (Fira Mono 3.206). Verify with
  `sha256sum`: Regular `8c86f2963208a353c3435e28ecb38a99ab68f14d7433ba00c1822cef9a9c1b44`, Bold
  `2a28efd740e8da1da75d40ac79f0db4f60a0f1aead0b15ca16b4694a11b45fc6` (research R1); stop on a
  mismatch
- [ ] T002 Keep the layout tests compiling after the move: in `client/tests/support/layout.rs`
  `REFERENCE_MONO_FONT_BYTES` includes `../../../../assets/fonts/FiraMono-Regular.ttf` (the shipped
  file); its doc comment says it is the shipped face. (T018 replaces it with the shipped constant.)
- [ ] T003 [P] `assets/fonts/PROVENANCE.md`: the typeface table gains Fira Mono (two files, SIL OFL
  1.1, `LICENSE-FiraMono-OFL.txt`); a *Fira Mono* section records upstream `mozilla/Fira`, the commit,
  version 3.206, the SHA-256 of each file (T001), and that the committed `LICENSE-FiraMono-OFL.txt`
  is the upstream `LICENSE` with LF line endings (contract C-5)
- [ ] T004 [P] `client/tests/fixtures/FONT-PROVENANCE.md`: Fira Mono joins the "faces the
  application ships" table (path `assets/fonts/`); the "for the tests only" table and its framing
  go; the feature-940 section stays as history and ends by pointing at feature 1260
- [ ] T005 [P] `docs/development/layout-snapshot.md` (around line 183, "Code is the exception"):
  the application ships Fira Mono and the snapshot measures that shipped face; no test-only copy

---

## Phase 2: Foundational — the registration module (plan D2; contract C-1–C-3)

- [ ] T006 Bytes guard tests, red first (fail to compile until T008), in a `#[cfg(test)] mod tests`
  of `client/src/ui/material/mono.rs`: `FIRA_MONO_REGULAR_BYTES` and `FIRA_MONO_BOLD_BYTES` each
  parse with `ttf_parser`, report family (name ID 1) `MONO_FAMILY` = `"Fira Mono"`, weight 400 and
  700, `is_monospaced()`, and advance 600 units per 1000-unit em for every printable ASCII character
  (C-1); each failure message names `Fira Mono`
- [ ] T007 `pin_monospace` tests, red first, same module, on a bare `fontdb::Database`
  (`iced::advanced::graphics::text::cosmic_text::fontdb`) holding a `Source::File` same-name face
  (a `tempfile` copy of the Regular bytes, loaded with `load_font_file`) loaded **before** the two
  bundled faces (`load_font_source(Source::Binary(..))`): the call returns 1 and the file face is
  gone; `db.family_name(&Family::Monospace) == MONO_FAMILY`; `Family::Monospace` weight 400 →
  the Regular `Binary` face, weight 700 → the Bold face, `Style::Italic` → the upright face of the
  nearest weight; a face of another family (e.g. `ROBOTO_REGULAR_BYTES`) is kept (FR-004); a second
  call returns 0 and leaves the face count unchanged (C-2 table)
- [ ] T008 Implement `client/src/ui/material/mono.rs` per contract C-1–C-3: `MONO_FAMILY`,
  `FIRA_MONO_REGULAR_BYTES` / `FIRA_MONO_BOLD_BYTES` (`include_bytes!` of
  `assets/fonts/FiraMono-{Regular,Bold}.ttf`, the path style of `ROBOTO_*_BYTES` in `text.rs`),
  `pin_monospace(db: &mut fontdb::Database) -> usize` (remove every face whose `families` contains
  `MONO_FAMILY` and whose `source` is not `Source::Binary`; set the monospace family) and
  `install_monospace(fonts: &mut iced::advanced::graphics::text::FontSystem)` (two
  `load_font(Cow::Borrowed(..))`, then `pin_monospace(fonts.raw().db_mut())`; no branch). Declare
  `mod mono;` in `client/src/ui/material/mod.rs`, re-export the four items there (line 262) and
  from `client/src/ui/mod.rs` (line 152) beside `ROBOTO_*`. T006 and T007 pass

---

## Phase 3: User Story 1 — Code looks the same on every machine (Priority: P1) 🎯 MVP

**Goal**: every `Font::MONOSPACE` text run — code blocks, inline code, diffs, tool output, the
terminal — draws the bundled Fira Mono on every host, ahead of a host font of the same name.

**Independent test**: T009 and T013 pass; quickstart §B B1–B8 on Linux.

- [ ] T009 [US1] Test, red until T010: in `client/src/ui/material/mono.rs` tests, after
  `test_support::renderer()`, the global `font_system()` database resolves `Family::Monospace`
  (weight 400 and 700) to a `Source::Binary` face of family `Fira Mono` whose bytes are
  `FIRA_MONO_REGULAR_BYTES` / `FIRA_MONO_BOLD_BYTES` (compare the face's data through
  `with_face_data`); message names `Fira Mono`
- [ ] T010 [US1] `client/src/ui/material/test_support.rs::renderer`: call
  `super::install_monospace(&mut fonts)` inside its `Once`, after the Roboto and icon loads. Write
  T013's tests (if FR-007 stands) and run them and T009 red before this change; both pass after
- [ ] T011 [P] [US1] `client/src/shell/startup.rs`: call `micold_client::ui::install_monospace` on
  `iced::advanced::graphics::text::font_system().write()` once, before `iced::application(...)`;
  beside the Roboto `.font(..)` comment, say why the monospace face is registered this way instead
  (`.font` cannot remove a same-name host face or set the monospace family; research R2)
- [ ] T012 [P] [US1] `client/src/showcase/main.rs`: the same call before its `iced::application(...)`
- [ ] T013 [US1] **Terminal (plan D4, FR-007 — droppable, see the header).** In-crate tests in
  `client/src/ui/terminal.rs` `mod tests` (line 1505), using `test_support::renderer()`:
  `cell_font(Flags::empty())` and `cell_font(Flags::BOLD)` resolve in the global database to the
  bundled Regular and Bold faces; `CellMetrics::new(s).width` equals Fira Mono's advance at `s`
  (600/1000 × s, from `FIRA_MONO_REGULAR_BYTES`' `hmtx`) for s = 12, 13, 14. No change to
  `cell_font` or `CellMetrics` (research R7). Red before T010, green after
- [ ] T014 [US1] User guide (FR-010): `docs/user-guide/appearance-theming.md` *The typeface*: the
  application also ships **Fira Mono** for code blocks, inline code, diffs, tool output and the
  terminal; bold code uses its Bold weight; characters it lacks still fall back to a system font;
  the terminal keeps its own character grid and stays outside Roboto and the type scale but no
  longer uses the host's monospaced font (line 58: replace "The terminal keeps its monospaced
  font"; under a declined FR-007, keep that line and say code elsewhere uses Fira Mono).
  `docs/user-guide/worktrees-and-sessions.md` line 1125: "in a monospace font" names Fira Mono
- [ ] T015 [US1] Visual pass, quickstart §B B1–B8 (B7 per T013), light and dark, through the
  `visual-pass` skill; evidence in `specs/1260-bundled-monospace-font/visual-pass/`

**Checkpoint**: the app and the showcase draw code in Fira Mono; in-crate component tests measure it.

---

## Phase 4: User Story 2 — Layout tests measure the face users see (Priority: P2)

**Goal**: the layout tests register the face through the application's own installer and guard
both weights; the repository holds one copy of each face.

**Independent test**: `cargo test -p micold-client --test layout_apparatus --test layout_snapshot`.

- [ ] T016 [US2] `client/tests/layout_apparatus.rs`
  `the_monospace_face_parses_and_is_the_face_that_measures` (line 169): keep its checks, and add the
  Bold face — `Family::Monospace` with `Weight::Bold` resolves to a `Source::Binary` face whose data
  is `micold_client::ui::FIRA_MONO_BOLD_BYTES`, and `measure(PROBE, Font { weight: Bold,
  ..Font::MONOSPACE })` equals the Bold face's own advances; every failure message names
  `Fira Mono` (FR-006). The Regular and Bold probe widths equal to the face's own advances are the
  SC-001 check, run on all three CI runners. Red until T018 (the test renderer loads no Bold face)
- [ ] T017 [US2] One-copy test in `client/tests/layout_apparatus.rs`, beside the guard: walk the
  repository from `CARGO_MANIFEST_DIR/../..`, skipping `target*`, `.git` and `.claude/worktrees`,
  and fail naming each file whose name starts `FiraMono` and ends `.ttf`/`.otf` outside
  `assets/fonts/` (FR-005, SC-003). It passes after T001; prove it can fail by running it once with
  a temporary copy in `client/tests/fixtures/` (delete the copy after)
- [ ] T018 [US2] `client/tests/support/layout.rs::renderer`: replace its monospace `load_font` and
  `set_monospace_family` with `micold_client::ui::install_monospace(&mut fonts)` inside the `Once`;
  `REFERENCE_MONO_FONT_BYTES` = `micold_client::ui::FIRA_MONO_REGULAR_BYTES` and
  `REFERENCE_MONO_FONT_FAMILY` = `micold_client::ui::MONO_FAMILY`; doc comment: the shipped face,
  registered by the application's installer (T016 passes)
- [ ] T019 [US2] Run `client/tests/layout_snapshot.rs`. Expected unchanged (research R4); if it
  differs, regenerate once with `UPDATE_LAYOUT_SNAPSHOT=1` and confirm the diff of
  `client/tests/fixtures/layout_snapshot.txt` touches monospace text nodes only (SC-005), saying so
  in the commit message

**Checkpoint**: layout tests measure the shipped Regular and Bold through the shipped installer.

---

## Phase 5: User Story 3 — Licensing and provenance stay correct (Priority: P3)

**Goal**: every release bundle carries the font licence set (data-model *Font licence set*:
`LICENSE-FiraMono-OFL.txt`, `LICENSE-Roboto-OFL.txt`, `LICENSE`, `PROVENANCE.md`, all from
`assets/fonts/`) at research R5's destination.

**Independent test**: `cargo test -p micold-client --test packaging_ships_font_licences --test
packaging_excludes_showcase`; `scripts/tests/macos-bundle.test.sh`.

- [ ] T020 [US3] Refactor, no behaviour change: move `repo_root`, `deb_assets` and `rpm_assets` from
  `client/tests/packaging_excludes_showcase.rs` (lines 41, 71, 89) into a new
  `client/tests/support/packaging.rs`; `packaging_excludes_showcase.rs` and
  `client/tests/deb_ships_no_service_units.rs` (its own copies, lines 37, 56) use it. Both tests
  stay green
- [ ] T021 [US3] New `client/tests/packaging_ships_font_licences.rs`, red until T024–T026: each of
  the four licence-set files exists in `assets/fonts/` and appears in the deb assets (destination
  `usr/share/doc/micold-ai-ide/fonts/`), the rpm assets (`/usr/share/doc/micold-ai-ide/fonts/`),
  the `packaging/windows/micold-ai-ide.iss` `[Files]` (`DestDir` `{app}\licenses\fonts`) and
  `scripts/macos-bundle.sh` (`Contents/Resources/licenses/fonts/`); a failure names the file and the
  bundle
- [ ] T022 [P] [US3] `client/tests/packaging_excludes_showcase.rs`: synthetic cases beside
  `a_windows_installer_that_ships_an_unlisted_file_fails` (line 613), red until T026: a licence-set
  file with `DestDir: "{app}\licenses\fonts"` passes; the same file with `DestDir: "{app}"` fails;
  an unlisted file under `{app}\licenses\` still fails
- [ ] T023 [P] [US3] `scripts/tests/macos-bundle.test.sh`: the staged bundle holds the four files in
  `Contents/Resources/licenses/fonts/`; an `expect_failure` case with one licence file removed from
  the fake tree, expecting `missing font licence: assets/fonts/<file>`; red until T025
- [ ] T024 [US3] `client/Cargo.toml`: `[package.metadata.deb] assets` and
  `[package.metadata.generate-rpm] assets` gain the four files at the destinations of T021
- [ ] T025 [US3] `scripts/macos-bundle.sh`: copy each of the four files, named (no wildcard), to
  `Contents/Resources/licenses/fonts/`, failing with `die "missing font licence: <path>"` when one is
  missing (the `missing icon:` pattern, line 86)
- [ ] T026 [US3] `packaging/windows/micold-ai-ide.iss` `[Files]`: four explicit lines to
  `{app}\licenses\fonts` (no wildcard, as the file's comment requires);
  `client/tests/packaging_excludes_showcase.rs`: add `WINDOWS_SHIPPED_LICENCES` (the four basenames),
  a helper `iss_file_entries` returning `(source, dest_dir)` pairs beside `iss_file_sources`
  (line 529), and pass the licence set to `windows_violations` (line 488), which accepts those
  basenames only with a `DestDir` under `{app}\licenses\`; the wildcard and showcase rules unchanged.
  T021 and T022 pass, `the_windows_installer_contains_no_showcase` stays green
- [ ] T027 [P] [US3] `.gitattributes`: `/assets/fonts/LICENSE*` and `/assets/fonts/PROVENANCE.md`
  join the `micold-packaging` set (beside `/LICENSE`, line 149)

**Checkpoint**: deb, rpm, `.app` and the Windows installer each carry the font licence set.

---

## Phase 6: Polish (close unit)

- [ ] T028 Quickstart §A: `sha256sum assets/fonts/FiraMono-*.ttf` equals `assets/fonts/PROVENANCE.md`;
  `git ls-files | grep -i firamono` lists only `assets/fonts/`; record the results in quickstart.md
- [ ] T029 Quickstart §C: record what CI's macOS and Windows runners proved (layout snapshot,
  packaging tests) and what needs a host (the look, installed bundle contents) in quickstart.md

---

## Dependencies & Execution Order

- Setup (T001–T005) → Foundational (T006–T008) → US1 (T009–T015).
- US2 (T016–T019) needs T008 (the shipped constants and installer); it does not need US1's call sites.
- US3 (T020–T027) needs T001 only (the licence file's new place); independent of US1 and US2 code.
- Within a story: each red test task before the task that turns it green (named in the task):
  T006, T007 → T008; T009, T013 → T010; T016 → T018; T021 → T024–T026; T022 → T026; T023 → T025.

## Parallel Opportunities

- T003, T004, T005 after T001.
- T011 and T012 after T008.
- T022, T023, T027 after T020; T024, T025 after T021.
- US3 shares no file with US1 or US2 and can run beside them.

## Implementation Strategy

MVP is M1: the face shipped and registered, the app drawing it, the user guide saying so. M2 then
turns the layout tests to the shipped installer and both weights; M3 adds the licences to the
bundles. Each merges on its own and leaves `main` green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Ship Fira Mono and draw all code with it 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: the application and showcase draw code blocks, inline code, diffs, tool output
  and terminal cells in the bundled Fira Mono (Bold for bold), ahead of any host font of that name;
  the user guide says so
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002, FR-003, FR-004, FR-007, FR-009,
  FR-010, FR-011; FR-008 (repository half: licence and provenance in `assets/fonts/`)
- **Verify**: `cargo test -p micold-client --lib -- mono terminal`; quickstart §B B1–B8 (visual-pass
  evidence in `visual-pass/`)
- **Depends on**: —
- **Tier**: full

### M2 — Layout tests measure the shipped face through the shipped installer

- **Tasks**: T016–T019
- **Deliverable**: `layout_apparatus` guards Regular and Bold as registered by
  `install_monospace`, and fails naming any `FiraMono*` font file outside `assets/fonts/`
- **Satisfies**: US2 acceptance scenarios 1–3; FR-005, FR-006; SC-001, SC-002, SC-003, SC-005
- **Verify**: `cargo test -p micold-client --test layout_apparatus --test layout_snapshot`
- **Depends on**: M1
- **Tier**: light (test code only; patterns in `tests/support/layout.rs` and
  `tests/layout_apparatus.rs`; no user-guide change: PR gets `docs-not-needed`)

### M3 — Font licences in every release bundle

- **Tasks**: T020–T027
- **Deliverable**: deb, rpm, the macOS `.app` and the Windows installer each ship the font licence
  set in their `fonts` licence directory, enforced by a packaging test and the macOS bundle script
  test
- **Satisfies**: US3 acceptance scenarios 1–2; FR-008; SC-004
- **Verify**: `cargo test -p micold-client --test packaging_ships_font_licences --test
  packaging_excludes_showcase --test deb_ships_no_service_units`; `scripts/tests/macos-bundle.test.sh`
- **Depends on**: M1 (licence file location)
- **Tier**: full (packaging manifests, installer script, Windows allow-list rule)

Polish (T028–T029) changes no code: the close unit does it in the close PR.
