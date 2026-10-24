# Contract: visual-pass checks (FR-009, FR-010)

1280×800, light and dark, real client + daemon on Xvfb + lavapipe, world from
[../data-model.md](../data-model.md) § Visual-pass world. Two crops per check; verdict in
`visual-pass/results.md`.

| NN | Check | PASS when |
|---|---|---|
| 01 | Review tab and badge in the strip | tab beside session tabs, badge legible, no collision with neighbours |
| 02 | Header: source control and *n of m reviewed* | control and progress fit beside the title; long PR title truncates, no overlap |
| 03 | File tree | reviewed / drafts / unseen marks distinct in both themes; filter box; `+a −r` bars; deep path and long name do not overlap |
| 04 | Diff with cards | bundled monospace face; inline draft card; sent card with its reply |
| 05 | Review bar idle | above the status bar, counts and **Send review (n)** |
| 06 | Review bar *Sending…* | state shown, bar keeps its height |
| 07 | Review bar error | error text readable, not clipped |
| 08 | Drawer on **Drafts** | over the diff, diff not narrowed |
| 09 | Drawer on **Replies** | as 08 |
| 10 | Shortcut sheet | every key legible, nothing clipped |
| 11 | *changed since you looked* | mark on the hidden tab and notice in the header |
| 12 | Drag-to-comment composer | composer at the dragged lines |
| 13 | Empty review | empty tree, zero counts, **Send review** disabled |
| 14 | Empty drawer | no drafts / no replies tabs read as empty, not broken |
| 15 | PR diff cannot be read | error shown in the review body |
| 16 | Two worktrees | switching A ↔ B shows each tab's own selection |
| 17 | Overview board | as-built cards: stripe on neutral, tiers, wrapped titles |

On FAIL: a styling or doc fix in this feature (shared component, not a local override), or an issue
linked from the row (FR-010). Checks not run (macOS, Windows) are listed with the reason.
