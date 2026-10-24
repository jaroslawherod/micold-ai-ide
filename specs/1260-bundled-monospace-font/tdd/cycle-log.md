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
