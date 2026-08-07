# Quickstart: Validating the Feature-Module MVU Architecture

**Feature**: 021 | **Date**: 2026-08-07 | **Plan**: [plan.md](./plan.md)

How to prove this feature works. Because it changes no user-visible behavior, validation is mostly
**measurement** and **the existing suite staying green** — not new manual scenarios.

## Prerequisites

```bash
mise trust                 # once per fresh worktree
mise run test              # baseline: whole workspace must be green before starting
```

Do not alternate between `mise run <task>` and bare `cargo` in one session — they use different
toolchain patch releases over the same `target/`, and switching rebuilds every dependency.

## Per-step validation (SC-009)

Every one of the 20 migration steps (research.md §6) must leave the application buildable, runnable
and green. Run after **each** step, and commit only on green:

```bash
mise run test              # whole workspace, matches CI
mise run test-core         # faster inner loop for core-only changes
mise run run               # the app must still launch and work
```

SC-009 is verified from git history, not just the endpoint: each step is its own commit, so
`git log` is the evidence. A step that needs a later step to compile is a planning error, not an
acceptable intermediate.

## Success criteria — how each is checked

| SC | Check | Command / method |
|---|---|---|
| **SC-001** | New floating surface touches one module + ≤1 registration line; zero central match edits | Permanent guard `surface_registration_cost.rs` (not a one-time count) |
| **SC-002** | New feature touches one module + ≤1 registration line, zero edits to other features | Permanent guard `feature_registration_cost.rs` |
| **SC-002a** | Both guards remain in the suite after this feature ships | Present in `crates/micold-client/tests/` |
| **SC-003** | Neither file among the largest; neither holds >1 feature. 500 lines indicative, **not a gate** | `find crates -name '*.rs' -exec wc -l {} + \| sort -rn \| head -10` |
| **SC-004** | Every feature module has a test using only its own types | Per-module test presence; see below |
| **SC-004a** | Per-feature nesting verdict recorded with evidence | research.md §5 — **already satisfied** |
| **SC-004b** | Tiers 1, 2 and shell split green with zero Tier 3 merged | Green CI at step 16, before step 17 |
| **SC-005** | Every capability has a fake; behavior tested through it with zero real I/O | `mise run test` with the per-capability tests |
| **SC-006** | Whole pre-existing suite passes, zero assertions modified, all three platforms | CI on Linux + macOS + Windows |
| **SC-007** | Zero direct cross-feature reducer writes; adding one fails a named guard | `feature_write_isolation.rs` |
| **SC-008** | Pre-change state loads and behaves identically | Manual procedure below |
| **SC-009** | Every step green in its own commit | `git log` + CI per commit |
| **SC-010** | "Where does feature X live?" answered by one module | Review against research.md §1 |

### SC-003 measurement

```bash
find crates -name '*.rs' -type f -exec wc -l {} + | sort -rn | head -10
```

Baseline at plan time: `main.rs` 3,567 and `app.rs` 2,434 are #1 and #2. Success means neither
appears near the top, and the next-largest files (`server.rs` 1,483, `terminal_pane.rs` 1,348,
`state.rs` 1,317) are the new leaders.

**Settled 2026-08-07**: FR-005 governs. A file containing exactly one feature and no longer among
the largest satisfies SC-003 **at any length**. The 500-line figure is a progress signal. Splitting
a coherent single-feature module to cross a numeric threshold is explicitly forbidden — it would
make the codebase worse while scoring the criterion green.

### SC-004 — per-feature isolation test

For each of the nine feature modules, a test that constructs **only** that feature's types:

```bash
mise run test -- features::
```

The test must not name any unrelated feature's types. This is the falsifiable form of "reason about
one feature in isolation" (spec User Story 2).

### FR-027 — no assertion modified

The single most important check in the feature. The existing suite is the behavior specification.

```bash
# No existing test file may have an assertion changed. Additions are fine;
# relocations are fine (research.md §3). Rewrites and deletions are not.
git diff main...HEAD -- crates/*/tests/ | grep -E '^-.*assert'
```

**Any output from that command is a defect** unless it is a pure relocation with the identical
assertion re-added elsewhere in the same diff.

## Manual procedures

Two things cannot be automated. Both fall under Principle I's GUI/process-spawn wiring exception —
thin glue with no decision logic of its own — and are recorded here as that exception requires.

### M1 — Persisted state loads identically (SC-008, FR-026)

1. Check out `main` at `44b9fd1` (pre-change) and `mise run run`.
2. Open a project, create a worktree, start two sessions, set a non-default theme and scrollback,
   apply a sidebar filter. Quit.
3. Back up the state directory.
4. Check out the post-change branch and `mise run run`.
5. **Expect**: the same project, worktrees, sessions, theme, scrollback and filters, with no
   migration prompt, no warning, and no rewrite of the state files.
6. Quit, and diff the state directory against the backup. **Expect**: no structural change.

### M2 — Overlay behavior is unchanged (FR-011, FR-012, FR-013)

The exit-animation snapshot is the riskiest obligation in the feature (research.md §9). The
automated suite covers it, but these are the behaviors most likely to shift in a way a test misses.

1. Open the About dialog; press Escape. **Expect**: it animates out, not disappears.
2. Open it again *while it is animating out*. **Expect**: it reverses smoothly; no flicker, no
   duplicate.
3. Open the sidebar filter panel, set two filters, close the panel. **Expect**: filters still
   applied (FR-013).
4. Open the project switcher, then right-click a project to open its context menu (both open at
   once). Press Escape. **Expect**: the context menu closes, the switcher stays (FR-012, D1).
5. With a popover open, open a modal. **Expect**: the popover closes (FR-012, D2).
6. Scroll the area beneath an open popover. **Expect**: it dismisses.

Run M2 after step 11 — the point where `Overlay` and `ClosingOverlay` are deleted.

## Documentation deliverable (Principle VII)

This change is not user-facing, so the user-guide obligation is met by architectural documentation:

- `docs/development/architecture.md` — the tier structure, where a feature lives, how to add a
  floating surface, how to add a capability, and the read/write asymmetry with its rationale.

Verified by the existing docs check in CI (currently passing in 6s).

## Definition of done

- [ ] All 20 steps merged, each green in its own commit (SC-009)
- [ ] SC-001 through SC-010 checked by the methods above
- [ ] `git diff main...HEAD -- crates/*/tests/` shows no modified assertion (FR-027)
- [ ] M1 and M2 performed and recorded
- [ ] Three new guard tests present and failing when their invariant is violated
- [ ] `docs/development/architecture.md` written and CI-verified
- [ ] Green on Linux, macOS and Windows (SC-006)
