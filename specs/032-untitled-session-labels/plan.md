# Implementation Plan: A session the AI CLI never titled still gets a label

**Branch**: `fix/the-name-of-past-session-is-still-not-shown` | **Date**: 2026-09-19 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/032-untitled-session-labels/spec.md`

## Summary

Feature 029 remembers every title an AI CLI records, and a session with none stays `Pending` —
"New session" — for good. The four sessions in 029 BUG-001 have real conversations and no
`ai-title`; a Copilot session from 1.0.36 or older has a `summary:` title the application never
reads (the 44 such sessions on the development machine are not listed at all, so FR-016 is verified
on listed-session fixtures; D10).

The change, all behind seams that already exist:

1. **A second, lower-priority label source.** `AiCliProvider` gains `read_label`: the
   conversation's first typed turn, read from a bounded 1 MiB prefix of the record file and shaped
   to one line of at most 80 graphemes (FR-002, FR-003, FR-012, FR-014). `claude` and Copilot
   implement it; `pi` answers `None`.
2. **A third label kind.** `SessionLabel::Derived(String)` sits between `Pending` and `Named`; it is
   persisted in a new `StoredSession.label` field and carried on the wire (protocol 13 → 14), so
   "label or title" survives a restart and a label never passes for a title (FR-005, FR-007,
   FR-008).
3. **Precedence in the daemon's existing passes.** Discovery and the 029 recovery passes fall back
   from title to label; recovery now also revisits `Derived` sessions, so a title that arrives later
   replaces the label (FR-006). Recovery counts labels toward its broadcast, and the spinner route
   re-arms the live lookup like the hook route does, so a running session's label shows within a
   tick or two (FR-010; research R9).
4. **Copilot's `summary:` is a title** when `name:` is absent (FR-016, D9).
5. The client adopts a `Derived` label as it adopts a title; the user guide says what an untitled
   row reads (FR-015).

## Technical Context

**Language/Version**: Rust, pinned to `stable` by `rust-toolchain.toml`.

**Primary Dependencies**: `serde` / `serde_json` (records, catalog), `uuid`, `tokio` (daemon hops,
unchanged). **New direct dependency of `micold-core`: `unicode-segmentation`** (already in
`Cargo.lock` via iced/cosmic-text/winit; research R8). `iced` untouched.

**Storage**: Local JSON only — the per-project state file (`micold_core::store`), additive
`label` field, no `schema_version` bump (research R2). Read-only access to the AI CLIs' own stores
(`~/.claude/projects`, `~/.copilot/session-state`).

**Testing**: `mise run test-core` while iterating the core; `mise run gate` before push. Layers per
requirement: research R11 and [quickstart.md](./quickstart.md) Part A.

**Target Platform**: Linux, macOS, Windows desktop; sandboxed and host daemon placements.

**Project Type**: Desktop application; three-crate Cargo workspace (`micold-core`, `micold-client`,
`micold-daemon`).

**Performance Goals**: SC-006 — the list of a 50-session project appears no slower. A label costs at
most one 1 MiB prefix read per untitled session, once (then remembered); the per-refresh title
re-read set is the same untitled set 029 re-reads today (research R7, R9).

**Constraints**: Offline (Principle IV). Every read and write best-effort (FR-011). The daemon stays
the catalog's single writer. No `cfg(target_os)` arm (Principle VI).

**Scale/Scope**: Tens of sessions per project; transcripts of up to ~10 MB; two providers changed.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design below.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every change is render-free and reachable from
  `tests/`: `first_turn.rs` pure functions, provider methods over temp directories, `store.rs`
  round-trip, `Catalog`/`DaemonState` integration with the real providers over temp stores, and the client's render-free
  `catalog_sync.rs`. No GUI glue changes, so the GUI exception is not invoked. Each contract clause
  (C1–C8) is a test written red first.
- [x] **II. Multi-Session Support**: PASS. The label is per-session state, written through
  `Workspace::find_session_mut(SessionId)`; each session reads its own provider's store under its own
  id and cwd (029 contract C16). A dedicated test gives two sessions in one worktree, one titled and
  one not, and checks neither takes the other's (spec *Concurrency*). Title-vs-label races are
  decided under the catalog lock (C6.3).
- [x] **III. Worktree Integration**: PASS. No lifecycle change; cwd comes from `SessionLocation::cwd`
  as in discovery and 029 recovery.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Local reads of the CLIs' own files and a
  local state file; nothing leaves the device. The label may contain pasted secrets; it stays in the
  same local file as a title (spec edge case).
- [x] **V. Rust + iced Stack**: PASS. "Title, label or neither" is a three-variant enum, so a label
  that passes for a title, or a session with both, is unrepresentable in memory (research R1).
- [x] **VI. Cross-Platform Parity**: PASS. No platform branch: paths from the existing provider
  derivations, line splitting on `\n` (`claude` and Copilot write `\n` on all three platforms; a
  `\r` left before it is JSON whitespace and parses), grapheme shaping platform-neutral. CI runs all
  three.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md`
  (*"New session" means…* and *Sessions from before this was true…*) is updated in the milestone
  that ships each behaviour (FR-015; Copilot `summary:` in M2, the running-session sentence in M3).
