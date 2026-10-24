# TDD cycle log: 1260 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T006, T008 | `FIRA_MONO_{REGULAR,BOLD}_BYTES` parse, family `Fira Mono`, weight 400/700, fixed-pitch, 600/1000 em for printable ASCII | tests and constants written in one pass (the constants are the `include_bytes!` the test names); red proven afterwards by mutation (see below) | `cargo test -p micold-client --lib -- mono`: `the_regular_bytes_are_fira_mono_regular`, `the_bold_bytes_are_fira_mono_bold` ok |
| T007, T008 | `pin_monospace` removes a same-name file face loaded first, keeps other families, sets the monospace family, resolves 400/700/italic to the bundled faces, idempotent | written with the function in one pass; red proven afterwards by mutation (see below) | `pin_monospace_drops_a_host_copy_and_resolves_to_the_bundled_faces` ok |
| T009, T010 | the in-crate test renderer resolves `Font::MONOSPACE` (400, 700) to the bundled bytes | before T010: `the_global_font_system_resolves_monospace_to_bundled_fira_mono ... FAILED` (mono.rs: "does not resolve to the bundled Fira Mono Regular") | after `install_monospace` in `test_support::renderer`: ok; `-- mono terminal` 155 passed |
| T013, T010 | terminal cells (plain, bold) resolve to bundled Regular/Bold; `CellMetrics::new(s).width` = Fira Mono advance at 12/13/14 | before T010: `cell_fonts_resolve_to_the_bundled_fira_mono_faces ... FAILED`; `cell_width_equals_the_bundled_face_advance` passes first run (characterization: the 0.6 ratio already equals Fira Mono's advance, research R7) | after T010: both ok |
| mutation (T006–T008) | — | `FIRA_MONO_BOLD_BYTES` pointed at the Regular file and `remove_face` skipped: `the_bold_bytes_are_fira_mono_bold ... FAILED` ("bundled Fira Mono Bold: weight"), `pin_monospace_drops_a_host_copy_... FAILED` ("a Fira Mono face not loaded from memory survived") | restored source: all `mono` tests ok |
| T011, T012 | application and showcase call `install_monospace` before `iced::application` | glue | full gate green; visual pass §B |
| T014 | user guide | docs | — |
| T016, T018 | `Family::Monospace` at Regular and Bold resolves to the shipped bytes and `Font::MONOSPACE` measures with each face's own advances | before T018: `the_monospace_face_parses_and_is_the_face_that_measures ... FAILED` ("Fira Mono Bold: Family::Monospace resolved to ... at weight 400") | after `install_monospace` in `renderer`: 12 `layout_apparatus` tests ok |
| T017 | one `FiraMono*.ttf/.otf` in the repository, under `assets/fonts/` | passes after T001; with a temporary `tests/fixtures/FiraMono-Regular.ttf` it FAILED naming that path (copy deleted) | `there_is_one_copy_of_fira_mono_in_the_repository` ok |
| T019 | `layout_snapshot` unchanged (research R4) | — | 105 passed, fixture untouched |

**Order note (tdd-verify finding 2, T032).** T013's red run was made in the working tree before
T010 was written, but the milestone was committed after the fact: in history `e5048827` (T013's
test) comes after `3f7edb28`, which already holds T010. The history does not corroborate that
order; the red output quoted above is the only evidence of it. T016 is not affected: its test and
T018 (`install_monospace` in `tests/support/layout.rs`) land together in `56efcebc`, so the history
neither shows nor contradicts its red-before-green order.

## M3

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T020 | `repo_root`, `manifest_path`, `deb_assets`, `rpm_assets` shared from `tests/support/packaging.rs` | refactor, no behaviour change | `packaging_excludes_showcase` and `deb_ships_no_service_units` green before and after |
| T021, T024–T026 | deb, rpm, `.app` script and `.iss` each ship the four licence-set files to their `fonts` licence directory | with `Cargo.toml`, the `.iss` and `macos-bundle.sh` reverted: `every_release_bundle_ships_the_font_licence_set ... FAILED`, 16 lines, one per file and bundle ("the deb ... does not ship assets/fonts/LICENSE-FiraMono-OFL.txt to usr/share/doc/micold-ai-ide/fonts/") | after T024–T026: 6 passed |
| T022, T026 | a licence-set file is allowed in the `.iss` only with `DestDir` under `{app}\licenses\`; an unlisted file there still fails | mutation (licence allowance removed from `windows_violations`): `a_windows_installer_that_ships_a_font_licence_under_licenses_passes ... FAILED`, `the_windows_installer_contains_no_showcase ... FAILED` | restored: 24 passed |
| T023, T025 | the staged `.app` holds the four files in `Contents/Resources/licenses/fonts/`; a missing one fails with `missing font licence: assets/fonts/<file>` | before T025: `macos-bundle.test.sh` 25 cases, 5 failures (four files missing, "a missing font licence is named: exited 0") | after T025: 25 cases, 0 failures |
| T027 | `.gitattributes` packaging set | config | `git check-attr` shows `micold-packaging: set` on the four files; `classify-change.test.sh` 69 passed |

## Close

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T034 | `Font::MONOSPACE` shapes `a` with the bundled Fira Mono and U+E5CD (`Icon::Close`, a Material Symbols icon Fira Mono lacks, per both cmaps) with another face and a non-zero glyph (FR-004) | written after FR-004's code (characterization); mutation: the fallback assertion flipped to require Fira Mono failed with "'\u{e5cd}' stayed on Fira Mono, which lacks it: no fallback face (FR-004)" | `cargo test -p micold-client --lib -- mono`: 8 passed |
