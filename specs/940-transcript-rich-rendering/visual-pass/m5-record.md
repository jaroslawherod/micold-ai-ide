# M5 visual pass — quickstart Part B (T042)

2026-10-21, cloud container. No logged-in `claude` exists here, so the pass drives the **real
window** (`ui::view`, the transcript pane through `BlockRenderers::RICH`) from the recorded
fixtures `markdown-turn` + `edit-turn` (the `session-transcript-rich` covered state), with the
highlight jobs answered as the shell does, rasterised headless by tiny-skia at 1280 dp wide in
both schemes. A scratch test (not committed) drew the frames; no pointer or keyboard was driven.

| Part B step | Result | Evidence |
|---|---|---|
| 1 Reply formatted; streaming; table | Pass. Headings, bold, strikethrough, inline code, nested bullets, numbered list, task list marks, quote bar, rule, link, a table with a header row and ruled cells; no formatting punctuation shown. Mid-stream (first 60 events) the reply is already formatted and the open block (`### Chec`) is drawn as a heading. Long cell wrapping: the `transcript_rich_table_wraps` gate (170 dp). | `m5-reply-light.png`, `m5-reply-dark.png`, `m5-streaming-light.png` |
| 2 Code blocks | Pass. `rust` coloured in both schemes; `teapot-lang` plain monospace with its tag; the untagged block plain with no tag; each on its own surface with a copy control. Sideways scroll vs wheel: not driven (the `code_block_wheel` gate covers it). | `m5-reply-*.png` |
| 3 Copy | Not driven (no input here); covered by M3/M4 copy tests. | — |
| 4 Links | `Example` drawn as a link; activation not driven (M3/M4 link tests). | `m5-reply-light.png` |
| 5 Edit/Write cards | Pass. Folded: path and `+4 −0`, `+1 −1`, `+3 −2` beside the status. Unfolded: `+`/`−` signs, no line numbers, "New content" on the Write, "Replaces every occurrence" on the replace-all hunk. The fixtures hold no MultiEdit (the "one hunk per edit" case is covered by `transcript_rich_renderers`). | `m5-folded-light.png`, `m5-edit-cards-*.png` |
| 6 Thinking; Read result | The fixture's thinking text is empty (redacted), so the formatted muted thinking is covered by `text_and_thinking_are_drawn_from_the_cached_document` only. Tool results stay plain monospace. | `m5-edit-cards-light.png` |
| 7 Light/dark; greyscale | Pass. Highlighting and diff fills follow the scheme; in greyscale the added and removed lines read by their sign. | `m5-*-dark.png`, `m5-diff-greyscale.png` |
| 8 Two windows | Not driven (one headless window); the copy state is per window by construction (M3 `RichCache` lives in the window state). | — |

Headless artefact, not a finding: card titles (`Write`, `Edit`, `Thinking`) are faint because
iced draws a button as `Status::Disabled` until its first redraw event (`iced_widget` 0.14.2
`button.rs:376`, `self.status.unwrap_or(Status::Disabled)`); the M4 live pass
(`m4-edit-card-meta-light.png`) shows the title at full strength.

Findings: none.
