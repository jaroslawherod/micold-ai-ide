# Implementation Plan: Refresh the Worktree List on Demand

**Branch**: `feat/refresh-worktrees-list` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/029-refresh-worktrees-list/spec.md`

## Summary

A third control joins the sidebar header, beside "Add a worktree" and "Hide sidebar", that re-reads
the active project's worktrees on demand.

The re-read itself already exists and is correct: `DaemonState::refresh_worktrees` re-discovers from
git and the filesystem, and `refresh_worktrees_and_broadcast` composes it with a catalog push. What
does not exist is a way for the user to ask for it — the six existing call sites are all internal
consequences of something else (attach, create, delete, include, exclude, project add). This feature
adds a seventh trigger and nothing more on the daemon side.

The client side is where the work is: one new correlated RPC, one boolean of in-flight state owned
by the worktree feature, a structurally-enforced single-flight, the codebase's first wire timeout,
and one new icon. The listing arrives back through the `CatalogChanged` path every other worktree
operation already uses, so FR-004's "a user must not be able to tell which trigger produced it" is
satisfied by not building a second path at all.

## Technical Context

**Language/Version**: Rust (stable, pinned by `rust-toolchain.toml`)

**Primary Dependencies**: `iced` (client GUI), `tokio` (client + daemon async), `serde` (wire),
`ttf-parser` (font gate, test-only). No new dependency.

**Storage**: None added. The refresh is transient; nothing about it is persisted (spec Assumptions).
Git remains the single source of truth for worktrees (`state.rs:80`).

**Testing**: `mise run test` (workspace). Logic-only iterations: `mise run test-core`. The layout
gates live in `micold-client`'s `layout_snapshot` test binary and need the client built.

**Target Platform**: Linux, macOS, Windows desktop (Constitution VI).

**Project Type**: Desktop application — a three-crate workspace (`micold-core`, `micold-client`,
`micold-daemon`).

**Performance Goals**: SC-002 — the refreshed list on screen within 2 s for ≤50 worktrees, with the
interface accepting input throughout. Met by the existing `spawn_blocking` discipline (research R9),
not by new work.

**Constraints**: SC-004 — visible feedback within 100 ms of the press, i.e. on the next frame, which
rules out waiting for the reply to change anything. The sidebar header has no spare width
(research R10). No new component may be added without its showcase entry and anatomy gate, so the
plan is built to need none.

**Scale/Scope**: One new wire message, one new icon, one new boolean field, ~4 new messages, 6
touched source files across three crates, plus tests and two docs pages.

## Constitution Check

*GATE: passed before Phase 0 research; re-checked after Phase 1 design — see the re-check below.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: Every unit of new logic is reducer-level or shell-level
  and is preceded by a failing test. The reducer transitions (`RefreshRequested` /
  `RefreshFinished` / `RefreshTimedOut`, the single-flight guard, `can_refresh_worktrees`) are pure
  and tested in `tests/features_worktree.rs`. The shell functions (`on_worktree_refresh_requested`,
  the timeout handler, the two reply arms) are tested in `daemon_sync.rs`'s own `#[cfg(test)]`
  module, which is where that file's existing tests live (`daemon_sync.rs:1805`) —
  **`src/shell/` is deliberately not claimed under the GUI-glue exception**, which names
  `src/main.rs`, `src/ui/` and `src/showcase/` only. The daemon arm is tested in
  `micold-daemon/tests/` against a real codec over an in-memory duplex, modelled on
  `mutation_semantics.rs`. What *is* claimed under the exception is the view change in
  `ui/sidebar.rs` and the one-line routing arm in `main.rs`: both invoke already-tested pure logic
  and branch on nothing of their own (research R7).
- [x] **II. Multi-Session Support**: No session state is read or written. A refresh replaces the
  worktree listing; `State::set_worktrees` already reconciles session references when a listing
  changes (`app.rs:431-453`), and this feature routes through it rather than around it.
- [x] **III. Worktree Integration**: The feature *is* worktree integration — it closes the gap where
  a worktree created outside the app required a manual project switch to become visible. No manual
  git step is introduced; git is read, never written.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Reads the local repository and filesystem only.
  `worktree::discover` runs `git worktree list --porcelain`, which contacts no remote — the same
  guarantee `BranchList` already documents (`messages.rs:369`). Nothing leaves the device. Works
  fully offline.
- [x] **V. Rust + iced Stack**: Rust and iced only. The in-flight state is a `bool` on the worktree
  feature rather than an untracked ambient flag, and the single-flight invariant is made
  unrepresentable at the point that matters: a busy button carries no `on_press`, so no second
  request *message* can exist to be sent (research R7).
