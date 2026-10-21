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
