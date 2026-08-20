# Baseline: Feature-Module MVU Architecture

**Feature**: 021 | **Task**: T002 | **Measured**: 2026-08-08
**Commit**: `88ee295` (`docs(018): close the tree-row visual passes on measurement, and file what they showed`)

Measured, not copied. The spec's figures have moved four times in ten days, so every later
measurement compares against *this* table, taken at the moment work started.

## The two target files

| File | Lines | Note |
|---|---:|---|
| `crates/micold-client/src/main.rs` | **3,567** | Largest in the repository. 851 lines (2715–3567) are an inline `#[cfg(test)]` module, so the production body is 2,715 |
| `crates/micold-client/src/app.rs` | **2,434** | Second largest. No inline test module; coverage is external |

## Ten largest source files

| Lines | File |
|---:|---|
| 3,567 | `crates/micold-client/src/main.rs` |
| 2,434 | `crates/micold-client/src/app.rs` |
| 1,483 | `crates/micold-daemon/src/server.rs` |
| 1,348 | `crates/micold-client/src/ui/material/terminal_pane.rs` |
| 1,317 | `crates/micold-daemon/src/state.rs` |
| 1,220 | `crates/micold-client/tests/app_state.rs` |
| 1,185 | `crates/micold-daemon/tests/mutation_semantics.rs` |
| 1,151 | `crates/micold-client/src/ui/material/animation.rs` |
| 1,020 | `crates/micold-client/tests/support/layout.rs` |
| 947 | `crates/micold-core/src/worktree.rs` |

**SC-003 reads against this table**: success is `main.rs` and `app.rs` no longer appearing near the
top, with `server.rs` (1,483) becoming the largest source file. Per the 2026-08-07 clarification,
FR-005 governs — a file holding exactly one feature satisfies the criterion at any length, and the
~500-line figure is a progress signal, not a gate.

## Structural counts

| Concern | Count | Location |
|---|---:|---|
| `State` fields | **37** | `app.rs` |
| `Message` variants | **130** | `app.rs` |
| `Overlay` variants | **10** | `app.rs` (one is `None` — 9 real surfaces) |
| `ClosingOverlay` variants | **9** | `app.rs` |
| Ad-hoc popover state fields | **7** | `app.rs` |
| Client integration-test files | **73** | `crates/micold-client/tests/` |
| Service ports in core | **7** | `micold-core` |

## Reducers

Two, over the same `Message` enum, split by purity rather than by feature (research.md §2).

| Reducer | Location | Lines |
|---|---|---:|
| `update_inner` | `main.rs:775–2028` | **1,253** |
| `State::update` | `app.rs:1165–1942` | **778** |

## Drift history

| Date | `main.rs` | `app.rs` | `State` | `Message` |
|---|---:|---:|---:|---:|
| 2026-07-28 | 2,914 | 2,245 | 36 | 124 |
| 2026-08-06 | 3,467 | 2,358 | 36 | 128 |
| 2026-08-07 | 3,567 | 2,434 | 37 | 130 |
| **2026-08-08 (this)** | **3,567** | **2,434** | **37** | **130** |

Flat over the last day — the commits since were docs and tests. The 22% and 8% growth over the
preceding ten days is the argument for SC-003's absolute target, and it has not reversed.

## Progress log

Re-measure at each phase checkpoint and append a row.

| Checkpoint | `main.rs` | `app.rs` | Notes |
|---|---:|---:|---|
| Baseline (T002) | 3,567 | 2,434 | Start of work |
| T015 (worktree form) | 3,567 | 2,198 | −236; first extraction |
| T016 (sidebar) | 3,567 | 2,122 | −76; two of eight features out |
| T017 (project) | 3,567 | 2,073 | −49; `SelectKind` left behind, see T021 |
| T018 (settings) | 3,567 | 2,063 | −10; validation stays in `main.rs` until Tier 3 |
| T019 (worktree) | 3,567 | 1,893 | −170; projections split worktree/sidebar by feature |
| T020 (notifications) | 3,567 | 1,876 | −17; a dead duplicate `Notification` deleted |
| T021 (session) | 3,567 | 1,727 | −149; at the phase checkpoint's ~1,700 target |
| T022 (connection) | 3,561 | 1,727 | first `main.rs` movement; source was `ui/mod.rs` |
| T023 (re-exports gone) | 3,561 | 1,689 | −38; every call site imports from `features::*` |
| **T058 (SC-004b checkpoint)** | **1,672** | **1,872** | Tiers 1–2 + shell split merged; `app.rs` grew, see below |
| **T078 (final)** | **1,675** | **1,334** | Phases 6–7; `app.rs` −538 as the reducer arms became routing |

## T078 — final re-measurement (2026-08-20)

