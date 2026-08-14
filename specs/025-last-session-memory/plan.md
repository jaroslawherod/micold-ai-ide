# Implementation Plan: Reopen on the session I was last using

**Branch**: `feat/025-last-session-memory` | **Date**: 2026-08-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/025-last-session-memory/spec.md`

**Bugfix**: 2026-08-14 — [BUG-001](./bugs/BUG-001.md) Updated from bugfix patch. This plan reasoned
about *which* session is current and about not starting it, and never asked what the terminal area
already said about the state those two combine to produce. See the Complexity Tracking note.

## Summary

The memory already exists and is thrown away at exit. `State::foreground_by_project` maps each
project to the session that was last in front of the user, and feature 008's BUG-001 just made the
restore honour a session whose process has stopped. What it lacks is a home on disk: the map lives
on the client's `State`, so quitting forgets it, and `boot()` never makes any session current — the
gap feature 024's research recorded as R12.

Three changes, and none of them is new machinery:

1. **Move the map into `Workspace`**, where it is persisted alongside sessions and worktree names
   in the per-project state file. One map with one meaning, rather than an in-memory one and a
   stored one that could disagree.
2. **Persist it from the daemon**, which already receives `SetViewedSession` on every path that
   changes which session is in front of the user, and is already the single writer of the store.
3. **Apply it at launch**, in `boot()`, by calling `restore_after_activation` — the same function
   a project switch uses. (The plan first said the opposite, to avoid that function's
   `focus_terminal()`; implementing it showed the requirement behind that was wrong. See research
   R5.)

There is **no protocol change** and therefore no schema-hash churn: the wire message this needs is
already sent, and the client reads the memory from its own load rather than waiting for the daemon.

## Technical Context

**Language/Version**: Rust, stable (pinned by `rust-toolchain.toml`)

**Primary Dependencies**: None new. `serde` for the one added field; the client and daemon already
share `micold_core::store::JsonFileStore`.

**Storage**: The existing per-project state file (`projects/<hash>.json`, `StoredProjectState`),
which already holds that project's sessions and worktree display names. One added field, defaulted,
so an older file loads unchanged and a newer file is ignored by an older reader — the same
compatibility argument `store.rs` records for feature 008's BUG-001 split, and the reason no schema
version moves.

**Testing**: `mise run test` (whole workspace, matching CI); `mise run test-core` while iterating on
the store and the resolution. The launch path itself is binary glue and falls under Principle I's
GUI/process-spawn exception — its decision (*which* session) is in the tested reducer, and only the
call from `boot()` is glue.

**Target Platform**: Linux, macOS, Windows desktop (CI covers all three).

**Project Type**: Desktop application — existing three-crate workspace, no new crate.

**Performance Goals**: No change. The memory is one id per project, written on a message the daemon
already handles and read from a file the client already loads.

**Constraints**: The daemon is the single writer of the store (`store.rs` has no locking, and the
client writing it would silently clobber whatever the daemon wrote since load). The client must
therefore *read* the memory and never persist it — the same split sessions already have.

**Scale/Scope**: One field in the store, one field on `Workspace`, one daemon write path, one call
in `boot()`. Four files in `micold-core`/`micold-daemon`/`micold-client`, plus a user-guide page.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Everything that decides anything is render-free and
  testable: the store round-trip (`micold-core`), the daemon recording the viewed session, and the
  resolution at launch (`explain_foreground`, already tested). The only glue is `boot()` calling
  it — no branch of its own, which is what the exception covers.
- [x] **II. Multi-Session Support**: PASS. No session is created, stopped, or mutated. FR-004 is
  explicit that restoring starts nothing, and there is a test for it. The memory is one id per
  project, so no session's state can leak into another's.
- [x] **III. Worktree Integration**: PASS. No worktree is created, switched or removed. A remembered
  session whose worktree is gone simply fails to resolve (FR-005) — the same path a stale id
  already takes.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS, and this is the principle the feature is
  *about*. The memory is one more field in a file the application already owns on the user's disk;
  nothing is transmitted (FR-011). Unreadable or outdated state degrades to "no memory" and never
  blocks a launch (FR-010) — the recovery behaviour `store.rs` already implements for the file as
  a whole.
- [x] **V. Rust + iced Stack**: PASS. `Option<SessionId>` rather than a sentinel; the absence of a
  memory is the `None`, not an id that might not exist. Whether the remembered session is usable
  stays a function of the sessions actually present, so a stale id cannot be represented as valid.
- [x] **VI. Cross-Platform Parity**: PASS. No platform branch — a serialised field and a lookup.
  Paths are already canonicalised by the store.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md` gains
  what reopening does, in the section that already describes switching and the current-session
  mark.
