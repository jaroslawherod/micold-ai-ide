# Contract: Generic Clipboard-Copy Message

The reusable primitive this feature introduces, intended for reuse by future "copy this
read-only label" needs (session titles, project names — see spec.md Assumptions), not only the
worktree context menu it currently serves.

## Message (`micold_ai_ide::app::Message`)

```rust
/// Copy arbitrary displayed text (e.g. a worktree name) to the system clipboard. The binary
/// performs the actual clipboard write; the reducer has no state to update.
TextCopyRequested(String)
```

**Contract guarantees**:
- C1 The pure core reducer (`State::update`) treats `TextCopyRequested` as a no-op — it belongs
  to the same "binary handles clipboard I/O" group as `TerminalCopyRequested` /
  `TerminalPasteRequested`. Asserting this is possible in a headless test (construct a `State`,
  call `update(TextCopyRequested(..))`, assert no visible field changed) but is not currently
  asserted, matching the untested precedent of its terminal siblings (see plan.md Complexity
  Tracking).
- C2 The binary's handler (`src/main.rs`) writes the payload verbatim to the system clipboard
  via `iced::clipboard::write` — no transformation, truncation, or trimming of the text.
- C3 **Known coupling (not a bug, but binding on new call sites)**: the binary's handler also
  dispatches `Message::WorktreeMenuDismissed` unconditionally, because today the message is only
  ever produced by the worktree context menu's "Copy name" item. This is harmless when no
  worktree menu is open (`WorktreeMenuDismissed` is idempotent — it just sets an `Option` to
  `None`), but a **future caller from an unrelated context is not required to route through a
  worktree menu**, and should confirm this side effect stays harmless for its case, or the
  handler should be revisited to only dismiss the menu when the request actually originated from
  it (e.g. by threading an `Option<MenuOrigin>` or splitting the dismiss into the call site) if a
  problematic case arises.
- C4 Clipboard-write failure is best-effort: there is no reducer-visible error path and no
  in-app error message is surfaced, matching `TerminalCopyRequested`/`TerminalPasteRequested`.

## Call site (`src/ui/mod.rs::worktree_menu_items`)

```rust
fn worktree_menu_items(dir: &str, display_name: &str) -> Vec<material::MenuItem<Message>>
```

**Contract**: the second parameter is the exact text the row displays (already resolved by the
caller via `State::worktree_display_name`), not the raw `dir_name`. The **Copy name** entry emits
`Message::TextCopyRequested(display_name.to_string())`. Any future call site reusing
`TextCopyRequested` for a different label follows the same rule: resolve the exact displayed
text before constructing the message, never the underlying identifier.
