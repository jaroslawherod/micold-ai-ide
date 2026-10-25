# Tasks: Markdown italic draws in a true italic face

**Input**: Design documents from `specs/1259-markdown-italic-face/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/fonts.md](contracts/fonts.md), [quickstart.md](quickstart.md)

**Tests**: Mandatory (Constitution I). A test task comes before the implementation task it covers
and must fail first (the task says what makes it red). The glue (`install_roboto` and its call sites
in `shell/startup.rs` and `showcase/main.rs`) is covered by the tests that go through
`test_support::renderer` / `tests/support/layout.rs::renderer` and by quickstart §B (plan,
Constitution Check I).

**Documentation**: the user guide in each milestone that changes what a user sees (T017, T021);
both provenance files in M1, the milestone that adds the faces (plan D1, D6).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US3) the task serves; Setup, Foundational and Polish tasks have none

Paths: `client` = `crates/micold-client`, `core` = `crates/micold-core`.

---

## Phase 1: Setup — the faces and their provenance (plan D1; FR-002, FR-007, FR-008, SC-004)

- [x] T001 Face guard, red first (fails to compile until T002: the `include_bytes!` files are
  missing): extend `client/tests/roboto_font.rs` from three to six faces. Add `ITALIC`,
  `MEDIUM_ITALIC`, `SEMIBOLD_ITALIC` (`include_bytes!("../../../assets/fonts/Roboto-Italic.ttf")`,
  `Roboto-MediumItalic.ttf`, `Roboto-SemiBoldItalic.ttf`, beside `REGULAR` line 18) and fold them
  into the existing tests rather than copying them: each italic face parses
  (`both_faces_parse`, line 50), reports `weight()` 400 / 500 / 600 and `is_italic()`
  (`the_two_faces_report_the_weights_the_type_scale_asks_for`, line 59), typographic family
  `Roboto` (name ID 16, else 1; line 82), is static (no `fvar`; line 101), covers the interface's
  own text (line 125), and its bytes differ from every upright face (line 150); each upright face
  reports `!is_italic()`. Rename the tests whose names say "two faces" to say "faces". Every failure
  message names the file (FR-008, contract C-1)
- [x] T002 Cut the three italic faces (research R1): fetch
  `https://raw.githubusercontent.com/google/fonts/main/ofl/roboto/Roboto-Italic[wdth,wght].ttf`
  into the session scratchpad, record its sha256, and run the instancer script in
  `assets/fonts/PROVENANCE.md` (the one that cut the upright faces: `fontTools.varLib.instancer`,
  `wdth=100`, `updateFontNames=True, optimize=True`) through `uv run --with fonttools` with the list
  `[(400, "Roboto-Italic.ttf"), (500, "Roboto-MediumItalic.ttf"), (600, "Roboto-SemiBoldItalic.ttf")]`,
  writing into `assets/fonts/`. Each file is about 160 KB, under 0.5 MB together (SC-004); stop if
  not. T001 passes
- [x] T003 [P] `assets/fonts/PROVENANCE.md`: the overview table's Roboto row lists the six files; the
  Roboto section is retitled from "Regular 400, Medium 500" to the six faces and gains the italic
  upstream path and its sha256 (T002), the cut command, the date and the fonttools version, and a
  row per italic file in the weight and checksum tables (sha256, name IDs 1/16);
  `LICENSE-Roboto-OFL.txt` is named as covering them (same project, same OFL) (FR-007)
- [x] T004 [P] `client/tests/fixtures/FONT-PROVENANCE.md`: the three italic faces join the faces the
  layout tests measure with (plan D6)

---

## Phase 2: Foundational — the Roboto module (plan D2; contract C-1, C-3)

- [x] T005 Resolution tests, red first (fail to compile until T006), in a `#[cfg(test)] mod tests`
  of a new `micold-material/src/roboto.rs`, on a bare `fontdb::Database`
  (`iced::advanced::graphics::text::cosmic_text::fontdb`) holding the six `ROBOTO_FACES` loaded as
  `Source::Binary`: `Family::Name("Roboto")` italic at 400, 500, 600 resolves to
  `ROBOTO_ITALIC_BYTES`, `ROBOTO_MEDIUM_ITALIC_BYTES`, `ROBOTO_SEMIBOLD_ITALIC_BYTES`; italic at 700
  (`Weight::Bold`) and upright at 700 both resolve to faces whose `weight` is 600 (SC-002); upright
  400/500/600 resolve to the three upright bytes. Follow `mono.rs`' helpers (`monospace`, line 117;
  `assert_bundled_face`, line 71) for the query and the byte comparison
