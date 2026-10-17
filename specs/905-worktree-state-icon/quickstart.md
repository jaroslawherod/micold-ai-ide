# Quickstart: Worktree State Icon

**Feature**: 905 | **Plan**: [plan.md](./plan.md) | Contracts: [state-icon](./contracts/state-icon.md),
[wire](./contracts/wire.md) | Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree (in the cloud environment: `scripts/gate.sh`).
- For §B: the `visual-pass` skill's private display, pinned client and daemon built together, and
  a private data directory. Claude Code hooks configured for the sessions (activity readable).
- For §C: a macOS bundle from `mise run app` and an installed Windows build.

## §A — Automated

```sh
mise run test-core   # session_state over every lifecycle × activity × flag; location_state; TurnClock::paused_on_user
mise run gate        # wire round-trip, daemon note_activity, client state, tree_view geometry, contrast, icons, showcase, docs
```

Expected: both green. Each test in [plan.md](./plan.md#test-strategy-by-layer) exists and fails
without the change it covers.

## §B — Recorded pass on Linux (visual-pass)

Project P with Default and worktrees `wt-run`, `wt-block`, `wt-idle`, `wt-none` (no session),
`wt-two` (main idle, helper asking for a permission). Screenshot each step that names a look, in
the light and the dark scheme.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Give `wt-run`'s session a long prompt; leave the rest. Nothing selected. | `wt-run` triangle; `wt-idle`, `wt-none` two bars; `wt-two` octagon; Default per its session. All in one column. | US1.1–1.3, US2.1, FR-001, FR-002, FR-004 |
| B2 | Let `wt-run` finish its turn. | Its icon turns to two bars within 1 s; name does not move. | US1.4, FR-006, FR-010 |
| B3 | In `wt-block`, prompt a tool that needs permission. Then grant it. | Octagon within 1 s; then triangle within 1 s. Tooltip lines `Blocked: needs permission`, then `Running`. | US2.1, US2.4, FR-009 |
| B4 | Make a session fail to start (provider binary off `PATH`). | Its row shows the octagon; tooltip `Blocked: failed`. | US2.2 |
| B5 | Restart the service while a session is mid-turn. | That row shows the octagon; tooltip `Blocked: resumable`. While disconnected, rows kept their icons. | US2.3, Edge: restart |
| B6 | A Codex or OpenCode session with no activity signal. | Empty slot (name still aligned); tooltip `State unknown`. | FR-005 |
| B7 | Expand and collapse `wt-two`; hover a row. | Icon unchanged either way; session sub-rows keep their activity dots. | FR-001, Edge: several sessions |
| B8 | Narrow the sidebar to its minimum. | Names ellipsize; icons keep size; row heights unchanged. | FR-010, Edge: narrow |
| B9 | Showcase: the `StateIcon` entry, then a greyscale screenshot of B1. | Three different shapes; each legible in both schemes. | FR-007, FR-012, US3, SC-002 |
| B10 | Open a second window on P; repeat B2. | Both windows change together. | Edge: several windows, SC-003 |
| B11 | From the B1 sidebar alone, name every worktree's state. | Matches the setup, in under 5 s. | SC-001 |

## §C — macOS and Windows

Repeat B1, B3 and B9 on macOS and Windows. Expected: the same glyphs and results (Principle VI).

**Recorded 2026-10-17 (close unit): handed to CI's OS matrix, not run on a real macOS or Windows
machine** (none here). What CI's three-platform leg covers: `showcase_completeness` (the `StateIcon`
showcase entry exists and renders every state) and the core/daemon tests, which include the
`location_state` rule table and the protocol round-trip. The glyphs are Material Symbols codepoints
pinned in `icons.rs` and `icons_font.rs` against the bundled font, so the same shapes ship on every
platform. B1 and B3 on real macOS and Windows hardware stay a manual check for a release candidate.