- [x] **VI. Cross-Platform Parity**: No platform-specific code. Path handling goes through
  `std::path` inside the existing `discover`. CI covers all three platforms; this change affects the
  build, so no documentation-only exemption applies.
- [x] **VII. Documentation First-Class**: User-facing, so both docs land in this change:
  `docs/user-guide/worktrees-and-sessions.md` gains a subsection under "Browsing worktrees", and
  `docs/user-guide/icons.md` gains its table row. Both are in-repo and CI-verified.
- [x] **VIII. Reusable UI Component Foundation**: **No new component.** The control is
  `IconButton` + `Tooltip`, the same two primitives its two neighbours are built from, via their
  existing chainable builders terminating in `.into()`. The one option seriously considered that
  *would* have added a component — a circular progress spinner — was rejected in research R6 with
  its cost stated (showcase catalogue entry, anatomy figure, idle-frame interaction).

**Post-design re-check**: unchanged, all eight still PASS. The design added no component, no crate,
no dependency, and no persisted state. The one thing worth naming again is the timeout in R5: it is
the codebase's first wire timeout, and it is a *shell* function with tests, not glue — so it does
not lean on Principle I's exception. Complexity Tracking below is empty because nothing needed a
justified deviation.

## Project Structure

### Documentation (this feature)

```text
specs/029-refresh-worktrees-list/
├── plan.md              # This file
├── spec.md              # The specification
├── research.md          # Phase 0 — R1..R10
├── data-model.md        # Phase 1 — the state and transitions
├── quickstart.md        # Phase 1 — validation, Part A automated / Part B by eye
├── contracts/
│   └── worktree-refresh.md   # The wire contract for the new RPC
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify)
└── tasks.md             # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

```text
crates/
├── micold-core/
│   └── src/protocol/
│       ├── messages.rs          # + ClientMsg::WorktreeRefresh { req, project }
│       └── version.rs           # PROTOCOL_VERSION 9 -> 10 (one edit, research R3)
├── micold-daemon/
│   ├── src/server.rs            # + the route() arm: refresh_and_broadcast, then ack
│   └── tests/worktree_refresh.rs        # NEW — the RPC over a real codec
└── micold-client/
    ├── src/
    │   ├── icons.rs             # + Icon::Refresh (U+E5D5), ALL, glyph()
    │   ├── app.rs               # + State::can_refresh_worktrees() (pure predicate)
    │   ├── features/worktree.rs # + refreshing: bool; + 3 Msg variants + their reducers
    │   ├── shell/daemon_sync.rs # + PendingOp::WorktreeRefresh, the send, the timeout,
    │   │                        #   the OperationOk / OperationError arms
    │   ├── main.rs              # + one routing arm (glue)
    │   └── ui/sidebar.rs        # + the header control (glue)
    └── tests/
        ├── features_worktree.rs         # reducer transitions + single-flight
        ├── icons.rs                     # codepoint pin for Icon::Refresh
        └── fixtures/layout_snapshot.txt # regenerated — the header gains a control

