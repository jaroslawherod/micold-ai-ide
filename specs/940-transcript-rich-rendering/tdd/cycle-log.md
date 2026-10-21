# TDD cycle log — 940 transcript rich rendering

No `tdd/test-list.md` was planned for this feature; the loop follows tasks.md's test-first order:
each test task written before its implementation task, run red, then made green.

## M1 (T001–T012) — markdown model

- **Red**: T006 (fence tests), T007 (`tests/markdown_subset.rs`) and T008
  (`tests/markdown_stream.rs`) written against a stub `Document` (`parse` returns no blocks,
  `update` does nothing, `code_blocks` is empty). First run: `markdown_stream` 3 passed, 1 failed —
  the M-2/M-3 checks held trivially on empty documents, so both gained "the text has its blocks and
  they settle" assertions; rerun 1 passed, 3 failed. `markdown_subset` 2 passed, 16 failed; the two
  code-id tests then gained count assertions so an empty document fails them too. The fence tests
  were written beside `fence_token`; their red was shown afterwards (review B M1) against a stub
  returning the tag unchanged: `cargo test -p micold-core --lib markdown::fence` 1 passed, 4 failed
  (cut, lower-case, aliases, empty tag), then green again with the implementation restored.
- **Green**: T009–T012 implemented. Failures on the way, each a real gap: a reference definition
  later in the same slice resolved the link (definitions are now cuts of their own); an image's
  text was split by bare-address detection (image spans are skipped); the fixture's two ordered
  lists were one list (same delimiter; the loose one now uses `)`); the first block's start became
  a second cut (`cuts` now merges it into `0`); a fence info string's rest (`{.numbered}`) and an
  end-of-line list marker counted as hidden text in the check (they are fence and list syntax).
  `markdown_subset` 18/18, `markdown_stream` 4/4, lib `markdown` 10/10.
- **Refactor**: inline event arms collapsed into one push; `cargo fmt`; clippy `-D warnings` clean
  on `micold-core`.
- **Review A fixes** (red → green as regression tests in `markdown/mod.rs`): a definition inside a
  quote or list item no longer cuts the container and is shown in it; an image inside a link keeps
  the link; a fence a container closes is not `open`; an empty task item has no empty paragraph;
  `lines` counts trailing blank lines. Lib `markdown` 15/15, `markdown_subset` 18/18,
  `markdown_stream` 4/4.
- **Notes**: cycles were batched per milestone, not one behaviour per commit.

## M2 (T013–T015) — edit diff

- **Setup**: `edit-turn.jsonl` recorded with `scripts/capture-stream-json.sh` (CLI 2.1.306). That
  CLI offers no `MultiEdit` tool (the first take's model said so and skipped it), so the
  recording is a `Write`, an `Edit` with `replace_all: true` and a multi-line `Edit` sharing a line;
  MultiEdit is covered by the test's synthetic set (README row, ledger decision).
- **Red**: T014's `tests/edit_diff.rs` (15 tests) against the `todo!()` stub: 0 passed, 15 failed,
  each in the stub's `todo!()` (every test calls `edit_diff` before its first assertion).
- **Green**: T015 implemented (`serde_json` shape checks, `review::diff::split_lines`,
  `similar::TextDiff` Myers with a 50 ms timeout); 15/15 first run.
- **Refactor**: the multi-edit test's tuple got a `Shape` alias (clippy `type_complexity`);
  `cargo fmt`; clippy `-D warnings` clean on `micold-core`.

## M3 (T016–T024) — the window's rich cache

- **Red**: T017–T019's `tests/features_transcript_rich.rs` (16 tests) against a `transcript_rich`
  stub whose methods were `todo!()` (types real, `TranscriptMsg` routing written):
  `cargo test -p micold-client --test features_transcript_rich` 1 passed, 15 failed, each in the
  stub's `todo!()`; the one that passed was T019's link test, whose routing in
  `structured_transcript::State::update` was written with the new variants (test-after). T016
  against `knows` returning `false` and `fills` still `[Rgb; 3]`: `cargo test -p micold-client
  --lib ui::syntax` failed to compile, ``can't compare `[Rgb; 3]` with `[Rgb; 4]` ``. T020's source
  scan of `interpret` was written after its arm (test-after): 16 passed on first run.
- **Green**: T021 (`knows` over two-face's grammar set, the grammars iced_highlighter builds
  with; `fills` gains `surface_container_high`), T022 (`RichCache`), T023 (routing, app state,
  `BlockCache` re-export), T024 (shell glue). First run 11 passed, 5 failed: three were test
  errors and fixed in the test — a code block's `code` has no final line break (M1's model; the
  expected copy texts had one), `markdown-turn` holds one assistant text entry (the "several
  entries" checks now also replay `edit-turn`), and a second `Transcript::new()` restarts
  revisions so the "entry left" case reused a revision (now one transcript rebuilt from a fresh
  history, as a session's is). Then 16/16.
- **Refactor**: `cargo fmt`.
- **Review B M3 follow-up**: T020's red shown afterwards: with the `ClipboardWriteChecked` arm
  moved into `interpret`'s `Task::none()` list, the built `clipboard_request` test binary run on
  `a_checked_write` fails, `panicked at ... clipboard_request.rs:247:47: the arm writes`; restored,
  1 passed. T019's link test stays test-after (its routing came with the variants).

## M4 (T025–T034) — shared rich components

- **Order, disclosed**: `CodeBlock` and `LinkText` were drafted before their tests (test-after);
  `InlineDiff`, `MarkdownView` and `DisclosureCard::meta` were written beside theirs. So the red
  below was shown against stubs: each new component's `From` impl began with `todo!("stub")` and
  `meta` dropped its text, with the builders and types real.
- **Red**: `cargo test -p micold-client --lib ui::material`: 754 passed, 17 failed — the 5
  `code_block` tests, 3 `link_text`, 3 `inline_diff`, 4 `markdown_view`, the copy control's
  `ripple_clipping` case (each `panicked at …: not yet implemented: stub`) and
  `disclosure_card::a_meta_is_never_cut_and_the_summary_gives_way`. `cargo test -p micold-client
  --test layout_snapshot`: 73 passed, 5 failed — `code_block_fits`, `code_block_wheel`,
  `inline_diff_marker_fits` in the stub, `an_edit_cards_meta_is_never_ellipsized` ("the meta is
  drawn whole" not found), and `the_layout_matches_the_committed_fixture` (session-transcript-pane:
  a tool card's status node 12.0 → 43.8 wide). That last one was a real bug, not the stub: `meta`
  wrapped every card's trailing status in a new row; fixed so a card without a meta passes its
  status through unchanged. T026's `composition_contrast` and `style_outline_discipline` cases
  passed against the stubs (they read the style and colour functions, which were real): test-after.
- **M4 review fixes (2026-10-21).** Review A r1 and the visual pass, each red first:
  `link_text::a_link_split_across_styles_is_one_focus_stop` (left `Some(2)`, right `Some(1)`),
  `markdown_view::a_long_number_fits_its_gutter` ("1000." ran into its item) and
  `inline_diff::the_no_newline_note_is_not_drawn_as_a_line` (the note was painted in
  `Font::MONOSPACE`, padded as a line). Green after: consecutive spans of one link collapse into one
  stop; `number_gutter` widens a numbered list's gutter to its longest number; the note is a
  marker plus a caption on its side's tint.
