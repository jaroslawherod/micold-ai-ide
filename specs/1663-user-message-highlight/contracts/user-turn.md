# Contract: `UserTurn` (micold-material)

```rust
UserTurn::new(content: impl Into<Element<'a, M>>, roles: Roles) -> UserTurn<'a, M>
impl<'a, M: 'a> From<UserTurn<'a, M>> for Element<'a, M>
```

- Width fills the parent; height shrinks to content.
- Draws a `surface_container_high` surface and a 4dp `primary` left edge; no shadow, no outline.
- Adds no text, no focusable widget and no message: it forwards content events untouched.
- Colours come only from `roles`; the same call site follows scheme and accent changes.
- Styles: `style::user_turn_surface(Roles)`, `style::user_turn_edge(Roles)`.

Call-site contract (`ui/transcript/entries.rs`): both `EntryKind::UserMessage` branches return `UserTurn::new(column, roles).into()`; no other `EntryKind` does.
