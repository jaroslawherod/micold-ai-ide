# Contract: the markdown model (`micold_core::markdown`)

Render-free and free of I/O. Types: [data-model.md](../data-model.md) *Markdown document*.
Decisions: research R1–R4.

## Surface

```rust
pub fn fence_token(tag: &str) -> Option<String>;

impl Document {
    pub fn parse(text: &str, streaming: bool) -> Document;
    pub fn update(&mut self, text: &str, streaming: bool);
    pub fn blocks(&self) -> &[Block];
    pub fn settled(&self) -> usize;
    pub fn code(&self, id: CodeId) -> Option<&CodeBlock>;
    pub fn code_blocks(&self) -> impl Iterator<Item = (CodeId, &CodeBlock, bool)>; // bool: settled
}
```

## Rules

| Input | Model |
|---|---|
| ATX or setext heading | `Heading { level }` |
| paragraph; raw HTML block | `Paragraph` (HTML as its literal text) |
| `**b**`, `*i*`, `~~s~~`, `` `c` `` | span `Style { bold / italic / strike / code }` |
| `[t](u)`, `<u>`, bare `http(s)://…` | span with `Link { target: u, activatable: classify(u) ∈ {Web, Mail} }` |
| `![d](u)` | span `"d (u)"`, no link, nothing fetched |
| inline HTML, footnote, math | literal text |
| `-`/`*`/`+` list, `1.` list, `[ ]`/`[x]` | `List { start, tight, items[task] }`, nested |
| `>` | `Quote { blocks }` |
| `---`, `***` | `Rule` |
| fenced or indented code | `Code(CodeBlock { tag, token, code, open })` |
| GFM table | `Table { align, header, rows }`; before its separator row arrives, a `Paragraph` |
| unmatched `**`, unclosed fence | literal `**`; `Code { open: true }` holding the text so far |

## Guarantees (tested in `crates/micold-core/tests/markdown_subset.rs` and `markdown_stream.rs`)

- **M-1 Coverage (SC-001)**: for the subset fixture, every element maps to its model; every
  received character that is not formatting punctuation appears in some span's or code block's text.
- **M-2 Settled is final (FR-006, SC-002)**: for every char-boundary prefix `p` of a fixture text
  and every longer prefix `q`, `parse(p, true).blocks()[..settled]` equals the same blocks of
  `parse(q, true)`.
- **M-3 Incremental = fresh (FR-005)**: feeding the prefixes in order through `update` gives,
  after each, a document equal to `parse(prefix, true)`.
- **M-4 Finished = streamed (FR-008)**: `update(full, false)` after the deltas equals
  `parse(full, false)`; `settled == blocks().len()`.
- **M-5 Never fails (FR-007)**: no input panics or drops text (a property test over random byte
  cuts of the fixtures and of arbitrary UTF-8).
- **M-6 Code ids are stable**: a `CodeId` names the same code block in every later document of the
  same growing text.
