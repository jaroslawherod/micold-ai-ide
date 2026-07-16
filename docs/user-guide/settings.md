# Settings

Open **Settings** from the overflow menu (the three-dots button) in the top toolbar.

## Terminal scrollback limit

Controls how many lines of earlier output each session's terminal keeps for scrolling back
through (see [Worktrees & sessions → Sizing, resize & scrollback](./worktrees-and-sessions.md)).

- **Default**: 10,000 lines.
- **Range**: 100 – 1,000,000 lines. Values outside the range (or non-numeric input) are
  rejected with a message and not saved.
- The value is **saved on your machine** and restored the next time you open the app.
- A changed limit applies to sessions started **after** the change; already-running terminals
  keep their current buffer.

Click **Save** to apply, or **Cancel** (or press Esc) to dismiss without changing anything.
