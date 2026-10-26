# Quickstart: Review UI — Stale-Awareness and Quick Comment

How to prove the feature works. Rules: [contracts/](./contracts/); types:
[data-model.md](./data-model.md). Reuses 482's fixture and setup
(`specs/482-worktree-changes-review/quickstart.md`).

## Prerequisites

- A worktree entry with at least five changed text files (482's fixture), opened in the Changes
  view (the Review tab once #1269 has merged).
- For §B: the `visual-pass` skill's private display and data directory.

## §0 — Automated (every milestone)

```sh
mise run test-core                                   # M1, M2: core-api.md I1–I8, S1–S10, D1–D8, K1–K5
mise exec -- cargo test -p micold-client --test features_changes   # M3–M5: H1–H15, Q1–Q12
scripts/gate.sh                                      # before every push: fmt, clippy, all tests, scripts
```

Expected: all pass. After a UI milestone, `UPDATE_LAYOUT_SNAPSHOT=1 mise exec -- cargo test -p
micold-client --test layout_snapshot` records the notice, dot and chips; the diff of
`tests/fixtures/layout_snapshot.txt` shows only those additions.

## §A — Holding while files change (M3; SC-001, SC-002)

1. Select `a.txt` and scroll its diff so line 40 is the first visible line.
2. From a terminal, append a line to `a.txt`, `b.txt` and `c.txt`. Within 2 s: the header reads *3
   files changed since you looked*, the three rows carry a dot, the diff and its first visible line
   are unchanged.
3. Edit `a.txt` again: the count stays 3. Undo the edit to `b.txt` (restore its content exactly):
   the count drops to 2 and `b.txt` loses its dot.
4. In `c.txt`, change a word on an already-added line (counts stay the same) after applying once:
   it counts again (AS12).
5. Select `c.txt` (in the set): its latest diff shows, its dot goes, the count drops by one, `a.txt`
   still carries its dot.
6. Pick two lines in `a.txt`, open the composer, type text, then press the notice: the diff shows
   the edits, the dots and the notice go, the composer keeps its text, the pick stays where its
   lines still exist.
7. Press **Uncommitted** off and on: the list re-reads at once, nothing is held.
8. Script SC-001: `for i in $(seq 1 10); do echo x >> f$i.txt; sleep 0.5; done` while a file is
   open: the first visible line does not move once until the notice is pressed.

## §B — Recorded pass on Linux (visual-pass), 1280×800, light and dark

1. §A step 2's state: header notice and row dots.
2. A drag from one line number to another five lines below (mouse): the pick follows the pointer;
   on release the composer is open under the last line with the caret in it.
3. The same drag with a touchpad press-and-drag.
4. Press **Esc** mid-drag: the earlier pick and composer come back.
5. Settings → Appearance → **Suggestion chips** off (fresh data directory): a composer shows no
   chips. Turn it on and Save: a composer shows *Rename*, *Add test*, *Why?*; **Tab** from the text
   area reaches each chip with its focus ring; pressing *Add test* in an empty box fills it with the
   caret at the end.
6. (M6, FR-013) Mark `a.txt` and `b.txt` reviewed (header *2 of m reviewed*), then append a line to
   `a.txt` from a terminal: `a.txt`'s row shows ○ with the dot (tooltip *Changed since you looked,
   and since you marked it reviewed*), `b.txt` keeps ✓, the header reads *1 of m reviewed* beside
   *1 file changed since you looked*. Select `a.txt`, append another line: **Mark reviewed** is
   unavailable (tooltip *This file changed since you looked: select it again to see its latest
   change*); select `a.txt` again and it is available.

Screenshots of steps 1, 2, 5 and 6 in both themes go in `visual-pass/`.

## §C — macOS and Windows

On each: §A steps 2, 5 and 6, and §B steps 2 and 5 (save shortcut **Cmd+Enter** on macOS).

## Not run here

Record in `visual-pass/` any step a container without a display could not run, and what stood in
for it (482's and 946's practice).

## Results (close unit, T047, 2026-10-26)

Run on the M6 tree (`02f5abe50`; debug build of the client and daemon pair) in a Linux cloud
container on an Xvfb display at 1280x800, driven with Xlib XTest. Fresh fixture: a repository
whose `a.txt` has 120 lines with every fifth line edited (one long hunk), `b.txt`–`e.txt` one
edited line each, and `f1.txt`–`f10.txt` committed unchanged; a fresh data directory; the
**Default** entry's Review tab. Screenshots: `visual-pass/close-*.png`.

### §0 — Automated

`micold-core` `review::` 337 passed (1 ignored); `micold-client --test features_changes` 224
passed. The full `scripts/gate.sh` result is in the ledger.

### §A — Holding while files change

| Step | Result | Evidence |
|---|---|---|
| 1 | PASS: new line 40 the first visible line after scrolling | `close-a1-scrolled-dark-1280.png` |
| 2 | PASS: 2 s after appending to `a.txt`, `b.txt`, `c.txt`: *3 files changed since you looked*, three dots, diff and first visible line unchanged (`+24 −24` still shown) | `close-a2-held-{dark,light}-1280.png` |
| 3 | PASS: a second edit to `a.txt` keeps 3; `b.txt` restored byte for byte drops it to 2 and its dot goes | `close-a3-restored-dark-1280.png` |
| 4 | PASS: after applying, rewording `c.txt`'s added line 10 (counts stay `+2 −1`) counts again: *1 file changed since you looked* (AS12) | `close-a4-counts-again-dark-1280.png` |
| 5 | PASS: with `a.txt` and `c.txt` in the set, selecting `c.txt` shows its latest diff (*reworded*), its dot goes, the count drops to 1, `a.txt` keeps its dot | `close-a5-select-in-set-dark-1280.png` |
| 6 | PASS: lines 6–7 of `a.txt` picked by a drag, *check this* typed, then `a.txt` and `b.txt` changed: notice and dots; pressing the notice shows the edits (`+2,123`, `+28 −24`, `b.txt` `+2 −1`), dots and notice gone, the box keeps *check this*, lines 6–7 stay picked | `close-a6-composer-held-dark-1280.png`, `close-a6-applied-dark-1280.png` |
| 7 | NOT RUN on screen: opening the source menu (to switch **Uncommitted** off and on) panics the client, on this tree, on `origin/main` (`63cf1dc4e`) and on main before any #1275 client code (`58ef1146d`), so not this feature's (see below). Stood in: H3 `a_read_the_user_causes_applies_even_queued_behind_a_watch_read` (`Msg::UncommittedToggled` while held: the read applies, nothing stays held) | test |
| 8 | PASS: the SC-001 loop (`f1`–`f10` appended 0.5 s apart) while `a.txt` was open at line 40: 276 frames of the diff pane grabbed every 25 ms over the loop and 3 s after it, 0 differed from the first; header *10 files changed since you looked*, the new files not listed until the notice was pressed (then 15 rows, `0 of 15 reviewed`, line 40 still first) | `close-a8-sc001-held-dark-1280.png`, `close-a8-applied-dark-1280.png` |

Found, not this feature's: clicking the **Working tree** source field panics at
`iced_widget-0.14.2/src/scrollable.rs:1371` (`layout.children().next().unwrap()` on `None`, in
`Scrollable::overlay` under `cdk::picker::Menu` → `Scale` → `Fade` → `Container` → `Container`,
reached through `overlay::Group`), with nothing held, on all three builds above. Reported to the
orchestrator for its own issue.

Observation, not a failure: with more rows than fit (after step 8) the list's scrollbar sits over
the right half of the row dots (`close-a2-held-light-1280.png`).

### §B — Recorded pass on Linux

Run per milestone and recorded in `visual-pass/README.md`: steps 1 (M3), 2 and 4 (M4), 5 (M5), 6
(M6), all PASS in both themes. Step 3 (touchpad) not run: Xvfb has no touchpad; a touchpad
press-and-drag reaches iced as the same left-button press, motion and release that step 2 drove.

### §C — macOS and Windows

Not run: this run had only a Linux cloud container, with no macOS or Windows machine or runner.
What stood in, per step:

- §A steps 2, 5, 6 and §B steps 2, 5: the Linux runs above and in `visual-pass/`. None of the
  feature's code is platform-specific: the hold, the dot and the notice are iced widgets and
  core logic; the drag uses iced's mouse events, which winit maps the same on all three; the file
  watch is the daemon's existing watcher (#482), unchanged here.
- The save shortcut (**Cmd+Enter** on macOS, Ctrl+Enter elsewhere) is the existing
  `TextArea::on_submit` binding of `micold-material`, not touched by this feature.
- CI runs the workspace tests on macOS and Windows for every PR: M1–M5 merged green there; M6
  (#1450) was in CI when this was written.

Left for a person with those machines: §C as written, in particular a trackpad drag on macOS
and the notice under Windows' scaling.
