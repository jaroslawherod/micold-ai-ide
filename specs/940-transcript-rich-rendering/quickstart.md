# Quickstart: validating Structured Session Rich Rendering (#940)

Prerequisite for everything below: #939's milestones M1–M6 merged (research R14).

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Markdown model | `cargo test -p micold-core --test markdown_subset --test markdown_stream` and `--lib markdown` | M-1–M-6 of [contracts/markdown-model.md](contracts/markdown-model.md): every subset element, literal fallbacks, settled blocks final at every prefix, incremental = fresh, finished = streamed (SC-001, SC-002) |
| Edit diffs | `cargo test -p micold-core --test edit_diff` | E-1–E-3 of [contracts/edit-diff.md](contracts/edit-diff.md): round trip on every fixture call, malformed → fallback, counts (SC-003) |
| Render cache, copy, links | `cargo test -p micold-client --test features_transcript_rich` | refresh derives changed revisions only and prunes; isolation across sessions; copy `Pending → Copied/Failed → Idle` by ticket, a stale expiry ignored, clipboard text = code byte for byte (SC-005); `OpenLink` emits an open for web/mail only; show in full per session |
| Renderer swap | `cargo test -p micold-client --test transcript_rich_renderers --test transcript_renderer_seam` | `RICH` changes only block content and the FR-017 meta; results and malformed inputs equal `PLAIN` (FR-018, FR-021, FR-022, SC-007) |
| Shared components | `cargo test -p micold-client --test showcase_completeness --test material_builder_api --test layout_snapshot` and `--lib ui::material` | `MarkdownView`, `CodeBlock`, `InlineDiff`, `LinkText`, `DisclosureCard::meta` have builders, showcase entries and their gates (contrast both schemes: SC-006) |
| Budget | `cargo test --release -p micold-client --test transcript_open_budget` (no `--ignored`: in release the budgets run by default) | the rich variant: 5,000 events with markdown, code and diffs, every card open, first draw < 1 s, each delta < 100 ms (SC-004) |
| Everything else unchanged | `scripts/gate.sh` | green (SC-007) |

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` on `PATH`; the dev client with a structured session (#939).

1. Ask for a reply using headings, bold/italic, nested lists with a task list, a quote, a link, a
   table and three code blocks (`rust`, an unknown tag, no tag). While it streams: the reply is
   formatted as it grows and text above the last block does not move (FR-005, FR-006). When done:
   no formatting punctuation visible; the table has a header row and wraps a long cell (FR-003).
2. Code blocks: the `rust` one is coloured, the others plain monospace with the tag shown; a long
   line scrolls sideways inside the block while the wheel still scrolls the transcript (FR-009,
   FR-010).
3. Copy each code block with the pointer, then with Tab and Enter; paste into an editor: exactly
   the code. The control shows "Copied" for about two seconds (FR-011, FR-012).
4. Activate an `https` link (opens in the browser) and Tab to it and press Enter; a relative link is
   plain text and does nothing (FR-004).
5. Ask Claude to write a new file, edit it with replace-all, and make two edits in one MultiEdit:
   each folded card shows the path and `+N −M`; unfolded, a diff with `+`/`−` signs, no line
   numbers, "Replaces every occurrence" on the replace-all hunk, one hunk per edit (FR-013–FR-017).
6. Unfold a thinking card: formatted, muted (FR-002). A `Read` result stays plain (FR-021).
7. Switch light/dark: highlighting and diff colours follow (US3 s6); take greyscale screenshots of
   a diff: added and removed lines readable by their sign (SC-006).
8. Two windows on the same session: same formatting; a copy confirmation in one does not appear in
   the other (Principle II).

Record the pass with the `visual-pass` skill into `specs/940-transcript-rich-rendering/visual-pass/`.

## Results (close, 2026-10-22)

- **Part A**: `scripts/gate.sh` green on main at `18e9cf09` and again on the close's tree at `77e6071e` (each 9,440 passed, 0 failed, 17 ignored); the markdown, edit
  diff, rich cache, renderer and shared-component rows run inside it. The release budget was run in
  M5 (2026-10-21): open with every card open `RICH` 118 ms (budget 1 s), slowest streamed markdown
  delta 2.7 ms (budget 100 ms); not re-run at close.
- **Part B**: recorded in M4 and M5 headless visual passes (`visual-pass/m4-*.png`,
  `visual-pass/m5-record.md`): steps 1, 2, 5, 6 and 7 seen; steps 3, 4 and 8 covered by
  `features_transcript_rich.rs` (copy, links, per-window copy state) and not driven on screen (no
  pointer input or second window headless). Not run against a logged-in `claude`.
- Known gaps, logged as follow-ups in `autopilot.md`: italic is drawn upright (no italic face
  bundled); code and diffs use the host's monospace face (the app bundles none).
