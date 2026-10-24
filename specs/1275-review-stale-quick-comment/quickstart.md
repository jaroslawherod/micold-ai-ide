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

Screenshots of steps 1, 2 and 5 in both themes go in `visual-pass/`.

## §C — macOS and Windows

On each: §A steps 2, 5 and 6, and §B steps 2 and 5 (save shortcut **Cmd+Enter** on macOS).

## Not run here

Record in `visual-pass/` any step a container without a display could not run, and what stood in
for it (482's and 946's practice).
