# Contract: block renderers (`micold-client/src/ui/transcript/blocks.rs`, FR-022)

The seam #940 swaps. Everything outside the four renderers — which entries exist, their order,
folding, streaming, follow-tail — is decided by the transcript model and the view state and never
by a renderer.

## Shape

```rust
/// What a renderer returns: the view's own message type, `TranscriptMsg`.
pub type Block = Element<'static, TranscriptMsg>;

pub struct BlockRenderers {
    pub assistant_text: fn(&AssistantText<'_>, &RenderCtx<'_>) -> Block,
    pub thinking:       fn(&ThinkingText<'_>,  &RenderCtx<'_>) -> Block,
    pub tool_input:     fn(&ToolInput<'_>,     &RenderCtx<'_>) -> Block,
    pub tool_result:    fn(&ToolResult<'_>,    &RenderCtx<'_>) -> Block,
}

impl BlockRenderers {
    /// #939: plain, wrapped text for every kind.
    pub const PLAIN: Self = /* … */;
}

pub struct BlockRef<'a> { pub session: SessionId, pub key: &'a EntryKey, pub revision: u64, pub streaming: bool }
pub struct AssistantText<'a> { pub block: BlockRef<'a>, pub text: &'a str }
pub struct ThinkingText<'a>  { pub block: BlockRef<'a>, pub text: &'a str }
pub struct ToolInput<'a>     { pub block: BlockRef<'a>, pub tool: &'a str, pub input_json: &'a str }
pub struct ToolResult<'a>    { pub block: BlockRef<'a>, pub tool: Option<&'a str>, pub parts: &'a [ResultPart], pub is_error: bool }

pub struct RenderCtx<'a> { pub roles: Roles, pub scheme: ColorScheme, pub cache: &'a BlockCache }

/// Keyed by `(SessionId, EntryKey, revision)`; its value type `CachedBlock` has no variant in #939.
pub struct BlockCache { /* HashMap<BlockKey, CachedBlock> */ }
impl BlockCache { pub fn get(&self, block: &BlockRef<'_>) -> Option<&CachedBlock>; /* len, is_empty */ }
```

`Element<'static, _>` because entries are drawn inside iced `lazy` (research R12): a renderer clones
what it shows, and runs only when the entry's `revision`, its fold state or the scheme changes. The
view's message type is `TranscriptMsg`; the pane maps it into the window's message. A folded card's
body is not shown, so its renderers do not run.

## Rules

- **R-1** The transcript view calls exactly one renderer per block shown, chosen by kind, from the
  `BlockRenderers` value the window holds (`BlockRenderers::PLAIN` in #939). Replacing one field
  changes only that kind's drawing.
- **R-2** A renderer draws block content only: no fold control, no status, no header, no scroll
  behaviour, and no message other than ones #940 adds for its own content.
- **R-3** A renderer must handle `streaming: true` (partial, possibly unbalanced, text) without
  failing; #940's markdown renderer may draw a streaming block as plain text until it finishes.
- **R-4** `BlockCache` is per window, keyed by `(SessionId, EntryKey, revision)`; #939 leaves it
  empty. #940 fills it in update after each applied `StructuredEvents` message (derived state such
  as parsed markdown or highlighted lines) and prunes it with the transcript.
- **R-5** The plain renderers draw the shared `Text` primitive at `TypeRole::Body`, wrapped to the
  entry's width; tool input and results also take a new `Text::monospace()` builder step
  (`Font::MONOSPACE`, as `material::DiffView` draws code); `Unshown` parts as
  a muted caption naming the part (`[image not shown]`); `is_error` adds nothing inside the block
  (failure is drawn by the card, FR-012).
