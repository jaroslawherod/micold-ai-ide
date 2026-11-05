# Data model: Highlighted user messages

No stored or protocol data changes. `EntryKind::UserMessage { text, attachments, command }` is read, not modified.

## UserTurn (view-only, `micold-material`)
- Inputs: `content: Element`, `roles: Roles`.
- Derived: fill = `roles.surface_container_high`; edge = `roles.primary`, width 4dp; corner = `shape::SMALL`; padding = `spacing::SM` inside the edge.
- No state, no validation; empty content still draws the edge and padding.
