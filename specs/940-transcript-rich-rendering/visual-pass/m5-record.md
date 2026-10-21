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
| 3 Copy | Not driven (no input here). Covered render-free: `a_copy_writes_the_code_byte_for_byte`, `the_read_back_decides_copied_or_failed_and_asks_for_the_timer` (M3) and the `CodeBlock` copied/failed poses (M4 pass). | — |
| 4 Links | `Example` drawn as a link; activation not driven (M3/M4 link tests). | `m5-reply-light.png` |
| 5 Edit/Write cards | Pass. Folded: path and `+4 −0`, `+1 −1`, `+3 −2` beside the status. Unfolded: `+`/`−` signs, no line numbers, "New content" on the Write, "Replaces every occurrence" on the replace-all hunk. The fixtures hold no MultiEdit (the "one hunk per edit" case is covered by `transcript_rich_renderers`). | `m5-folded-light.png`, `m5-edit-cards-*.png` |
| 6 Thinking; Read result | Pass. The recorded fixtures' thinking is empty (redacted), so a synthetic assistant event with a markdown thinking block was applied to the same window: unfolded, the thinking is formatted (heading, bold, numbered list, inline code, quote) and muted (its text ink `#49454E` against the reply's `#1C1B1E` in light). Tool results stay plain monospace. | `m5-thinking-light.png`, `m5-thinking-dark.png`, `m5-edit-cards-light.png` |
| 7 Light/dark; greyscale | Pass. Highlighting and diff fills follow the scheme; in greyscale the added and removed lines read by their sign. | `m5-*-dark.png`, `m5-diff-greyscale.png` |
| 8 Two windows | Not driven on screen (one headless window). Render-free instead: `features_transcript_rich::sessions_and_windows_share_nothing` gives two windows their own `RichCache` over one session and asserts they derive the same document, that a copy in one leaves the other's copy state empty, and that forgetting in one leaves the other. | test |

Headless artefact, not a finding: card titles (`Write`, `Edit`, `Thinking`) are faint because
iced draws a button as `Status::Disabled` until its first redraw event (`iced_widget` 0.14.2
`button.rs:376`, `self.status.unwrap_or(Status::Disabled)`); the M4 live pass
(`m4-edit-card-meta-light.png`) shows the title at full strength.

Findings: none. Not seen on screen: pointer and keyboard input (steps 3, 4, the sideways scroll of 2) and a second window (8); each is held by the tests named above.