docs/user-guide/
├── worktrees-and-sessions.md    # + "Refreshing the list" under "Browsing worktrees"
└── icons.md                     # + the Refresh table row
```

**Structure Decision**: the existing three-crate workspace, unchanged. The feature crosses all three
because the wire type belongs to `micold-core`, the git re-read to `micold-daemon`, and everything
the user touches to `micold-client` — which is the split the codebase already enforces. Within the
client it follows feature 028's layout: the message vocabulary and state in `features/worktree.rs`,
the external-system conversation in `shell/daemon_sync.rs`, the rendering in `ui/sidebar.rs`, and a
single routing arm in `main.rs`.

## Implementation Phases

Ordered so the one risk that can fail for reasons unrelated to this feature's logic is met early
(research R10), and so nothing user-visible exists before the logic under it is tested.

**Phase A — the wire (core + daemon).** `ClientMsg::WorktreeRefresh`, the version bump in the same
edit, the daemon arm, and `tests/worktree_refresh.rs` driving it over a real codec. Ends with the
daemon able to serve a refresh nothing yet asks for. Verifiable on its own.

**Phase B — the client's logic.** `refreshing` on the worktree feature, its three messages and
reducers, `can_refresh_worktrees`, `PendingOp::WorktreeRefresh` and the send/reply/timeout functions
in `daemon_sync.rs`. All tested; still no control on screen.

**Phase C — the icon.** `Icon::Refresh` in its three places at once, with the codepoint pin. Small
and separable; its two gates (`icons.rs`, `icons_font.rs`) either pass immediately or name exactly
what is wrong.

**Phase D — the control, and the header-width risk.** Add the button to `ui/sidebar.rs` and the
routing arm to `main.rs`, then **immediately** run `layout_text_overflow` and the `layout_snapshot`
gates. If the header squeezes, take research R10's fallback 1 (ellipsize the title) before going
further — not after, because everything downstream would need re-recording.

**Phase E — snapshot and docs.** Regenerate `layout_snapshot.txt` deliberately
(`UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`), then write both
docs pages.

**Phase F — the visual pass.** quickstart §B by eye, via the repo's `visual-pass` skill. The one
thing no gate can answer is R6's open question: whether an inert greyed button reads as *busy* or as
*broken*. If it reads as broken, that is a finding to file, not a thing to fix inside this feature.

## Risks

| # | Risk | Mitigation |
|---|------|-----------|
| 1 | The header has no spare width; a fourth control pushes "Worktrees" past its clip at the 180 px minimum (research R10) | Met in Phase D, before anything downstream. Fallback: ellipsize the title with the existing `Ellipsized` component |
| 2 | The 30 s timeout is the codebase's first, and a copied second one would be worse than a shared helper | Recorded in research R5 and in the code beside the constant: the next operation that wants one lifts this into a helper |
| 3 | An inert greyed button is a weaker "busy" cue than a spinner, and is ambiguous with FR-005's "no project" greying | Tooltip text distinguishes them; the completion snackbar carries the real signal. Phase F looks at it deliberately; a spinner is a separate change with its own gates |
| 4 | A refresh landing at the same moment as a create/delete could push two catalogs | No mitigation needed: both go through `refresh_worktrees_and_broadcast`, which is idempotent — the second re-read simply reports the same truth |

## Complexity Tracking

No Constitution Check violations. Nothing to justify — and after the implementation that is a
result rather than an omission, so here is what was actually checked at the end (T040, T043).

**Principle VI, checked rather than claimed.** The whole diff was read for the three things that
break parity, and it contains none of them: no `cfg(target_os)` / `cfg(unix)` / `cfg(windows)`
anywhere, no `Command::new`, and no new path handling in `src/` — the one path that moves is
`ClientMsg::WorktreeRefresh { project: PathBuf }`, which is the same field every other worktree RPC
already carries and is read on the daemon side by the same `discover` that served it before. Every
other `PathBuf` in the diff is a test fixture. The one new file that touches the filesystem is
`micold-client/tests/refresh_is_only_on_demand.rs`, a source scan, and it normalises separators the
way `documentation_is_not_read.rs` does (`replace('\\', "/")`) so it reads the same on Windows.

**Three implementation deviations from the plan, none of them a violation.**

1. `can_refresh_worktrees()` was written into `features/worktree.rs` — inside the existing
   `impl crate::app::State` block, beside `has_visible_worktrees` — rather than into `app.rs` as
   T017 said. The predicate reads one worktree-feature field and one workspace field; putting it
   next to the sibling predicate that does the same keeps both discoverable, and `app.rs` stays
   what feature 028 made it. The reason is written in its doc comment rather than only here.
2. No `main-shell-sidebar-expanded-refreshing` covered state was registered (T032). The busy form
   withholds a press and swaps a tooltip, and neither is layout — so a second fixture block would
   have recorded a byte-identical copy of the first. T032's own escape clause allows saying so in a
   comment; what was added instead is `gates/refresh_busy_holds_the_header.rs`, which *asserts* the
   identity and fails on the day it stops being true. A comment that can rot was traded for a check
   that cannot.
3. T035 was answered by an existing test rather than a new one. The transition sweep in
   `features_worktree.rs` already drives `RefreshRequested`, `RefreshFinished` and
   `RefreshTimedOut` and asserts the listing and the arrangement are untouched; the traceability
   was recorded in that test's doc comment. A second copy would have been the duplication this
   codebase's comments repeatedly warn against.

**Two quickstart rows were wrong and were corrected (T039).** §A.2 named `handshake.rs` for the
version bump — it reads the constants symbolically and cannot fail on one — and
`bar_controls_hold_their_size.rs` for FR-001, which reads the terminal's bottom bar rather than the
sidebar header. The audit is recorded under the table with what actually answers each row. It is
worth noting the shape of the error: both rows named a real, passing gate, so nothing would ever
have reported that the obligation was unmet.