Against the table at the top of this file, measured on the finished tree. **Comment lines are
stripped before counting fields and variants**, which the baseline did not say it did; the shapes
being counted are unambiguous either way.

### The two target files

| File | Baseline | Now | Change |
|---|---:|---:|---|
| `crates/micold-client/src/main.rs` | 3,567 | **1,675** | **−53%**, and it no longer holds an inline test module |
| `crates/micold-client/src/app.rs` | 2,434 | **1,334** | **−45%** |

Ranked among all source files, `main.rs` went **1st → 4th** and `app.rs` **2nd → 8th**.

**SC-003 as this file framed it is half met, and the half that is not is `main.rs`.** The criterion
reads "neither remains among the largest, with `server.rs` (1,483) becoming the largest source
file". `server.rs` is now 1,582 and **5th**; the largest is `tests/app_state.rs` at 2,146, and
`shell/daemon_sync.rs` at 2,118 is the largest *production* file. `main.rs` is still above
`server.rs`. T069 records the substance: FR-005 governs, neither file holds more than one feature,
and `root_is_routing_only.rs` pins that at an exact 0 rather than measuring it.

### Ten largest source files

| Lines | File | Then |
|---:|---|---|
| 2,146 | `crates/micold-client/tests/app_state.rs` | 1,220 (6th) |
| 2,118 | `crates/micold-client/src/shell/daemon_sync.rs` | did not exist |
| 1,745 | `crates/micold-client/src/ui/material/terminal_pane.rs` | 1,348 (4th) |
| 1,675 | `crates/micold-client/src/main.rs` | 3,567 (1st) |
| 1,582 | `crates/micold-daemon/src/server.rs` | 1,483 (3rd) |
| 1,466 | `crates/micold-daemon/src/state.rs` | 1,317 (5th) |
| 1,399 | `crates/micold-daemon/tests/mutation_semantics.rs` | 1,185 (7th) |
| 1,334 | `crates/micold-client/src/app.rs` | 2,434 (2nd) |
| 1,324 | `crates/micold-client/tests/feature_write_isolation.rs` | did not exist |
| 1,231 | `crates/micold-client/src/ui/terminal.rs` | — |

`shell/daemon_sync.rs` is where `main.rs`'s I/O went; it is the shell, not a feature, and is out of
FR-005's scope for the reason T069 records. The daemon's two files grew ~7% on their own — Q1 put
the daemon out of scope and it kept moving, which is the same drift the baseline was taken to
measure.

### Structural counts

| Concern | Baseline | Now | Note |
|---|---:|---:|---|
| `State` fields | 37 | **41** | +4; other features shipped meanwhile |
| `Message` variants | 130 | **116** | **−14**; T064 nested the form's vocabulary |
| `Overlay` variants | 10 | **0** | the enum is gone (T037) |
| `ClosingOverlay` variants | 9 | **0** | gone (T036) — one `Closing` type, no per-surface list |
| Ad-hoc popover state fields | 7 | 8 | +1 (`about_open`, T037); they are no longer *ad hoc* — each is read by one registration |
| Client integration-test files | 73 | **104** | +31 |
| Service ports in core | 7 | **10** | +3 |
| Registered floating surfaces | — | **16** | one line each, in one file |
| Feature modules | — | **10** | one registration line each |
| Cross-feature writes (`ALLOWED`) | — | **0** | 43 at its peak |
| Root decision arms | 93 | **0** | exact, and pinned (T063) |

### Reducers

| Reducer | Baseline | Now | Change |
|---|---:|---:|---|
| `update_inner` (`main.rs`) | 1,253 | **186** | **−85%** |
| `State::update` (`app.rs`) | 778 | **285** | **−63%**, and every one of its arms is routing |

The two reducers together went from **2,031 lines to 471**. `State::update` is now one arm per
message and nothing else; `update_inner` is the shell's I/O half.

### Drift history

| Date | `main.rs` | `app.rs` | `State` | `Message` |
|---|---:|---:|---:|---:|
| 2026-07-28 | 2,914 | 2,245 | 36 | 124 |
| 2026-08-06 | 3,467 | 2,358 | 36 | 128 |
| 2026-08-07 | 3,567 | 2,434 | 37 | 130 |
| 2026-08-08 (baseline) | 3,567 | 2,434 | 37 | 130 |
| **2026-08-20 (T078)** | **1,675** | **1,334** | **41** | **116** |

The growth the baseline was taken to argue about has reversed: 22% and 8% up over the ten days
before, 53% and 45% down over the twelve days since.

## T025 — SC-010 review at the Tier 1 checkpoint

SC-010: *"A maintainer can answer 'where does this feature live?' by naming a single module, for
every feature named in FR-001."* FR-001 names nine. Eight pass; one does not, by design.