- [x] **VIII. Reusable UI Component Foundation**: PASS (not engaged). No widget change; the row,
  tooltip and terminal bar render `SessionLabel::display()`.

## Project Structure

### Documentation (this feature)

```text
specs/032-untitled-session-labels/
├── plan.md              # This file
├── research.md          # Phase 0 (R1–R11)
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1 (Part A automated, Part B on the dev machine)
├── contracts/
│   └── first-turn-label.md   # C1–C8
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/
├── Cargo.toml                 # + unicode-segmentation (workspace dep, root Cargo.toml)
├── src/
│   ├── first_turn.rs          # NEW — read_prefix, claude_first_turn, copilot_first_turn, shape_label
│   ├── lib.rs                 # + mod first_turn
│   ├── session.rs             # SessionLabel::Derived, Session::set_derived_label
│   ├── store.rs               # StoredSession.label
│   ├── provider.rs            # AiCliProvider::read_label (+ 4 impls); Copilot read_title summary:
│   └── protocol/version.rs    # PROTOCOL_VERSION 14
└── tests/
    ├── first_turn_label.rs          # NEW — C2–C5
    ├── first_turn_label_corpus.rs   # NEW — #[ignore] real-store probe (quickstart B1)
    ├── fixtures/first_turn/         # NEW — synthetic claude/Copilot records
    ├── copilot_provider.rs          # + C4 via provider, C7
    ├── ai_cli_provider.rs           # + claude read_label via provider (C3 over a temp dir)
    ├── ai_cli_provider_seam.rs      # + fake with_label
    ├── session_name_round_trip.rs   # + C8.1–C8.3
    └── schema_hash.rs               # pinned version 14

crates/micold-daemon/
├── src/
│   ├── catalog.rs             # + record_session_label
│   └── state.rs               # discovery fallback; recovery candidates = not Named; label step;
│                              # labels counted; drain_signals re-arms name_stale on spinner
└── tests/
    └── untitled_session_labels.rs   # NEW — C6, FR-001/004/005/006/009/010/011, isolation

crates/micold-client/
├── src/catalog_sync.rs        # adopt Derived as well as Named
└── tests/session_title_sync.rs     # + C8.5

docs/user-guide/worktrees-and-sessions.md   # FR-015 (M1; + Copilot in M2; running session in M3)
```

**Structure Decision**: the existing workspace. Record-format knowledge stays below the provider
seam (`first_turn.rs` is called only by `provider.rs`); precedence stays in the daemon, which is the
catalog's single writer; the client only stops ignoring a non-`Pending` label.

## Delivery shape (input to milestones)

- **`claude` first** — the reported bug: provider rule, `Derived` everywhere it has to exist,
  precedence, persistence, wire, client, user guide. Past and running `claude` sessions (US1 #1–4,
  US2, US3 for `claude`).
- **Copilot second** — `summary:` title (FR-016, US1 #6, SC-008) and Copilot `read_label` (US1 #5,
  SC-009, US3 for Copilot), plus the corpus probe and Polish.

Until the Copilot half lands, `CopilotProvider::read_label` returns `None`, so Copilot rows behave
exactly as today — no half-wired state on `main`.

## Post-Design Re-check

All eight principles still PASS with the contract in hand. Two points:

- **II** is enforced at two levels: `Session::set_derived_label` refuses anything but `Pending`, and
  the catalog re-checks under the lock, so FR-005 does not rest on caller care.
- **V**: the wire bump (research R3) is the price of keeping the enum honest across the socket;
  accepted over a wire that sends labels as titles.

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected because |
|---|---|---|
| New `micold-core` dependency `unicode-segmentation` (new to the daemon and sandbox image) | FR-003 counts user-perceived characters | `chars()` splits grapheme clusters; the crate is already compiled into the client via iced, is dependency-free, MIT/Apache-2.0 (research R8) |
| `PROTOCOL_VERSION` 13 → 14 | `SessionLabel::Derived` is wire-visible | sending labels as `Named` makes the wire misstate the catalog (research R3) |
