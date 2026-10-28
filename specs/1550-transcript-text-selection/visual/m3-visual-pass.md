# Visual pass: M3 selectable leaves

Headless (tiny-skia, the shipped faces) render of the four selectable leaves with a range
selected, light (left) and dark (right): `SelectableText` across wrapped rows, a muted `Text`,
a `CodeBlock` (line 1 from `ain`, line 2 through its tab and up to `hello, `), and an
`InlineDiff` (the path's `src`, a removed line's tail, a whole added line, a context line's head).
Rendered by a scratch test that was not committed; the image is `m3-selectable-leaves.png`.

| Check | Result |
|---|---|
| The selected bytes, and only they, are on `primary_container`, wrapped rows included | pass |
| Selected glyphs in `on_primary_container`, readable on every surface (prose, code surface, both diff tints) | pass |
| Code and diff selections follow raw bytes: a tab is highlighted as the spaces it draws; the gutter and padding are not selected | pass |
| The fill is distinct from each surface; light is the faintest (lavender on the grey code surface), still clearly visible | pass |
| Nothing else moves: unselected text and rows sit where the unselected leaves draw them | pass (pixel gate tests) |

Not covered here: the real client under a window manager. Nothing in the transcript uses these
leaves until M4a/M4b, whose visual pass covers selection in the running app.