| FR-001 feature | Single module? | Where |
|---|---|---|
| worktree | yes | `features/worktree.rs` |
| session / terminal | yes | `features/session.rs` |
| project / workspace | yes | `features/project.rs` |
| sidebar | yes | `features/sidebar.rs` |
| settings | **partial** | `features/settings.rs`; validation still in `main.rs`'s `SettingsSaved` arm |
| notifications | yes | `features/notifications.rs` |
| daemon connection | yes | `features/connection.rs` |
| overlays | **no** | still enumerated in `app.rs` — Tier 2 (T026–T040) is what gives it a module |

`features/worktree_form.rs` is a ninth module beyond FR-001's list: the creation form is the one
feature whose intermediate state nothing else reads (research.md §5), so it is a module in its own
right rather than part of `features/worktree.rs`.

**So SC-010 is not yet met, and Tier 1 was never going to meet it.** Overlays are Tier 2's subject
and settings' validation is Tier 3's; both are named in the criterion, so the criterion closes at
the end of the feature rather than at this checkpoint. Recording it as passing here would have
required either ignoring `overlays` in FR-001's list or calling `app.rs` a module for it.

### Line count against the baseline

`app.rs` **2,434 → 1,689**, a 31% reduction, against a Phase 3 checkpoint expectation of "roughly
1,700 lines (types out, both reducers still in)". Both reducers are indeed still in: `State::update`
in `app.rs` and `update_inner` in `main.rs` are untouched, and together they are most of what
remains.

`main.rs` 3,567 → 3,561. Essentially flat, as expected — Tier 1 does not touch the shell, and the
six lines are `connection_status` losing its inlined precedence to `features/connection.rs`.

### Three counts worth carrying forward

- **Visibility widenings**: 4 (`rematch_branches`, `reset_branch_search`, `worktree_tags`,
  `session_mut`), all private → `pub(crate)`, all pointed at T062.
- **Dead code found and removed**: 1 (`app::Notification`, never constructed).
- **Task mis-groupings corrected**: 3 (`SelectKind` T017→T021; the three sidebar projections
  T019→`features/sidebar.rs`; `sidebar_entries`, which T016 left behind).

## T058 — the SC-004b checkpoint

SC-004b: *"Tiers 1, 2 and the shell split are each demonstrated green with no part of Tier 3
merged."* Two claims, and both are checked below rather than asserted.

**Commit**: `e91d468` on `main`, 2026-08-18. **CI run**: [32175454220][run], all eight checks green
— `classify change`, `docs check`, `fmt + clippy`, `assertion freeze (advisory)`, the three
`build + test` matrix jobs, `ci complete`.

[run]: https://github.com/jaroslawherod/micold-ai-ide/actions/runs/32175454220

### The green, and what "all three platforms" actually covers

| Platform | What CI runs there | Suites | Tests | Result |
|---|---|---:|---:|---|
| ubuntu-latest | core `--all-targets`, workspace build, the 11 all-platform gates, **`cargo test --workspace`** | 272 | 2,625 | pass |
| macos-latest | core `--all-targets`, workspace build, the 11 all-platform gates | 72 | 709 | pass |
| windows-latest | same as macOS | 72 | 706 | pass |

**T058's own wording overstates what this demonstrates, and the difference is worth writing down
rather than rounding off.** The task says "full suite green on all three platforms". CI runs the
full workspace suite on **Linux only** — `.github/workflows/ci.yml`'s "Test (full workspace)" step
carries `if: runner.os == 'Linux'`, because the client and daemon suites need the iced system
dependencies the Linux job installs. macOS and Windows get the render-free core, a workspace
*build* (so the GUI binary is proven to compile everywhere), and the eleven client gates that read
source text or a reducer and open no window.

So what is demonstrated here is: **the full suite green on Linux, and everything that can run
without a window green on all three.** That is enough for SC-004b, whose subject is the tiers, and
every structural guard this feature added is in the Linux run. It is *not* enough for **SC-006**
— *"The complete pre-existing test suite passes … on all three supported platforms"* — which
**T077 owns and cannot satisfy against this workflow as configured**. T077 will have to either run
the client suite on macOS and Windows (which is a CI change, not a code change) or restate SC-006
against what the matrix covers. Flagged here so it is a decision at T077 rather than a discovery.

### Zero Tier 3 merged — checked step by step

research.md §6 defines Tier 3 as steps 17–20. Each was checked against the tree at `e91d468`:

| # | Tier 3 step | Merged? | How that was checked |
|---|---|---|---|
| 17 | Per-feature reducer modules, root retains routing | **no** | No `reducers/` module exists. `State::update` is still one function (`app.rs:902`, 834 lines) and `update_inner` is still one function (`main.rs:450`) |
| 18 | Worktree form as a nested unit with its own message type | **no** | No `Message` enum anywhere under `features/`; the form's 14 variants are still in the root enum |
| 19 | `Outcome` + worktree-delete returning consequences | **partial, and deliberately** | `features/mod.rs:33` holds `Outcome` with **exactly one** variant, `ClipboardWrite` — see below |
| 20 | Guard: no feature reducer writes another feature's data | **no** | Feature 021's FR-024a/SC-007 appear in no test. (Both identifiers do occur in `crates/`, belonging to features 017 and 018 — different features' numbering, not this one's) |

