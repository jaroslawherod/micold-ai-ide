# Quickstart: Overview cards, full text without truncation

## A — Automated

```sh
mise run test-core        # overview_expansion (K1–K4), ExpandedText, worktree_dir (K5–K6)
scripts/gate.sh           # whole gate: fmt, clippy, workspace tests, layout snapshot
grep -nE 'tracing|log::|println' crates/micold-core/src/overview_expansion.rs \
  crates/micold-core/src/overview_text.rs   # FR-018: no output
```

Expected: all green; the grep prints nothing; `tests/layout_snapshot.rs` passes with the
`overview-expanded` state and its text-overflow and containment gates.

## B — Visual pass (component showcase and client, both themes, scale 1.0 and 2.0)

1. Showcase *Surfaces*: collapsed cards with 1-, 2-, 3- and 5-line titles at 280/380/480 px —
   3 lines then an ellipsis; activity 2 lines; nothing clipped (FR-001, FR-007).
2. Showcase: the expanded card over its neighbours, elevated, contrast as feature 1120 (FR-015).
3. Client, Overview, full grid: hover a card ~0.4 s → expands down over the card below; no other
   card moves; leave → collapses (Story 2).
4. Hover a bottom-row card → shifted up, whole; a very long activity → card as tall as the view,
   header and title stay, wheel scrolls inside, then the Overview (FR-008).
5. Keyboard only: Tab/arrows expand each focused card; Escape collapses, Overview stays; Escape
   again closes it; Page Down scrolls a long expanded card (Story 3, D1).
6. Expanded card: click over the overlapped neighbour activates the expanded card with its ripple;
   right-click and Menu key open its menu, target mark over the whole card; other cards do not
   expand while it is open (Story 4).
7. Full-screen mode: same behaviour; zoom unaffected.
8. The directory line reads `~/…` for a worktree under the home folder (D2).

## Results (close, 2026-10-21)

- **A** (T047): the FR-018 grep prints nothing. `cargo test -p micold-core --all-targets` passes
  every overview test (`overview_expansion` 29, `overview_text`, `worktree_dir`). Its only
  failures are permission-refusal tests that do not belong to this feature:
  `settings_refuses_save_over_failed_read`, `settings_write_is_logged` and two directory-removal
  tests. They fail only when run as root, and `scripts/gate.sh` runs them unprivileged.
  `scripts/gate.sh` on the close tree (14faf6dd, after the close review fix): green.
- **B** (T048): every item passes. The record, its evidence and its limits are in
  [visual-pass.md](visual-pass.md).
