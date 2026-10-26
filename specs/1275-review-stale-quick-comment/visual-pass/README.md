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

### M4 review round 1 — a fast click (review A F1)

Same setup, M4 tree with the round 1 fixes. XTest sent the button's press and release back to
back with no wait, so they reach the window in one batch of events, before the press's message is
handled.

| File | Shows | Verdict |
|---|---|---|
| `m4-c-1-fast-click-{light,dark}-1280.png` | the fast click on new line 2: line 2 picked, **Add comment** under it, no box | PASS |
| `m4-c-3-moved-and-clicked-{light,dark}-1280.png` | then the pointer moved with no button over the numbers of lines 3–8 and a plain click on the diff text: the screen is pixel-identical to the one above (pick still line 2, no box opened, nothing focused) — before the fix the drag stayed on, the pick followed the pointer and the next release opened a box | PASS |

A full drag afterwards (2 → 5) still opens the box under the released line, as in §B.

## M5 — suggestion chips (T043)

Real client (debug build of the M5 tree, client and daemon pair) on a cloud Xvfb display at
1280x800, driven with Xlib XTest. Fixture: the M4 repository, its **Default** entry's Review tab,
`a.txt` open, a new comment's box opened by a drag over new lines 2–7 (quickstart §B step 5). The
data directory starts with no `suggestion_chips` key (a fresh install's value).

| File | Shows | Verdict |
|---|---|---|
| `m5-b5-1-chips-off-{light,dark}-1280.png` | switch off: the box is as before M5, text area then Cancel/Save, no chips (SC-004) | PASS |
| `m5-b5-2-settings-switch-on-{light,dark}-1280.png` | Settings → Appearance: **Suggestion chips** under **Reduce motion** with its note, ticked; after **Save** `settings.json` holds `"suggestion_chips": true` | PASS |
| `m5-b5-3-tab-order-light-crop.png` (four frames stacked), `m5-b5-3-tab-add-test-dark-1280.png` | the box already open shows the chips once saved (FR-026); **Tab** from the text area reaches *Rename*, *Add test*, *Why?*, then **Cancel**, each with the focus ring (FR-027) | PASS |
| `m5-b5-4-add-test-filled-light-1280.png`, `m5-b5-4-add-test-by-key-dark-1280.png` | *Add test* pressed in the empty box (light: clicked; dark: **Space** on the focused chip): the box reads *Add a test that covers this.* with the caret at the end and **Save** available; typing ` ok` then appended at the end | PASS |
| `m5-b5-5-notice-dot-chips-{light,dark}-1280.png` | `b.txt` and `c.txt` appended to from a shell with the box open: *2 files changed since you looked*, the two dots and the chips together (SC-006); in light, **Space** on the focused *Why?* had added *Why is this needed?* on a new line after the text, the caret back in the box | PASS |

## M6 — reviewed files while held (T046)

Real client (debug build of the M6 tree, client and daemon pair) on a cloud Xvfb display at
1280x800, driven with Xlib XTest. Fixture: the M5 repository, its **Default** entry's Review tab
(quickstart §B step 6).

| File | Shows | Verdict |
|---|---|---|
| `m6-b6-1-marked-{light,dark}-1280.png` | `a.txt` and `b.txt` marked with **Mark reviewed**: both rows ✓, header *2 of 5 reviewed*, `b.txt` open | PASS |
| `m6-b6-2-held-reviewed-{light,dark}-1280.png` | a line appended to `a.txt` from a shell: *1 file changed since you looked*, `a.txt`'s row back to ○ with the dot, `b.txt` still ✓, *1 of 5 reviewed*; the open `b.txt` diff unchanged | PASS |
| `m6-b6-3-dot-tooltip-{light,dark}-1280.png` | hovering `a.txt`'s dot: *Changed since you looked, and since you marked it reviewed* | PASS |

Observation, not a failure: the tree stays in the order it had (Path sort). Under **Unreviewed
first** the order follows #1271's judgement on the live whole-change ids, not the hold, so it can
move while changes are held (unchanged by M6).