**The one thing that looks like Tier 3 and is not.** `Outcome` exists before its own task (T065,
Phase 6) because T045 and T056 sit in Phase 5 and ask for it. T045's record argues the distinction
and it holds up on inspection: FR-021's `Outcome` is a feature reducer handing a consequence to the
**root** to route to another feature, and none of that is here — there is no root interpreter, no
reducer split, and no feature learning anything about another. FR-015a borrows the same vocabulary
for a different job and names the **shell** as interpreter. One variant, one emitter
(`selection::copy_request`), shell-interpreted. T065 extends it; its text needs "introduce" → "extend".

### What is merged, in one place

- **Tier 1** — ten feature modules under `features/`: `connection`, `help`, `notifications`,
  `project`, `session`, `settings`, `sidebar`, `worktree`, `worktree_form`, plus `mod`
- **Tier 2** — `overlay/{mod,registry}.rs`. The `Overlay` (10) and `ClosingOverlay` (9) enums are
  **gone from `app.rs` entirely**; `app.rs:538` documents the single field that is their last remnant
- **Shell split** — eleven modules under `shell/`: `capabilities`, `clipboard`, `daemon_sync`,
  `env_include`, `os_theme`, `persist`, `service_control`, `startup`, `subscriptions`, `workspace`,
  plus `mod`

### Structural counts re-measured

| Concern | Baseline (T002) | At T058 | Δ |
|---|---:|---:|---|
| `main.rs` total / production body | 3,567 / 2,715 | **1,672 / 861** | −53% / **−68%** |
| `app.rs` (no inline tests) | 2,434 | **1,872** | −23% |
| `update_inner` | 1,253 | **185** | **−85%** |
| `State::update` | 778 | **834** | **+7%** |
| `State` fields | 37 | 41 | +4 |
| `Message` variants | 130 | 137 | +7 |
| `Overlay` / `ClosingOverlay` variants | 10 / 9 | **0 / 0** | enums deleted |
| Service ports | 7 | **10** | `EnvIncludeResolver`, `OsThemeProbe`, `FolderBrowser` (split from `FolderScanner` at T048) |

**Two of these numbers went the wrong way and neither is a surprise.** `State::update` grew because
Phase 5 moved `update_inner`'s *pure* arms down into it while its effectful arms went to `shell/` —
which is why one reducer shrank 85% and the other did not shrink at all. That 834-line function is
precisely Tier 3 step 17's subject, so the checkpoint's job is to record it as the remaining work,
not to have fixed it. `Message` and `State` grew because the feature is not the only thing landing
on `main`; other features' work continues alongside.

### SC-003 is not met at this checkpoint, and was not due to be

SC-003 reads against the ten-largest table above: success is `main.rs` and `app.rs` no longer near
the top. Current top three: `shell/daemon_sync.rs` **1,960** (1,517 production + an inline test
module), `app.rs` **1,872**, `ui/material/terminal_pane.rs` **1,745**. `main.rs` has left the top
three; `app.rs` has not, and it will not until step 17 takes the 834-line reducer out of it.
`daemon_sync.rs` is the largest file in the repository, which is the shell split concentrating one
external system in one module — FR-005 governs (a file holding exactly one feature satisfies the
criterion at any length), but it is worth watching rather than assuming.

### The caveat on the Windows green

**A green Windows job is weaker evidence here than it looks.** `crates/micold-core/tests/env_include_resolve.rs`
flaked intermittently on `windows-latest` throughout Phases 4–5; it is diagnosed and fixed in
[BUG-004](../011-env-include-script/bugs/BUG-004.md), which landed one commit before this
checkpoint. That job has now passed five consecutive runs — but three of those were *before* the
fix, so the streak is not evidence the fix worked, and BUG-004 says so in its own words. A red
Windows job at T077 should be read against that report before it is read as a regression.

**Also unresolved and pointed at T077**: `crates/micold-core/tests/typeahead_budget.rs` fails
load-dependently on this machine, filed as
[021-branch-typeahead-search BUG-003](../021-branch-typeahead-search/bugs/BUG-003.md). It measures a
debug build against a release budget, so its margin is ≈ 0. It has not failed in CI — but it is a
*core* test, so unlike the client suite it runs on **all three** matrix jobs, and a loaded runner
can fail it anywhere. That makes it the second known load-sensitive test standing between this
checkpoint and T077's three-platform green.
