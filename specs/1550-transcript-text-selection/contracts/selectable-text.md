# Contract: `SelectableText` and `SelectionArea` (shared components, `micold-material`)

New files `crates/micold-material/src/selectable_text.rs` and `selection_area.rs`, exported from
`lib.rs`, each with a showcase entry (Principle VIII builder API terminating in `.into()`).

## `SelectableText<'a, M, K>`

```rust
SelectableText::new(spans, roles)        // the same span inputs LinkText takes
    .role(TypeRole::Body).monospace().muted(bool).wrapping(Wrapping::WordOrGlyph)
    .slot(key: K)                        // the unit's identity, echoed in on_hit
    .selected(range: Option<Range<usize>>)
    .on_hit(f: impl Fn(K, Hit) -> M)
    .on_link(f: impl Fn(String) -> M)    // as LinkText
    .into()
```

`Hit { offset: usize, click: Click /* Single | Double | Triple */, line: Range<usize> }`.

- **T-1 Hit**: a left press on a glyph publishes `on_hit` with the byte offset of the grapheme
  boundary nearest the pointer, in the widget's whole text (line start + in-line index; one
  paragraph per hard line, research R5). It never captures the event.
- **T-2 Drag**: while its own left button is down, a cursor move or a redraw with the cursor over it
  publishes `on_hit` again when the offset changed; a cursor right of it, on one of its lines,
  answers that line's end (#1550 M6: a short diff or code line ends before its row).
- **T-3 Visual line**: `line` is the byte range of the visual row under the pointer.
- **T-4 Highlight**: with `selected = Some(r)`, the bytes of `r` are drawn on `primary_container` in
  `on_primary_container` on every surface, wrapped rows included; with `None` it draws exactly as the
  non-selectable leaf would (pixel-equal, a gate). FR-020.
- **T-5 Links**: a press on a link span publishes the link message and no `on_hit` (FR-004).
- **T-6 Parity**: a widget operation reports `(slot, text)`; the text equals what is drawn.

Adopted through a `.selectable(slot, range, on_hit)` builder on `Text`, `LinkText`, `CodeBlock`
(each code line; full lines even when scrolled sideways), and `InlineDiff` (path, hunk labels, line
text without the gutter). Callers that do not call it are unchanged.

## `SelectionArea<'a, M>`

```rust
SelectionArea::new(content)
    .id(widget_id)                       // focus target
    .dragging(bool)
    .on_press(f: impl Fn(u8 /* clicks */) -> M)
    .on_edge(f: impl Fn(Edge /* Up | Down */) -> M)
    .on_release(m: M)
    .on_context(f: impl Fn((u16, u16)) -> M)
    .on_focus_change(f: impl Fn(bool) -> M)
    .into()
```

- **A-1 Controls first**: a left press a child captured (button, toggle, copy control, link,
  permission control, "jump to latest") publishes nothing from the area. FR-004.
- **A-2 Focus**: an uncaptured left press inside takes keyboard focus and publishes `on_press`.
  FR-007. The children see each mouse event at its own place (a move where it went, a press or
  release where the last move left the pointer), not where the runtime's batch of events ended,
  so a fast drag starts at the press (#1550 M6).
- **A-3 Edge scroll**: while `dragging` and the cursor is above (below) the bounds, children see the
  cursor clamped to the top (bottom) row and the area publishes `on_edge(Up)` (`Down`) each frame it
  requests. FR-003.
- **A-4 Context**: a right press anywhere inside, on text or a control, publishes `on_context(origin)`
  and no `on_press`. FR-013.
- **A-5 Release**: left release publishes `on_release` and nothing else. FR-012.
