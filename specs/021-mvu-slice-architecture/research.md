# Phase 0 Research: Feature-Module MVU Architecture

**Feature**: 021 | **Date**: 2026-08-07 | **Plan**: [plan.md](./plan.md)

All figures below were measured against `main` at `44b9fd1` on 2026-08-07. The spec's own baseline
table has drifted three times in ten days; treat these as a snapshot and re-measure before acting
on any one number.

## 1. The monolith, measured by concern

Before deciding where anything should go, the 130 message variants were partitioned by the feature
they serve. Every variant is assigned exactly once; the total is 130.

| Concern | Variants | Share | Destination |
|---|---:|---:|---|
| Session / terminal | 33 | 25% | `features/session.rs` + reducer module |
| Project / workspace | 22 | 17% | `features/project.rs` + reducer module |
| **Worktree creation form** | **22** | **17%** | **`features/worktree_form.rs` — nested unit** |
| Worktree (list, menu, delete, rename) | 13 | 10% | `features/worktree.rs` + reducer module |
| Sidebar | 9 | 7% | `features/sidebar.rs` + reducer module |
| Daemon / connection | 9 | 7% | `features/connection.rs` + reducer module — see §4 |
| Settings | 7 | 5% | `features/settings.rs` + reducer module |
| Miscellaneous (copy, no-op, diagnostics, logout survival) | 5 | 4% | shell |
| Window / input (cursor, resize, focus) | 3 | 2% | `shell/subscriptions.rs` |
| Theme / appearance | 3 | 2% | `features/settings.rs` |
| Overlay lifecycle | 2 | 2% | `overlay/registry.rs` |
| Notifications | 2 | 2% | `features/notifications.rs` |

