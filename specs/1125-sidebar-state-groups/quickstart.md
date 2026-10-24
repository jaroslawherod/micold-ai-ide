# Quickstart: Sidebar grouped by state, with Overview as its first row (#1125)

## Part A: automated

| Command | Proves |
|---|---|
| `mise run test-core` | contracts G1–G6 and T1: group of each state, order, stable partition, counts, empty groups dropped, header text, open rule, run group state, second line (FR-001–FR-003, FR-005, FR-011, FR-020) |
| `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test sidebar_tree --test sidebar_state` | S4, S5 (render-free): `sidebar_groups`, `sidebar_list`, `GroupToggled` per project across a re-read and a project switch, tooltip lines, `hover_holds`, row metrics with headers (FR-001–FR-006, FR-012, S7) |
| `scripts/build-lock.sh cargo test -p micold-client --test features_navigation --test navigation --test navigation_apply` | S6: chords in grouped order, a chord or reveal into a closed group opens it (FR-006) |
| `scripts/build-lock.sh cargo test -p micold-client --test overlay_registry --test overlay_dispatch_ordering` | S3: `SidebarMoreMenu` opens, closes on Escape / outside press / displacement (FR-015) |
| `scripts/build-lock.sh cargo test -p micold-client --lib -- overview tree_view composition_contrast keyed_column` | S2 `Opened` rule and badge equality; S5/S8 two-line row, heading, `Selection::Primary` fill, keyed rows keep focus; R12 contrast pairs ≥ 4.5:1 in both themes (FR-007–FR-010, FR-017, FR-019) |
| `scripts/build-lock.sh cargo test -p micold-client --test icons` | `Icon::MoreHorizontal` maps to U+E5D3 |
| `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot --test layout_text_overflow` | the grouped sidebar matches the regenerated fixture; nothing overflows or overlaps at the covered widths (FR-016, FR-022) |
| `scripts/build-lock.sh cargo test -p micold-client --test showcase_completeness` | the Overview row, a header and two-line rows are in the showcase (FR-021) |
| `! grep -nE 'tracing::\|log::\|println!\|eprintln!\|dbg!' crates/micold-core/src/sidebar_groups.rs` | nothing new is logged (FR-023) |
| `mise run gate` | the whole merge gate |

Regenerate the fixture only once the sidebar is done:
`UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`,
then read the fixture diff: only sidebar paths and the new `main-shell-sidebar-grouped` state may
change. After a rebase onto #1124 or #1126, regenerate again; never hand-merge the fixture.

## Part B: visual pass (the `visual-pass` skill)

Prerequisite: the showcase on a private display, then the client on a project with worktrees in
several states (one with a known issue title, one merged, one with a permission prompt, one with
failing checks), light then dark. Compare with the mockup board's sidebar
(`Main.dc.html#sidebar`, Layout Redesign, "Proposal: session view").

| # | Do | Expect |
|---|---|---|
| B1 | Look at the sidebar top | Overview row first with its icon and "2 need action"-style badge in the error (or warning) container colours; the chip bar under it: **All**, the type chips, then add, hide, ⋯ (US2.1, US2.2, US4.1) |
| B2 | Read the groups | headers "Needs action · n", "Working · n", "Idle · n", "Done · n" in that order; Done collapsed; the failing-checks worktree under Needs action (US1.1–US1.3) |
| B3 | Read the rows | first line the issue title (or branch); second line `fix/1117 · 1 session · 1 running`; *Default* reads "Default" / "project root · …"; no type or issue chip (US3.1–US3.4, US3.7) |
| B4 | Select a session; open the Overview from the row | selected row in the primary container colour, legible; while the Overview is open only the Overview row looks selected (US2.4, US3.6) |
| B5 | Press **All** and type chips; open ⋯ and choose each item; press Escape on the open menu | filters as before; each item does what the old header button did; the menu closes (US4.2–US4.6) |
| B6 | Tab through the sidebar; Enter on a header; a location chord into collapsed Done | ring visits every control in order; the header toggles; Done opens and the row is selected (US5.1–US5.4) |
| B7 | Make a session start working while its row has the keyboard | the row moves to Working; the ring stays on it; counts update (US1.6, US5.5) |
| B8 | Drag the sidebar to its narrowest (180 px) and widest | both lines and headers ellipsize; counts stay; chips wrap; nothing overlaps (Edge Cases) |
| B9 | Repeat B1–B4 in dark | the same, every text legible (FR-019) |
| B10 | Compare B1–B4 screenshots with the mockup | grouping, order, two-line rows and the Overview row match; differences recorded with a reason (sentence-case headers, Done in the list rather than pinned at the foot: research R8) |

Record screenshots under `specs/1125-sidebar-state-groups/visual-pass/` and the results in
`visual-pass.md`.
