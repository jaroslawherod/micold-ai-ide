# Contract: Edit, MultiEdit and Write diffs (`micold_core::structured::edit_diff`)

Render-free. Types: [data-model.md](../data-model.md) *Edit diff*. Decisions: research R6, R7.

## Surface

```rust
pub fn edit_diff(tool: &str, input_json: &str) -> Option<EditDiff>;
```

Called only for a card whose input is complete (`ToolCard.streaming == false`).

## Input shapes (Claude Code's names)

| Tool | Required | Optional | Hunks |
|---|---|---|---|
| `Edit` | `file_path`, `old_string`, `new_string` (strings) | `replace_all` (bool) | one |
| `MultiEdit` | `file_path` (string), `edits` (non-empty array of objects with `old_string`, `new_string`) | per edit `replace_all` | one per edit, in order |
| `Write` | `file_path`, `content` (strings) | — | one, all `Added`, note `NewContent` |

Anything else → `None`: another tool name, input that is not a JSON object, a missing required
field, a field of another JSON kind, an empty `edits`. Unknown extra fields are ignored.

## Rules

- Lines split as `review::diff` does: `\n` and `\r\n` both end a line; a trailing newline starts
  no extra line; a CRLF-only difference is no change.
- Line diff with `similar` (Myers, 50 ms deadline); equal → `Context`, delete → `Removed`, insert
  → `Added`.
- `replace_all: true` → note `ReplaceAll`; old == new → note `NoChange` and no lines; a Write of
  `""` → note `EmptyWrite`.
- Only the final newline differs → the lines are equal and `old_no_newline != new_no_newline`.
- No line numbers anywhere; a hunk is named by `ordinal` of `of`.
- `added`/`removed` sum the hunks' lines.

## Guarantees (tested in `crates/micold-core/tests/edit_diff.rs`)

- **E-1 Round trip (SC-003)**: for every Edit/MultiEdit/Write in the fixtures and the synthetic
  set, each hunk's `Context`+`Removed` lines equal the old text's lines and `Context`+`Added` the
  new text's.
- **E-2 Fallback (FR-018)**: every malformed input of the table above gives `None`, never a panic.
- **E-3 Counts (FR-017)**: `added`/`removed` equal the counts of the hunks' lines.