**Decision**: partition by this table.
**Rationale**: it is derived from the existing message names rather than imposed, so it matches the
boundaries the codebase already implies. No variant needed a judgement call except the five
miscellaneous ones, which are genuinely shell concerns.
**Alternatives considered**: partitioning by `State` field (rejected — 37 fields do not map cleanly
onto 130 variants, and several fields serve two features); partitioning by view module (rejected —
`ui/` is organized by *surface*, not by feature, so `ui/sidebar.rs` renders three features' data).

## 2. There are two reducers, and the larger one is in the shell

The spec's Tier 3 addresses "the single long reducer". Measurement contradicts that premise:

| Reducer | Location | Lines | Holds |
|---|---|---:|---|
| `State::update` | `app.rs:1165–1942` | 778 | Pure state transitions |
| `update_inner` | `main.rs:775–2028` | **1,253** | The same features' effectful arms — tasks, I/O, daemon calls |

Both match on the same `Message` enum. A feature's handling is split across them by *purity*, not
by feature: `SettingsSaved` mutates the draft in one and writes the file in the other.

**Decision**: treat Tier 3 and the shell split as one pass over each feature boundary, not two
independent efforts. For each feature, its pure arms and its effectful arms move in the same step,
into `features/<name>.rs` and the relevant `shell/<system>.rs` respectively.
**Rationale**: splitting only `app.rs::update` would leave 1,253 lines untouched and make SC-003's
shell target unreachable. Worse, it would fix a feature's boundary in one file while leaving the
other free to drift.
**Consequence for the spec**: this does not contradict any requirement — FR-004a says "the single
long reducer", which is now read as "the reducer, wherever its arms live". FR-019a's
split-by-external-system still governs *where the effectful arms land*. **Resolved**: FR-004a was
amended on 2026-08-07 to read "wherever its arms live" and to require a feature's pure and effectful
arms to land on the same feature boundary. The spec now states what this section discovered.
**Alternatives considered**: unify the two reducers first, then split once (rejected — a temporary
2,031-line function is a worse intermediate state than either endpoint, and violates FR-028's
requirement that every step be a shippable improvement).

## 3. `main.rs` is 25% test code

Lines 2715–3567 are an inline `#[cfg(test)]` module: 851 lines, leaving a 2,715-line production
body. `app.rs` has no inline test module; its coverage is external, across 71 files in `tests/`.

**Decision**: inline tests move with the code they cover. When `update_inner`'s daemon arms move to
`shell/daemon_sync.rs`, the tests covering them move too.
**Rationale**: FR-027 forbids deleting or rewriting assertions, and SC-003 measures whole files. The
only way to satisfy both is relocation. This also means SC-003's shell target is less demanding
than it first appears — roughly 2,715 production lines to distribute, not 3,567.
**Alternatives considered**: converting inline tests to `tests/` integration tests (rejected — many
exercise private items and would need visibility widened purely to relocate them, which is a
behavior-adjacent change FR-027's spirit resists).

## 4. An eighth concern the spec does not name

FR-001 enumerates seven features: worktree, session/terminal, project/workspace, sidebar, settings,
notifications, overlays. Measurement finds an eighth with 9 message variants — daemon connection
lifecycle (`DaemonConnected`, `DaemonEvent`, `DaemonGridFrame`, `DaemonDisconnected`,
`DaemonConnectFailed`, `DaemonVersionMismatch`, `DaemonBuildMismatch`,
`ConnectionTakeoverRequested`, `ConnectionRestartServiceRequested`) plus its own `State` fields and
a `connection_status` projection in `main.rs:2106`.

**Decision**: treat connection as a feature module (`features/connection.rs`) with a reducer module,
and record it as an addition to FR-001's list. **Done**: FR-001 now names daemon connection, which
brings SC-004a and SC-010 — both scoped "for every feature named in FR-001" — over it automatically.
**Rationale**: it satisfies every test FR-001 applies — it has its own data, its own operations and a
maintainer should be able to name one module for it. Leaving it unassigned would strand 9 variants
in the root reducer and quietly violate FR-002.
**Note**: this does *not* reopen Q1. The daemon **process** stays out of scope; what is in scope is
the *client's* handling of its connection, which has always been client code.
**Alternatives considered**: folding connection into session/terminal (rejected — it would push that
feature to 42 variants, the largest by far, and connection state outlives any individual session);
treating it as shell (rejected — it has genuine decision logic, e.g. version-mismatch and takeover
policy, which FR-002 keeps out of the root and Principle I keeps out of untested glue).

## 5. Per-feature nesting record (FR-003, SC-004a)

FR-003 permits a nested state/message/reducer unit **only** where a feature "is opened, edited and
dismissed as a unit whose intermediate state no other feature reads". Each feature was tested
against that bar by grepping for external readers of its state.

| Feature | Independent lifecycle? | External readers of intermediate state | Verdict |
|---|---|---|---|
| **Worktree creation form** | **Yes** — opened, multi-step edited (type → ticket/name → branch source → typeahead → conflict resolution), submitted or cancelled | `ui/worktree_form.rs` (its own view) and the generic overlay snapshot only. No other feature reads `worktree_form`. | **Nested unit** |
| Settings draft | Yes — opened, edited, saved or cancelled | `ui/settings_form.rs` (its own view) and the generic snapshot only | **Qualifies, deferred** — see below |
| Worktree (list/menu/delete/rename) | No — the worktree list is read by sidebar, session start, and the switcher | sidebar, session, project | Feature module + reducer module |
| Session / terminal | No — sessions are read by sidebar, toolbar, terminal pane, daemon sync | 4+ features | Feature module + reducer module |
| Project / workspace | No — the workspace is read by nearly everything | all | Feature module + reducer module |
| Sidebar | No — `sidebar_entries()` reads worktrees *and* sessions to build its rows | reads across two features | Feature module + reducer module |
| Settings (applied, not draft) | No — theme and scrollback are read by the shell and terminal | shell, terminal | Feature module + reducer module |
| Notifications | No — pushed to by every feature | all | Feature module + reducer module |
| Connection | No — status is read by the toolbar and terminal pane | 2 features | Feature module + reducer module |
| Overlays | N/A — Tier 2's registry, not a feature | — | Overlay registry |

**Decision**: exactly **one** nested unit — the worktree creation form.

**Rationale**: it is the only feature that both clears the lifecycle bar and is large enough for
nesting to pay. Its 22 variants are 17% of the entire message enum, all of them prefixed
`AddWorktree*`/`WorktreeCreate*`, and its state (`WorktreeForm`, `WorktreeFormStatus`,
`BranchSource`, `ResolutionState`, `app.rs:140–326`) is genuinely a multi-step wizard. Nesting
removes 22 variants from the root vocabulary in one move.

**Why settings is deferred rather than nested**: it clears the lifecycle bar on the same evidence,
but it is 7 variants over a flat 4-field draft with no multi-step flow. Nesting would add a message
wrapper and a routing arm to save 7 root variants — cost roughly equal to benefit. FR-004b
explicitly permits concluding that a reducer module suffices, and this is that conclusion, recorded
with its evidence rather than left implicit. Should the settings surface grow a second page or a
validation flow, the bar is already met and promotion is a local change.

**SC-004a is satisfied by this table**: every feature named in FR-001 (plus connection, §4) has a
recorded verdict and the evidence behind it. The count of nested units is one — a valid outcome
under FR-004b, which permits any count including zero.

## 6. Migration sequence (FR-028, SC-009)

Twenty steps. Each leaves the application buildable, runnable and green, and each is its own commit
so SC-009 is verifiable from history rather than only from the endpoint. Tiers 1, 2 and the shell
split contain no Tier 3 work, satisfying SC-004b.

### Tier 1 — type-first extraction (steps 1–7)

Each step moves a type cluster with its helper functions out of `app.rs` into `features/`, and
`pub use`s it back from `app.rs` so no call site changes in the same commit. Purely mechanical;
the existing suite is the entire safety net.

| # | Moves | From | ~Lines |
|---|---|---|---:|
| 1 | `WorktreeForm`, `WorktreeFormStatus`, `BranchSource`, `ResolutionState` + impls | `app.rs:86–326` | 240 |
| 2 | `SidebarEntry`, `DefaultNode`, `WorktreeNode`, `TagFilter`, `matches_filters`, `worktree_location_label` | `app.rs:372–456` | 85 |
| 3 | `ProjectMenu`, `clamp_menu_anchor`, `SwitcherEntry`, `RenameDraft`, `SelectKind` | `app.rs:327–371, 457–497` | 85 |
| 4 | `SettingsDraft` | `app.rs:469–484` | 16 |
| 5 | `WorktreeRenameDraft` + `State`'s worktree helpers (`worktree_tree`, `filtered_worktree_tree`, `visible_worktrees`, `worktree_tags`, `worktree_display_name`) | `app.rs:498–510, 2156–2245` | 105 |
| 6 | `NoticeLevel`, `Notification` — reconciled against the existing `micold_core::notify` queue | `app.rs:923–944` | 22 |
| 7 | `State`'s session helpers (`sessions_in_worktree`, `active_sessions`, `switch_active`, `record_foreground`, `restore_after_activation`, `restore_foreground`, `arm_notice`, `note_background_restart`, `session_mut`) | `app.rs:2014–2155` | 142 |

After step 7 the re-exports are removed and call sites import from `features::*` — one mechanical
commit. Expected: `app.rs` down to roughly 1,700 lines (types out, both reducers still in).

### Tier 2 — overlay registry (steps 8–11)

| # | Step | Ships |
|---|---|---|
| 8 | Introduce the uniform floating-surface type and registry, built on feature 017's `Layer`/`Surface`/`Trigger` (FR-014). `Overlay`/`ClosingOverlay` remain, deriving into it. | Both representations coexist; every existing overlay test green |
| 9 | Migrate the 7 ad-hoc popovers (`help_menu_open`, `project_switcher_open`, `sidebar_filter_open`, `worktree_menu_open`, `project_menu_open`, `terminal_context_menu`, `session_menu_open`) onto the registry, preserving the popover-before-modal dismissal priority | 7 loose fields gone; FR-012, FR-013 held by existing tests |
| 10 | Collapse the six central match sites onto generic dispatch: the overlay enum, `on_escape`, `ui/mod.rs`'s keyboard-subscription mirror, the view match, `capture_overlay`, and the closing-snapshot enum | SC-001's "zero central match statements" reached |
| 11 | Delete `Overlay` and `ClosingOverlay`; add the registration guard test (FR-010) | 19 variants gone |

Step 8 is the risk concentration point: FR-011's exit-animation snapshot, including reopening a
surface mid-animation, must survive. `overlay_transition_identity.rs` and
`overlay_dismissal_delta.rs` are the arbiters and neither may be modified.

### Shell split — orthogonal to the tiers (steps 12–16)

| # | Step | Ships |
|---|---|---|
| 12 | `shell/capabilities.rs` — one struct assembled at boot, replacing the 9 inline construction sites (`main.rs:523, 532, 649, 1295, 1310, 1330, 1924, 2604, 2709`) | FR-018 |
| 13 | Declare the three missing capabilities in core — clipboard, OS theme probe, env-include resolution — each with a fake | FR-015, FR-019 |
| 14 | Split `main.rs` by external system into `shell/{startup,persist,daemon_sync,subscriptions,env_include,os_theme}.rs`, inline tests moving with their subjects | FR-019a |
| 15 | Move `update_inner`'s effectful arms to the system they address | The 1,253-line reducer starts shrinking |
| 16 | Guard test: non-shell code names no concrete implementation | FR-017 |

**SC-004b checkpoint**: after step 16, Tiers 1 and 2 and the shell split are all merged with zero
Tier 3 work. Demonstrating green here is the criterion.

### Tier 3 — reducer split and outcomes (steps 17–20)

| # | Step | Ships |
|---|---|---|
| 17 | Split the remaining `State::update` into per-feature reducer modules; root retains routing only | FR-004a, FR-002 |
| 18 | Promote the worktree creation form to a nested unit with its own message type (§5) | FR-003 — 22 variants leave the root enum |
| 19 | Introduce `Outcome` and convert the worktree-delete path to return session and overlay consequences instead of writing them | FR-021, FR-023 |
| 20 | Guard test: no feature reducer writes another feature's data, naming the offending path on failure | FR-024a, SC-007 |

**Decision**: this ordering.
**Rationale**: it follows the spec's "value per unit of architectural risk". Steps 1–7 are the
largest reduction for the least commitment and are individually revertible. Tier 3 is last because
§5's nesting evidence is only trustworthy once the types are separated and the boundaries visible.
**Alternatives considered**: feature-at-a-time vertical slices, taking one feature through all
tiers before starting the next (rejected — Tier 2's registry is cross-cutting and would have to be
built during the first feature's slice and retrofitted to the rest, and it would make SC-004b
undemonstrable since every commit would mix tiers).

## 7. Capability shapes (FR-015, FR-016)

Seven ports exist. Three are missing, and one of the three does not fit the existing trait shape —
which is why FR-015a was added to the spec on 2026-08-07, sanctioning an effect-request form where
the framework precludes a synchronous port.

| Capability | Status | Shape |
|---|---|---|
| `Git`, `ProjectStore`, `SettingsStore`, `FolderScanner`, `TerminalBackend`, `TerminalHandle`, `AiCliProvider` | Exist | Unchanged; `FakeGit` already ships in `core/git.rs:467` as precedent |
| Env-include resolution | Missing | Plain trait — `resolve(&self, cwd: &Path) -> EnvIncludeSnapshot`. Logic already isolated at `main.rs:397–450`. |
| OS theme probe | Missing | Plain trait — `detect(&self) -> Result<SystemScheme, ()>`. Wraps the single `dark_light` call at `main.rs:2678`; this is the codebase's only direct OS branch, so porting it also serves Principle VI. |
| Clipboard | Missing, **and awkward** | Not a plain trait — see below |

**The clipboard wrinkle.** All three real clipboard operations (`main.rs:1840, 1847, 1856`) go
through `iced::clipboard::write`/`read`, which return an `iced::Task`, not a value. A synchronous
`trait Clipboard { fn write(&self, s: String); }` cannot be implemented over them without blocking.

**Decision**: model clipboard as a *request* in the feature's outcome vocabulary, interpreted by
the shell, rather than as a called port.
**Rationale**: it matches how the effect actually works in this framework, and it reuses the
outcome channel Tier 3 introduces anyway rather than inventing a second mechanism. A test asserts
the feature emits the clipboard request; the shell's translation to `iced::clipboard::write` is
thin glue covered by Principle I's GUI-wiring exception.
**Alternatives considered**: a channel-based async trait (rejected — real machinery for three call
sites); leaving clipboard unported (rejected — FR-015 names it explicitly).

## 8. Where FR-017 already holds

`app.rs` constructs **no** concrete implementation. Every one of the 9 construction sites is in
`main.rs`, i.e. already in the shell. So FR-017's "non-shell code depends only on declared
capabilities" is *already true* of the pure reducer, and the real gap is FR-018: the shell
constructs them inline at the point of use — four of the nine inside `update_inner` — rather than
assembling once at startup.

**Decision**: scope step 12 to assembly, and write the FR-017 guard test (step 16) as a
regression lock on a property that already holds rather than as a migration.
**Rationale**: honest scoping. Reporting FR-017 as new work would overstate the increment; the
spec's own assumption already says the service layer is "an extension, not a greenfield".

## 9. Open risks

- **Step 8 is the single riskiest change.** The exit-animation snapshot (FR-011) is subtle: the app
  renders a *copy* of an overlay whose live state has been cleared. Mitigation: land steps 8–11 as
  four separate commits so a bisect lands on one of them, not on a monolithic overlay rewrite.
- ~~**SC-003's 500-line targets are aggressive.**~~ **Resolved 2026-08-07**: SC-003 was clarified so
  that FR-005 governs and the 500-line figure is indicative, not a gate. A file containing exactly
  one feature and no longer among the largest satisfies the criterion at any length, and splitting a
  coherent module to cross a threshold is explicitly forbidden. The estimates below remain ±20%, but
  a miss is now a progress signal rather than a failure.
- **No cross-platform verification until CI.** The OS-theme capability is the only platform-branching
  code touched. Its fake makes it testable everywhere, but the real path still needs all three CI
  platforms green (SC-006).
