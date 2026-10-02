# Contract: the shared picker row — two wrapping lines, highlight in view, row tooltip

**Modules**: `crates/micold-client/src/ui/material/picker.rs`, `ui/material/typeahead.rs`,
`ui/cdk/picker.rs`, `ui/picker_scroll.rs` (NEW), `ui/worktree_form.rs`, `shell/issues.rs` ·
**Research**: R5, R6, R10, R12, R15 · **Data model**: [§7](../data-model.md)

## 1. `Row::details`

```rust
impl Row {
    pub fn details(self, text: impl Into<String>, spans: Vec<Range<usize>>) -> Self;  // NEW
}
```

A row **with** details renders as:

```text
[marker]  label ................ TypeRole::Body,    roles.on_surface,         wraps
          details .............. TypeRole::Caption, roles.on_surface_variant, wraps
```

- Both texts use `Wrapping::WordOrGlyph`: a word wider than the row breaks inside the word
  (FR-004). Neither is truncated, ellipsized or clipped.
- Height is `Shrink` with `density::MENU_ITEM_BASE` as the minimum, and `spacing::XS` above and
  below the text column. Width fills the list.
- The picked-row marker and the row's leading inset are aligned to the label's first line.
- Emphasis uses today's two channels (accent colour and bold) on both lines.
- A disabled row with details dims both lines as a single-line row dims its label.

A row **without** details is built by today's code path: fixed `MENU_ITEM_BASE` height, one line,
`fit_around` truncation. Nothing about it changes (FR-029). `picker_parity.rs`, `menu_anatomy.rs`
and the layout snapshot's branch-picker and `Select` records are the proof.

`TypeRole` and colour roles are stated only inside `material/` (`material_boundary.rs`,
`type_role_call_sites.rs`); the call site passes text and spans.

## 2. The wrapping label

`EmphasisedLabel` gains a wrapping mode. It shapes label and emphasis as **one** paragraph with
`Paragraph::with_spans`, bounded by the row's width, and reports the paragraph's height as its
own. Today's mode (separate paragraphs laid side by side, `fit_around`) stays for single-line
rows. Unit tests in `material/picker.rs`:

- at a width narrower than the text, the laid-out height is more than one line and the width is at
  most the bound;
- a 256-character title without spaces lays out inside the bound;
- emphasised and plain runs keep their order and their text (concatenated runs equal the input).

## 3. Highlight kept in view (FR-007, SC-007)

```rust
// ui/picker_scroll.rs — NEW
pub fn picker_highlight_into_view<M: Send + 'static>() -> Task<M>;
```

- `menu_element` gives the highlighted row, and only it, the widget `Id` `PICKER_HIGHLIGHT`.
- The operation is two passes, as `focus::into_view`: find that `Id`'s bounds, then scroll the
  enclosing scrollable by `focus::delta_into_view(row, viewport)` (reused). A row already wholly
  visible causes no scroll. A row taller than the viewport is aligned to its top.
- The shell chains it after `FormMsg::IssueHighlightMoved` (`main.rs` where it routes issue
  messages; `shell/issues.rs`). The reducer stays render-free and unchanged.
- Up and Down still move by one issue: the reducer's index arithmetic is untouched (FR-007).
- Scope: the issue picker only. The branch picker and `Select` are not wired (FR-029; ledger
  follow-up).

Test `tests/picker_highlight_into_view.rs`: a list of 12 issues whose rows are one to five lines
tall, in a list eight base rows high. After each of 11 Down presses and 11 Up presses, with the
operation applied, the highlighted row's bounds lie inside the scrollable's viewport.

## 4. `Row::tooltip` and `Row::key`

```rust
impl Row {
    pub fn tooltip(self, text: impl Into<String>) -> Self;  // NEW
    pub fn key(self, key: u64) -> Self;                     // NEW
}
pub const ROW_TOOLTIP_REST: Duration = Duration::from_secs(3);
pub const ROW_TOOLTIP_LINES: usize = 3;
```

`menu_element` wraps a row that has a non-empty tooltip text in

```rust
material::Tooltip::new(row, text, roles)
    .after_rest(ROW_TOOLTIP_REST)
    .max_lines(ROW_TOOLTIP_LINES)
    .subject(key)          // when a key is set
```

- An empty text, or no `tooltip` call, wraps nothing (FR-020).
- The tooltip's text is exactly the text passed: no number, title, reporter, labels or heading
  (FR-019).
- The names are generic (`typeahead_is_generic.rs`): nothing in `material/` or `cdk/` says
  "issue".

The issue picker (`ui/worktree_form.rs` `issue_picker`) builds, for every issue from any source
(FR-006):

```rust
let e = issue.emphasis(&matched.spans);
TypeaheadRow::new(issue.title_line(), e.title)
    .details(issue.details_line(), e.details)
    // from M5:
    .key(issue.number)
    // and only when !issue.description().is_empty():
    .tooltip(issue.description())
```

and its placeholder becomes `"Search by number, title, label or reporter"` in M3 (FR-011).

## 5. A tooltip inside the floating list (FR-023)

- `cdk::picker::Menu` implements `Overlay::overlay`, forwarding to its content's
  `Widget::overlay` with the list's layout and translation. The row tooltip's panel is then an
  overlay above the list.
- Side: `TooltipPosition::Bottom`. The existing `place` flips to the top when the panel does not
  fit below and never covers the trigger while either side fits.
- The panel takes no pointer or keyboard input: a click on the row under an open tooltip picks
  the issue, and focus stays in the search field.
- `one_overlay_implementation.rs` and `overlay_stacking.rs` stay green: the forwarding adds no
  second overlay implementation.

Gate `tests/gates/picker_row_tooltip_clears_its_row.rs` (beside `tooltip_clears_its_row.rs`):
with a real hover held past the rest delay on the first row, the last row, and a row at the
list's lower edge, exactly one panel opens; it lies inside the window, does not intersect its
row, and is at most three `Caption` lines plus the panel's padding tall.

## 6. Showcase and layout coverage (FR-028)

- The `Typeahead` entry of the showcase (`showcase/sections/controls.rs`) gains a pose: a list
  with two-line rows of differing height — a short row, a row with a long title, a row with many
  labels — one of them highlighted and one picked. `showcase_completeness.rs` and
  `showcase_captions.rs` hold it.
- `tests/support/covered_states.rs` gains issue lists with a 256-character title, 20 labels and
  mixed heights, at the default window and at a narrow one. `layout_text_overflow.rs` and
  `gates/containment.rs` then fail on any text record clipped or outside the list, and NEW
  `gates/issue_rows_show_all_text.rs` fails when a row's text records, joined, lack any part of
  the title, the reporter or a label (SC-001).
  `fixtures/layout_snapshot.txt` is regenerated in the same change.
