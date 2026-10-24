# Contract: the guide's review shortcut table (FR-004)

Checked by `crates/micold-core/tests/review_keys_doc.rs`.

## In the guide

`docs/user-guide/reviewing-changes.md`, under the keyboard heading:

```markdown
<!-- review-shortcuts:start -->
| Key | Does |
|---|---|
| `j` | <sheet text of the entry> |
| `Ctrl+Shift+Enter` (`Cmd+Shift+Enter` on macOS) | <sheet text of the entry> |
<!-- review-shortcuts:end -->
```

- One body row per binding, in the key table's order.
- Key cell: the chord in backticks, modifiers in the order `Ctrl+Alt+Shift+<key>`; when the binding
  uses the platform's primary modifier, the cell adds ``(`Cmd+…` on macOS)``. A key with no
  modifier has no suffix.
- Does cell: the binding's sheet text, verbatim.
- Text outside the markers is free prose.

## In the test

| Test | Fails when |
|---|---|
| `the_guides_table_is_the_review_key_table` | a row is missing, extra, out of order, or its key or text differs; message lists both row sets |
| `each_primary_chord_names_its_macos_form` | a binding with the primary modifier has a key cell without the macOS suffix, or one without it has a suffix |
| `the_markers_are_there` | either marker is missing or `end` comes before `start`; message names the marker |

### If the table is thinner

- **No sheet text in the table** (the `?` sheet keeps its wording in the client): the core test
  compares the `Key` cells only, in order, and a client test in `crates/micold-client/tests/`
  compares each `Does` cell with the sheet's text for that key, using the same marker helpers.
- **No primary-modifier flag**: the suffix is derived from the modifier set: a binding whose
  modifiers include the platform's primary one (iced's `command`, `Ctrl` on Linux / Windows) gets
  ``(`Cmd+…` on macOS)``; a binding with only `Shift` or no modifier gets none.

The marker and row helpers come from `crates/micold-core/tests/support/marked_table.rs`, shared with
`terminal_only_doc.rs` (research R3).

## Attribute

`.gitattributes`, after `docs/**`:

```text
# Read by a test, so a test input rather than documentation: this page's review shortcut table
# must equal the review key table, which `crates/micold-core/tests/review_keys_doc.rs` checks
# (feature 1276, FR-004). Must stay after `docs/**`, which sets the attribute.
/docs/user-guide/reviewing-changes.md -micold-docs
```