- [x] T006 Implement `micold-material/src/roboto.rs` per contract C-1 and C-3: move `ROBOTO` and
  the three `ROBOTO_*_BYTES` from `micold-material/src/text.rs` (lines 110–121, doc comments
  with them); add `ROBOTO_FAMILY`, the three italic `ROBOTO_*_ITALIC_BYTES` (`include_bytes!` of
  T002's files), `ROBOTO_FACES` (upright then italic), `ROBOTO_WEIGHTS = [400, 500, 600]` and
  `install_roboto(fonts: &mut FontSystem)`, which loads each of `ROBOTO_FACES` with
  `load_font(Cow::Borrowed(..))` (no branch; the pin call joins it in T020). Declare `mod roboto;`
  in `micold-material/src/lib.rs` (beside `pub mod mono;`; `client/src/ui/material/mod.rs`
  before #1262) and re-export the new names beside the moved ones (line 271) and from `client/src/ui/mod.rs` (line 158), so every existing
  `ui::ROBOTO*` path (e.g. `mono.rs` line 137, `tests/add_worktree_form_survives_a_refusal.rs`)
  keeps compiling. T005 passes

---

## Phase 3: User Story 1 + User Story 2 — Emphasis is italic, bold italic stays bold (P1, P2) 🎯 MVP

**Goal**: every emphasised run in `MarkdownView` and `LinkText` draws Roboto's own italic at the
weight of the text around it; bold italic draws the 600 italic, as heavy as plain bold (600);
code inside emphasis stays upright Fira Mono. US2 rides in this phase because the 600 italic it
needs is required by FR-002 for US1 (headings and bold roles keep their weight); what is left of
US2 alone is the showcase sample (see the ledger, milestone cut).

**Independent test**: T007, T012, T014 pass; `layout_snapshot` changes only in
`session-transcript-rich`; quickstart §B B1–B4, B6, B7 on Linux.

- [x] T007 [US1] Test, red until T008: in `micold-material/src/roboto.rs` tests, after
  `test_support::renderer()`, `test_support::resolves_to_bundled` (line 638) holds for `ROBOTO`
  with `Slant::Italic` at weights 400, 500, 600 → the three italic bytes, and at `Weight::Bold` →
  `ROBOTO_SEMIBOLD_ITALIC_BYTES` (FR-001, FR-002, FR-003, FR-005); messages name the face
  (assumes no host Roboto italic: it resolves through the global font system, so on a machine
  with `fonts-roboto` installed it can fail until the pin joins `install_roboto` in T020)
- [x] T008 [US1] `micold-material/src/test_support.rs::renderer`: replace the three
  `load_font(ROBOTO_*)` lines (73–75) with `super::install_roboto(&mut fonts)` inside the `Once`.
  T007 passes
- [x] T009 [US1] `client/tests/support/layout.rs::renderer`: replace the three Roboto `load_font`
  calls (lines 193–201) with `micold_client::ui::install_roboto(&mut fonts)` inside the `Once`; the
  `REFERENCE_FONT_*` constants stay for the apparatus guard (contract C-3).
  `cargo test -p micold-client --test layout_apparatus` stays green (plan D5)
- [x] T010 [P] [US1] `client/src/shell/startup.rs`: call `micold_client::ui::install_roboto` on the
  same `font_system().write()` guard as `install_monospace` (line 88), before
  `iced::application(...)`; drop the three `.font(ROBOTO_*_BYTES)` calls (lines 102–104); keep
  `.default_font(ROBOTO)`; the comment says the italic faces are registered here too and why
  through the font system (the pin, T020)
- [x] T011 [P] [US1] `client/src/showcase/main.rs`: the same change (install beside line 59, drop
  lines 81–83)
- [x] T012 [US1] Span-font tests, red first, in `micold-material/src/link_text.rs` `mod tests`
  (line 244), calling `painted` directly: an italic span is `Slant::Italic` at the base weight
  (FR-010); a bold+italic span is `Weight::Bold` + `Slant::Italic` (US2, FR-003); a code span with
  `italic` set is `Font::MONOSPACE` with `Slant::Normal` (Edge: inline code), and at `Weight::Bold`
  when also bold (contract C-4). The code case is red until T013
- [x] T013 [US1] `micold-material/src/link_text.rs::painted` (line 114):
  `if s.style.italic && !s.style.code { font.style = Slant::Italic }`; nothing else changes. T012
  passes
- [x] T014 [US2] Core test, red until T015: in `core/tests/markdown_subset.rs`, beside the
  `style("italic")` assertion (line 118), `style("bold italic") == Style { bold: true, italic: true,
  ..Style::default() }` (FR-009)
- [x] T015 [US2] `core/tests/fixtures/stream-json/markdown-subset.md` line 11: `**bold**, *italic*,
  ***bold italic***, ~~struck~~` (plan D4). Run `mise run test-core`; fix any count or text
  assertion in `core/tests/markdown_subset.rs` / `markdown_stream.rs` that the extra run moves.
  `core/tests/fixtures/stream-json/README.md` (line 91): mention the bold-italic run if it lists
  the paragraph's contents. T014 passes
- [x] T016 [US1] Run `client/tests/layout_snapshot.rs` (plan D5, research R7). If it differs,
  re-record once with `UPDATE_LAYOUT_SNAPSHOT=1` and confirm the diff of
  `client/tests/fixtures/layout_snapshot.txt` touches only `session-transcript-rich`'s paragraph
  holding `*leaves*` (or its block's wrap); say so in the commit message. Any other state changing
  is an FR-004 regression to fix, not to record
- [x] T017 [US1] User guide: `docs/user-guide/appearance-theming.md` *The typeface* (around line
  87): emphasis in agent replies is drawn in Roboto's own italic, which ships with the application,
  at the weight of the text around it (bold italic stays bold); code inside emphasis stays upright
- [x] T018 [US1] Visual pass, quickstart §B B1–B4, B6, B7, light and dark, through the
  `visual-pass` skill; evidence in `specs/1259-markdown-italic-face/visual-pass/`

**Checkpoint**: the app and the showcase draw emphasis in the bundled italic, bold italic at 600.

---

## Phase 4: User Story 3 — The same look with a host Roboto installed (Priority: P3)

**Goal**: a host-installed Roboto italic, or a host Roboto upright at a weight the application does
not ship (Bold 700), never wins over the bundled faces (FR-006, FR-003 on every host, FR-004's one
exception).

**Independent test**: T019 passes; quickstart §B B5.

- [ ] T019 [US3] Pin tests, red first (fail to compile until T020), in `micold-material/src/roboto.rs`
  tests, on a bare `fontdb::Database` seeded as contract C-2 says, in this order: a `tempfile` copy
  of `ROBOTO_ITALIC_BYTES`, a copy of `ROBOTO_REGULAR_BYTES` and a copy of `ROBOTO_SEMIBOLD_BYTES`
  with the OS/2 `usWeightClass` patched to 700 (find the `OS/2` record in the table directory,
  write big-endian 700 at table offset 4; glyphs unchanged), each loaded with `load_font_file`
  (`mono.rs`' host-copy test, line 128, is the pattern), then the six bundled faces as
  `Source::Binary`. Assert: the patched file loads with weight 700; `pin_roboto_italics` returns 2;
  the host 400 face stays; each C-2 table row (italic 400/500/600/700 → bundled italic twins,
  upright 700 → `ROBOTO_SEMIBOLD_BYTES`, upright 400/500/600 → the face it resolved to before the
  call); a face of another family (`FIRA_MONO_REGULAR_BYTES`) is kept; a second call returns 0
- [ ] T020 [US3] Implement `pin_roboto_italics(db: &mut fontdb::Database) -> usize` in
  `micold-material/src/roboto.rs` per contract C-2 (remove faces whose `families` contains
  `ROBOTO_FAMILY`, whose `source` is not `Source::Binary`, and whose `style` is not `Normal` or
  whose `weight` is not in `ROBOTO_WEIGHTS`; return the count; the shape of `pin_monospace`,
  `mono.rs` line 35); `install_roboto` calls it on `fonts.raw().db_mut()` after loading the faces
  (contract C-3). Re-export it from `micold-material/src/lib.rs` and `client/src/ui/mod.rs`
  beside `install_roboto`. T019 passes; T007 stays green
- [ ] T021 [US3] User guide: `docs/user-guide/appearance-theming.md` *The typeface*: a Roboto
  installed on the computer does not replace the bundled faces for emphasis or bold, so the
  interface looks the same with or without it
- [ ] T022 [US3] Visual pass, quickstart §B B5 (host Roboto Italic in `~/.local/share/fonts`,
  remove it after), through the `visual-pass` skill; evidence in
  `specs/1259-markdown-italic-face/visual-pass/`

**Checkpoint**: emphasis and bold resolve to the bundled faces on a host with its own Roboto.

---

## Phase 5: Polish (close unit)

- [ ] T023 Quickstart §A: `sha256sum assets/fonts/Roboto-*Italic.ttf` equals
  `assets/fonts/PROVENANCE.md`; file sizes (SC-004); the `layout_snapshot.txt` diff scope; record
  the results in quickstart.md
- [ ] T024 Quickstart §C: record what CI's macOS and Windows runners proved (layout snapshot, the
  resolution and pin tests) and what needs a host (the look) in quickstart.md

---

## Dependencies & Execution Order

- Setup (T001–T004) → Foundational (T005–T006) → US1+US2 (T007–T018) → US3 (T019–T022).
- Within a phase, each red test before the task that turns it green: T001 → T002; T005 → T006;
  T007 → T008; T012 → T013; T014 → T015; T019 → T020.
- T009–T011 need T006 only. T016 needs T008, T009 and T013 (italic advances measured). T018 needs
  T010, T011, T013 and T015.
- US3 needs T006 (`install_roboto`, `ROBOTO_WEIGHTS`); it changes no US1 file but `roboto.rs`,
  the re-export lines and the user-guide section.

## Parallel Opportunities

- T003, T004 after T002.
- T010 and T011 after T006, beside T008/T009.
- T012–T013 (link_text) and T014–T015 (core) share no file with T007–T011.

## Implementation Strategy

MVP is M1: the six faces shipped and registered through one installer, `painted` asking for the
slant everywhere but code, the showcase showing bold italic, the user guide saying so. M2 then pins
the faces against host copies. Each merges on its own and leaves `main` green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Draw emphasis in the bundled Roboto italic 🎯 MVP

- **Tasks**: T001–T018
- **Deliverable**: in the application and the showcase, `*emphasis*` in agent replies draws in
  Roboto's own italic at the surrounding weight, `***bold italic***` draws slanted and as heavy as
  bold, and code inside emphasis stays upright; the showcase's markdown sample shows both runs
- **Satisfies**: US1 acceptance scenarios 1–3; US2 acceptance scenarios 1–2; US3 scenario 1;
  FR-001, FR-002, FR-003 (hosts without a Roboto Bold 700), FR-004, FR-005, FR-007, FR-008, FR-009,
  FR-010; SC-001, SC-002 (bundled faces), SC-004
- **Verify**: `cargo test -p micold-client --lib -- roboto link_text`; `cargo test -p micold-client
  --test roboto_font --test layout_snapshot --test layout_apparatus`; `mise run test-core`;
  quickstart §B B1–B4, B6, B7 (evidence in `visual-pass/`); merge check: `mise run gate`
  (`scripts/gate.sh` in a cloud session)
- **Depends on**: —
- **Tier**: full

### M2 — A host Roboto never replaces the bundled faces

- **Tasks**: T019–T022
- **Deliverable**: `pin_roboto_italics` removes host Roboto italic faces and host upright faces at
  unshipped weights, so on a machine with its own Roboto installed emphasis and bold draw exactly
  as on one without
- **Satisfies**: US3 acceptance scenario 2; FR-006; FR-003 and FR-004's exception on every host;
  SC-002, SC-003
- **Verify**: `cargo test -p micold-client --lib -- roboto`; quickstart §B B5; merge check:
  `mise run gate` (`scripts/gate.sh` in a cloud session)
- **Depends on**: M1
- **Tier**: light (one function and its tests in `roboto.rs`, the pattern of `pin_monospace` and its
  host-copy test in `mono.rs`, named in plan D2 and contract C-2)

Polish (T023–T024) changes no code: the close unit does it in the close PR.
