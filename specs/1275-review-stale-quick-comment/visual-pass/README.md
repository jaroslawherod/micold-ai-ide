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

## M4 — drag over line numbers opens the composer (T034)

Real client (debug build of the M4 tree, client and daemon pair) on a cloud Xvfb display at
1280x800, driven with Xlib XTest (button press, motion in 4 px steps, release; keys after giving the
X window input focus). Fixture: the M3 repository with eight lines added to `a.txt`; its
**Default** entry's Review tab, `a.txt` open in the unified layout (quickstart §B steps 2–4).

| File | Shows | Verdict |
|---|---|---|
| `m4-b-1-mid-drag-{light,dark}-1280.png` | §B step 2, button still held: pressed on new line 2, dragged to 7; numbers 2–7 of the new side filled, the removed line 5 (old side only) skipped; no box or **Add comment** moved in | PASS |
| `m4-b-3-typed-{light,dark}-1280.png` | §B step 3: released on 7 — the box opened under line 7 with the caret in it; `looks` typed straight away with no click | PASS |
| `m4-b-4-second-drag-{light,dark}-1280.png` | §B step 4 mid-drag with the box open: 8 → 10 across a hidden-lines row and a hunk header (no numbers, pick unchanged by them); the box with its text stays under 7, rows do not move | PASS |
| `m4-b-6-after-esc-release-{light,dark}-1280.png` | §B step 4: **Esc** mid-drag, then release — pick back to 2–7, the box and `looks` as before the press | PASS |

Found and fixed during the pass: in the first run the **Add comment** slot (and an open box) followed
the pick's last line while dragging, so the rows under the pointer moved down by the slot's height
and the drag stopped short (pressed 10, dragged to 15, picked 10–13). Since then the slots stay
under the pick from before the press until the release (`changes::slot_pick`, test
`drag::the_slots_stay_put_while_the_pick_follows_the_pointer`).

Not run in the container: a touchpad press-and-drag (no touchpad device under Xvfb; it reaches iced
as the same mouse button and motion events) and a release outside the window (XTest cannot move the
pointer past the 1280x800 screen; the release is caught window-wide by the same listener, Q9 test).