- [x] **VIII. Reusable UI Component Foundation**: PASS — trivially. No UI component is added or
  changed. The restored session is presented by the existing reveal (FR-012), which is feature
  024's, unchanged.

Re-checked after Phase 1 design: unchanged, all PASS. The design added no new crate, no new
component, no protocol message and no schema version; the largest new surface is one field on a
struct that is already persisted.

## Project Structure

### Documentation (this feature)

```text
specs/025-last-session-memory/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── last-session-memory.md   # What is remembered, who writes it, what restores it
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── store.rs                        # +last_session on StoredProjectState (serde default);
│                                   # read into Workspace on load, written on save
└── workspace.rs                    # +foreground_by_project, moved here from client State so the
                                    # memory has one home and can be persisted

crates/micold-daemon/src/
└── catalog.rs                      # record the viewed session + persist (the single writer)
crates/micold-daemon/src/server.rs  # SetViewedSession also updates the catalog, not only
                                    # the per-client `viewed` map

crates/micold-client/src/
├── app.rs                          # foreground_by_project moves out of State
├── features/session.rs             # record_foreground / explain_foreground read the new home
└── main.rs                         # boot() applies the memory via restore_after_activation

crates/micold-core/tests/           # store round-trip, back/forward compatibility
crates/micold-client/tests/         # resolution at launch, and that nothing is started
docs/user-guide/worktrees-and-sessions.md
```

**Structure Decision**: The existing workspace, unchanged in shape. The one structural decision is
*where the memory lives*: moving `foreground_by_project` from the client's `State` into
`micold_core::workspace::Workspace` is what makes it persistable at all, and it puts the map beside
the sessions it refers to — the same file, the same load, the same writer.

## Complexity Tracking

> No constitution violations. The table records the two designs this rejected, because both are the
> obvious first move and both are worse.

| Considered | Why it looked necessary | Why it was not taken |
|-----------|------------------------|---------------------|
| Add `last_session` to `ProjectSnapshot` and carry it over the wire | The daemon owns persistence, so the client "should" learn it from the catalog | It is a protocol change — schema hash, version handshake, both sides — to deliver a value the client can already read from its own load at boot. It would also arrive *later* than the first frame, so the launch would still start on the overview and then jump. (research R3) |
| Keep the map on the client's `State` and persist it separately | Smallest diff; nothing moves | Two homes for one fact, which can disagree — and the client cannot write the store without clobbering the daemon (`store.rs` has no locking). The map has to move to be persistable by the writer that owns the file. (research R2) |

### Added by [BUG-001](./bugs/BUG-001.md) — a new state, an inherited sentence

This feature adds no widget and no screen, which is why the whole plan reads as persistence work.
What it does add is a **state that was previously unreachable**: a session current at launch with no
process and no output. The terminal area already had wording for "nothing to show" —
`Starting…` — written for feature 006, where the only way to have no grid was that a process was on
its way. That assumption is what this feature invalidates.

So the cost of this feature was not only the field and the lookup. It was also every inherited
sentence whose truth depended on a state being unreachable. Nothing in the plan prompted anyone to
go looking for those, and a diff that adds no UI does not show them.

The rejected fixes are worth recording beside the rejected designs above:

| Considered | Why it looked necessary | Why it was not taken |
|-----------|------------------------|---------------------|
| Render nothing at all when there is no grid | The placeholder is wrong, so remove it | Trades a wrong sentence for a blank rectangle. The user is looking at a session they asked to return to; "nothing here, no reason given" is not an improvement on a wrong reason |
| Have the restore populate an empty grid | Then the pane has something to render and the branch is unreachable again | The client would be fabricating terminal state for a process that never ran. It also re-hides the branch rather than fixing it, so the next feature to reach it inherits the same trap |
