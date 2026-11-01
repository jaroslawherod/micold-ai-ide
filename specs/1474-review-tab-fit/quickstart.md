# Quickstart: Review tab fits the smallest window

## A — Automated

```bash
mise run test-core                                   # drawer::width
cargo test -p micold-client --test sidebar_state --test features_window --test overview
cargo test -p micold-client --test layout_snapshot every_review_state_fits   # via scripts/build-lock.sh
mise run gate
```

Expected: the reducer table of [contracts/sidebar-effective-state.md](contracts/sidebar-effective-state.md)
passes; the review gate has no `#1474` entries; `layout_snapshot` fixture unchanged.

## B — Visual pass (visual-pass skill)

Prerequisite: a project with a worktree that has changed files and at least one draft comment.

1. Window 1000px wide, sidebar expanded. Drag the window to 640px: the sidebar slides to its strip
   and the main area widens.
2. Open the Changes tab at 640x480: the diff header's chips and buttons sit inside the header row;
   with a diff loading the placeholder sits inside its box; open the comments drawer: header,
   tabs and each row's buttons are inside it and clickable; the tree filter and rows are inside the
   tree pane. Light and dark.
3. Press **show** on the strip: the sidebar expands over the narrow main area; resize to 660px: it
   stays; widen to 720px: it stays as `hidden` says (expanded); shrink to 650px: it collapses again.
4. Hide the sidebar at 1000px; resize to 640px and back: hidden at both.
5. Overview full screen at 640px; close: the strip. At 1000px with the sidebar expanded: expanded.
6. At 1280x800 the review tab looks as before.
