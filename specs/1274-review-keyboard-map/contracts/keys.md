# Contract: Review keys (#1274)

The user-facing key map and how a press is decided. Rules K1–K4 are core (`review::keys`, M1);
K5–K7 are client (M2). Data shapes: [data-model.md](../data-model.md).

## K1 — The table (`micold_core::review::keys::REVIEW_KEYS`, public, sheet order)

The client binds only from this table and the `?` sheet renders it; #1276's guide test compares the
user guide against it. `primary` = Cmd on macOS, Ctrl elsewhere.

| Key | primary | shift | Labels (Linux/Windows · macOS) | Action | Repeats | Sheet text |
|---|---|---|---|---|---|---|
| `j` | no | — | `j` | `NextFile` | yes | Next file |
| `k` | no | — | `k` | `PrevFile` | yes | Previous file |
| `n` | no | — | `n` | `NextHunk` | yes | Next change |
| `p` | no | — | `p` | `PrevHunk` | yes | Previous change |
| `]` | no | — | `]` | `NextComment` | yes | Next comment |
| `[` | no | — | `[` | `PrevComment` | yes | Previous comment |
| `v` | no | — | `v` | `ToggleReviewed` | no | Mark file reviewed / not reviewed |
| `g` | no | — | `g` | `NextUnreviewed` | no | Next file not reviewed |
| `c` | no | — | `c` | `Comment` | no | Comment on the picked lines or the current change |
| `?` | no | — | `?` | `ToggleSheet` | no | Show or hide these shortcuts |
| `Enter` | yes | yes | `Ctrl+Shift+Enter` · `Cmd+Shift+Enter` | `Send` | no | Send review |
| `Enter` | yes | no | `Ctrl+Enter` · `Cmd+Enter` | — (composer's) | no | Save comment |

## K2 — `resolve(press, holder)`

| Holder | Press | Result |
|---|---|---|
| `Other` | anything | none (FR-002) |
| `Sheet` | text `?`, unmodified, not repeat | `ToggleSheet` (closes) |
| `Sheet` | anything else | none (FR-012) |
| `Composer` | the `Send` entry's press, not repeat | `Send` |
| `Composer` | anything else | none (the text area keeps it) |
| `Review` | a press matching an entry with an action (K3), repeat only if `repeats` | its action |
| `Review` | no text, unbound char, modified, or repeat of a non-repeating key | none |

## K3 — Matching an entry

- `primary: false` entries (single keys): `press.key == Char(c)` and none of Ctrl, Alt, logo held;
  Ctrl **and** Alt without logo (AltGr) counts as none. Shift is not compared.
- `primary: true` entries: `press.key` equals the entry's key, `press.primary`, `press.shift ==
  mods.shift`, and neither Alt nor the non-primary of Ctrl / logo held.

## K4 — Labels

`ReviewKey::labels()` → `(linux_windows, macos)`: a char is itself in both; primary + shift + Enter is
(`Ctrl+Shift+Enter`, `Cmd+Shift+Enter`); primary + Enter is (`Ctrl+Enter`, `Cmd+Enter`). The client
shows the running platform's form (sheet, tooltips); the user guide lists both forms.

## K5 — Building a press (client `keymap.rs`)

`primary_held(mods)`: macOS `logo`, else `ctrl`. `review_press(&KeyInput, repeat) -> Press` fills
`Press` (key from the produced text, `Enter`, else `Other`). No key is matched in the client.
Conflict test (FR-011): the `Send` entry's chord, in both platform forms, equals no #803 FR-006
reserved chord (macOS, Windows, GNOME lists) and no existing app chord.

## K6 — Where presses are heard

- A `keyboard::listen()` subscription in `ui/mod.rs`, held only while the client's holder is
  `Review` or `Sheet`. It builds `Press` from `KeyPressed { text, modifiers, repeat, .. }` and sends
  `resolve`'s action as one Review-tab message. Captured presses (a focused field, a button's
  Enter/Space) never reach it.
- The comment composer's `TextArea` passes each press through `.on_press(..)` to
  `resolve(press, Holder::Composer)`; `Send` becomes `SendPressed`. Its Ctrl/Cmd+Enter save no
  longer matches with Shift held (US4 s4).

## K7 — Action → effect (Review tab reducer)

| Action | Effect |
|---|---|
| `NextFile` / `PrevFile` | `step_file`; select (one `select_file` path: diff from top, row in view, collapsed directory expanded, current hunk cleared) |
| `NextHunk` / `PrevHunk` | `step_hunk`: `Current(i)` → mark and scroll header to top; `File` → select, keep `pending_edge`, resolve on load; `Nothing` → nothing |
| `NextComment` / `PrevComment` | `step_comment`; select if `select`; `Card` → existing pending jump to the card; `Drawer` → open the drawer on it |
| `ToggleReviewed` | the **Mark reviewed** control's message (#1271), selected file only |
| `NextUnreviewed` | `next_unreviewed`; `Select` → select; `AllReviewed` → note |
| `Comment` | `comment_target`; `FocusComposer` → focus it; `Picked` → **Add comment**'s message; `Hunk` → pick that range then **Add comment**; `Refused` → note |
| `ToggleSheet` | open / close the `ShortcutSheet` |
| `Send` | `Msg::SendPressed` (no-op when unavailable, as today) |
