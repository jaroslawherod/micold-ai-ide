# Visual pass M4 (shortcut sheet Composer group)

Route: headless sway + pixman, lavapipe, binary `micold-showcase` built from this worktree and pinned
in `.visual-pass/`. Scrolled to the ReviewBar/Keycap/SideSheet area (just before Snackbar).

| Check | Verdict |
|---|---|
| "shortcut sheet": Review group (13 rows) then Composer group with `Ctrl+Shift+A` / "Add attachment" | PASS |
| "shortcut sheet, composer only": title, Composer group title, single attach chord row, Close | PASS |
| "shortcut sheet, single keys off": Composer group still present (see image) | PASS |
| Group titles, key chips, labels legible; no clipped or overlapping text | PASS |
| `/shortcuts` typed in the real client composer | NOT RUN (needs a daemon and a session; showcase only) |

Images: `visual-pass-m4-showcase-sheets.png` (full + composer-only), `visual-pass-m4-single-keys-off.png`.

Observations (not defects): the long "Comment on the picked lines or the current change" label wraps to two
lines, as designed; chips are small but crisp.
