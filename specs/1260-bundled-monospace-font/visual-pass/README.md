# Feature 1260 visual pass (quickstart §B)

Date 2026-10-24. Component showcase (`micold-showcase`, debug build of HEAD) on Xvfb, `ICED_BACKEND=tiny-skia`
(CPU rasteriser, no GPU), 1600x1400. Light and dark scheme. Specimen rendered with ImageMagick from
`assets/fonts/FiraMono-{Regular,Bold}.ttf`. The crops are in this directory.

| Check | Verdict | Evidence |
|---|---|---|
| B1 code block, inline code, tool output (light + dark) | PASS: Fira Mono (same `l`/`r`/`0`/`{}` shapes as the specimen); prose in Roboto | specimen_fira_regular_bold.png, b1_inline_light/dark.png, b1_codeblock_light/dark.png, b7_terminal_light/dark.png |
| B2 diff view and inline diff (light + dark) | PASS: added/removed/context lines in Fira Mono; file labels and captions Roboto | b2_diffview_light/dark.png, b2_diffview_added_light.png, b2_inline_light/dark.png |
| B3 bold monospace | NOT SHOWN: no showcase pose has bold inline code or bold terminal cells (the markdown pose has only bold prose). The Fira Mono Bold specimen is in specimen_fira_regular_bold.png | - |
| B4 same-name host font | PASS: a DejaVu Sans Mono copy renamed to family "Fira Mono" was installed (`fc-match "Fira Mono"` resolved to it). The rust code block (360x120 crop) and the terminal text are pixel-identical to the pre-install run: 0 differing pixels in the code block; the terminal differs in 62 pixels, all right of the text, in the pane's scrollbar/shadow strip. Host copy removed, `fc-cache -f` re-run | b4_codeblock_ref.png vs b4_codeblock_hostfake.png, b4_terminal_hostfake.png |
| B5 CJK / emoji in code | NOT SHOWN: no pose contains them | - |
| B6 two windows | NOT APPLICABLE (showcase) | - |
| B7 terminal pane (light + dark) | PASS: cells in Fira Mono, columns aligned, no gaps or overlap. Bold cells: NOT SHOWN, the pose has none | b7_terminal_light/dark.png |
| B8 interface text | PASS: labels, headings and buttons are Roboto in both schemes | b8_text_light/dark.png |

Not covered: bold weight (B3), CJK/emoji fallback (B5), multiple windows (B6); the real client's session transcript.
Scrolling the showcase is not repeatable on tiny-skia, so B4 was matched with a pixel search of the pre-install crop
inside the post-install frames, not frame against frame.
