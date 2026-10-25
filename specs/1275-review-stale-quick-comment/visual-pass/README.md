# Visual pass evidence — #1275

## M3 — hold and notice in the Review tab (T025)

Real client (debug build of the M3 tree, the pinned client and daemon pair) on a cloud Xvfb display
at 1280x800, driven with Xlib XTest. Fixture: a repository with five changed text files, its
**Default** entry's Review tab, `a.txt` open; then `a.txt`, `b.txt` and `c.txt` appended to from a
shell (quickstart §A step 2).

| File | Shows | Verdict |
|---|---|---|
| `m3-a2-review-held-light-1280.png`, `m3-a2-review-held-dark-1280.png` | §A step 2: *3 files changed since you looked* in the header, a dot on the three rows, the open diff and its counts unchanged (`+2 −2`) | PASS |
| `m3-a5-reselect-open-file-dark-1280.png` | §A step 5 on the open file (review A F2): selecting the dotted `a.txt` shows its latest counts (`+3 −2`), its dot goes, the count drops to 2, the scroll stays | PASS |
| `m3-a6-notice-applied-dark-1280.png` | §A step 6: the notice pressed, the list shows the latest counts, no dots, no notice | PASS |

Observations, not failures:

- At 1280 the notice takes header width from the base line, which ends in an ellipsis
  (*Uncommitted change…*) while the notice shows; it comes back whole once applied.
- The dot is #1271's row dot (`UnreadMark`), drawn at the row's end where #1271 put it; it sits
  close to the `−2` count.
