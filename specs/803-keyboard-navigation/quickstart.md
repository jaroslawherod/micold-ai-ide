# Quickstart: Keyboard Navigation

**Feature**: 803 | **Plan**: [plan.md](./plan.md) | Contracts: [keys](./contracts/keys.md),
[ui](./contracts/ui.md) | Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree (in the cloud environment: `scripts/gate.sh`).
- For §B: the `visual-pass` skill's private display, client and daemon built together, a private
  data directory.
- For §C: a macOS bundle from `mise run app` and an installed Windows build.

## §A — Automated

```sh
mise run test-core   # navigation::step wrap rules
mise run gate        # keymap decode/route/near-miss/docs, navigation update and apply, sidebar focus row, TreeItem anatomy, showcase
```

Expected: both green; each test in [plan.md](./plan.md#test-strategy-by-layer) fails without the
change it covers.

## §B — Recorded pass on Linux (visual-pass)

Known projects P (available), Q (available), R (folder renamed away: unavailable). P has Default
(one session), `feat-1_a` (main + two helpers, row collapsed), `fix-2_b` (no session), `chore-3_c`
(one session) and enough rows that `chore-3_c` is below the fold at the sidebar's height; a run
group with one worktree with a session.

| # | Do | Expect |
|---|---|---|
| B1 | Terminal of Default focused; press Ctrl+Shift+. four times, typing `x` after each | `feat-1_a` main, `fix-2_b` (ring on its row, start action shown, terminal still on `feat-1_a` main, no `x` typed into it), `chore-3_c` (scrolled into view, terminal shows it, ring on its row, no `x` typed: the keyboard was on the stop's row, not the terminal, so FR-010 keeps it in the sidebar), the run's worktree (if group expanded), then Default again (wrap) |
| B2 | Ctrl+Shift+, from Default | the last location (wrap backwards) |
| B3 | On `feat-1_a` main, Ctrl+Shift+] three times | helper 1 (row opened, sub-row highlighted), helper 2, main |
| B4 | On a one-session location, Ctrl+Shift+[ | nothing changes, no error |
| B5 | Collapse the run group; step through locations | its worktree is skipped; tag filter `fix` set: only Default, `fix-2_b` (and the current one) are stops |
| B6 | Ctrl+Shift+PageDown twice from P | Q, then P (R skipped); P's sessions kept running (count in the switcher) |
| B7 | Ctrl+Shift+E to release; Ctrl+Shift+. | the sidebar row holds the focus ring; terminal shows the session but is not focused |
| B8 | Open the Overview; Ctrl+Shift+. | Overview closes, next location's session shown |
| B9 | Open Settings / the delete-worktree dialog / a row's context menu / the switcher panel; press each chord | nothing moves |
| B10 | In the terminal run `cat -v`; press all six chords, then Ctrl+[ | nothing printed for the six; `^[` for Ctrl+[ |
| B11 | Switch the X keyboard layout to German (`setxkbmap de`); repeat B1's first press and B3 with the keys at the US `[`/`]`/`,`/`.` positions | same results |
| B12 | Hold Ctrl+Shift+. | steps once per repeat, ends on a consistent row/terminal pair |

Record screenshots for B1, B3, B7 in `specs/803-keyboard-navigation/visual-pass/`.

## §C — macOS and Windows

On each: B1, B3, B6 (macOS: Cmd+Shift; on a laptop Cmd+Shift+Fn+Up/Down for B6), B10. Confirm no
system action fires (macOS Mission Control, Windows emoji/IME panel, GNOME/IBus emoji).
