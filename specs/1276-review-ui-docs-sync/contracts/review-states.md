# Contract: review states in the layout fixture (FR-006, FR-007, FR-008)

Registered in `crates/micold-client/tests/support/covered_states.rs`. A slice that already
registered a state under another name keeps its name; this list is the coverage that must exist
after M2, not a rename request. Anchor paths are found from the built tree; names below are the
contract.

| State | Shows | Anchors (at least) |
|---|---|---|
| `session-area-changes-open` (redefined) | Review tab selected in the strip, header above, review body | `session.header`, `session.header.changes`, `session.strip`, `review.tab` |
| `review-tab-beside-sessions` | strip with two session tabs and the Review tab with badge *Review · 3* | `review.tab`, `review.tab.badge` |
| `review-tab-hidden-changed` | a session tab shown, Review tab hidden with *changed since you looked* | `review.tab`, `review.tab.changed` |
| `review-bar-counts` | review bar with drafts, sent and replies counts, **Send review (n)** enabled | `review.bar`, `review.bar.send`, `review.bar.overflow`, `status_bar` |
| `review-drawer-drafts` | drawer open on **Drafts** | `review.drawer`, `review.drawer.tabs`, `review.diff` |
| `review-drawer-replies` | drawer open on **Replies** | `review.drawer`, `review.diff` |
| `review-source-open` | source control open (press; `over` records) | `review.source`, `review.source.list` |
| `review-tree-filtered-sorted` | file tree with filter text and a non-default sort, collapsed and open directories | `review.tree`, `review.tree.filter`, `review.tree.sort` |
| `review-reviewed-progress` | reviewed / drafts / unseen marks, *n of m reviewed* with bar | `review.progress`, `review.tree.row.mark` |
| `review-shortcut-sheet` | `?` sheet open | `review.sheet` |
| `review-stale-notice` | *n files changed since you looked* in the header | `review.stale` |
| `review-drag-composer` | composer opened by a line-number drag | `review.composer` |
| `review-empty` | no changes: empty tree, zero counts, **Send review** disabled; drawer with no drafts and no replies | `review.tree`, `review.bar`, `review.bar.send` |
| `review-two-worktrees` | worktree B's Review tab shown while A's holds its own selection | `review.tab`, `review.tree` |

## Gates

- Existing, at 1280×800, over all states: fixture match, coverage narrowing (`layout_snapshot.rs`
  T016), text overflow, containment, `status_bar_fits`, `session_area_fits`.
- New `gates/review_tab_fits.rs`, every `review-*` state and `session-area-changes-open`, both
  themes, at `MIN_WINDOW_SIZE` and 1280×800:
  - no text overflow (`support::layout::text_overflows`);
  - no escape outside the declared exemptions (`support::layout::escapes`);
  - `review.bar` bottom ≤ `status_bar` top, and both inside the window;
  - when `review.drawer` is present, `review.diff` has the same width as without it and the drawer
    is inside the diff's rectangle (overlay, not push).
- Deliberate-failure check (US2 Independent Test): +1 px on the review bar's padding fails
  `layout_snapshot` naming `review.bar`.

## Doc

`docs/development/layout-snapshot.md` § *What is covered* gets one paragraph naming each state above
and what it anchors, in the feature-paragraph form used there (feature 1124, 1125, 1126), and the
`session-area-changes-open` sentences are replaced (US2 sc.3).
