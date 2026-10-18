# Quickstart: Quick-Switcher

**Feature**: 968 | **Plan**: [plan.md](./plan.md) | Contracts: [keys](./contracts/keys.md),
[ui](./contracts/ui.md) | Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree (in the cloud environment: `scripts/gate.sh`).
- For §B: the `visual-pass` skill's private display, client and daemon built together, a private
  data directory.
- For §C: a macOS bundle from `mise run app` and an installed Windows build.

## §A — Automated

```sh
mise run test-core   # quick_switch filter/rank/reseat/step, 500-entry timing (SC-003)
mise run gate        # keymap decode/route/near-miss/docs, quick_switch candidates/gate/choose/close, registry, showcase
```

Expected: both green; each test in [plan.md](./plan.md#test-strategy-by-layer) fails without the
change it covers.

## §B — Recorded pass on Linux (visual-pass)

Known projects P (available, active), Q (available, 2 running), R (folder renamed away:
unavailable). P has Default (one session), `feat-1_login-page` (main + two helpers named
`helper-a`, `helper-b`), `fix-2_logout` (no session), `missing-3_x` (folder removed: `missing`),
ten more worktrees with a session each, and a tag filter that hides `chore-*` rows except the
current one.

| # | Do | Expect |
|---|---|---|
| B1 | Terminal focused; press Ctrl+Shift+P | dialog over a scrim, field focused, empty; list: Default, its session, each location then its sessions, then Q and R; current location and session marked; first entry highlighted; terminal shows no `^P` |
| B2 | Type `logpg` | only `Login page` and what matches in order; Login page highlighted, its matched letters emphasised |
| B3 | Down twice, Enter | `helper-b`'s sub-row is selected under the opened Login page row; terminal shows helper-b and has the keyboard |
| B4 | Ctrl+Shift+P, type `logout`, Enter | Logout row selected with its start-session action; no session started; the row has the keyboard ring |
| B5 | Ctrl+Shift+P, Down to the end | highlight stops on Q; R is shown with **unavailable**, never highlighted; clicking R does nothing |
| B6 | Enter on Q | Q becomes active, its current session restored; P's sessions still running (count unchanged) |
| B7 | Ctrl+Shift+P, type `zzzz` | "No matches"; Enter keeps it open |
| B8 | Escape; then reopen and click the scrim; then reopen and press the chord | each time it closes and the terminal has the keyboard again |
| B9 | Open with a very long worktree name in view | name on one line, cut with "…", emphasis visible |
| B10 | Open the project switcher panel, press the chord | nothing opens |
| B11 | Open the switcher, from another window start a session in P | it appears in the list; query and highlight kept |
| B12 | Dark and light theme | dialog, rows, emphasis and disabled row legible in both |

## §C — macOS and Windows

On each: Cmd+Shift+P (macOS) / Ctrl+Shift+P (Windows) with a terminal focused opens the switcher
and sends nothing to the shell (run `cat -v`, expect no output); no system action fires; B2–B4
behave as on Linux. On a non-US layout (e.g. German, French), the P key position opens it.
