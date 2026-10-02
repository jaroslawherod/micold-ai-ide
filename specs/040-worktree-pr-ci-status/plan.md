# Implementation Plan: Pull Request and Check Status for Each Worktree

**Branch**: `feat/worktree-pr-ci-status` | **Date**: 2026-10-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/040-worktree-pr-ci-status/spec.md`

## Summary

Each worktree row of the sidebar gains a small indicator of its branch's pull request — open, draft,
merged or closed — and, for an open or draft one, one combined check status. The tooltip names the
pull request, the row menu opens it in the browser, and a merged pull request whose branch holds no
newer work marks the worktree as removable. All of it is off until the user turns on one switch in
Settings.

The design rests on five decisions:

1. **One GraphQL request per reading, through 034's `gh` runner**
   ([R1](./research.md#r1--how-github-is-reached-the-existing-gh-runner-one-graphql-request-per-reading),
   [R2](./research.md#r2--the-query-aliased-pullrequestsheadrefname-connections-check-counts-by-state)).
   Up to 50 branches are aliased into one query; checks arrive as counts by state, so the answer
   does not grow with the number of checks. 12 requests an hour against SC-006's 30. No new crate.
2. **The reading runs in the client, on the host; the daemon answers two local git questions**
   ([R5](./research.md#r5--where-the-reading-runs-in-the-client-on-the-host),
   [R11](./research.md#r11--no-commits-beyond-the-merged-pull-request-a-read-only-daemon-rpc)): the
   remotes (034's `RemoteList`, unchanged) and, new, whether a merged branch has later commits
   (`MergedBranchCheck`).
3. **Every rule is a pure function or a render-free reducer**: which pull request is the branch's
   ([R3](./research.md#r3--which-pull-request-is-the-branchs-select_pull_request)), the check
   reduction ([R4](./research.md#r4--the-combined-check-status-a-pure-reduction-over-the-counts)),
   the failure split ([R10](./research.md#r10--two-kinds-of-failure-cannot-be-read-at-all-and-a-passing-failure)),
   the rate-limit pause ([R9](./research.md#r9--the-request-limit-read-the-reset-time-from-the-response-headers)),
   the schedule ([R8](./research.md#r8--the-schedule-a-render-free-reducer-one-timer-subscribed-only-when-needed))
   and staleness ([R13](./research.md#r13--staleness-and-the-clock)).
4. **The switch is a service-owned setting, off by default**
   ([R12](./research.md#r12--the-switch-a-service-owned-setting-off-by-default)), so the daemon's
   existing `SettingsChanged` broadcast turns every window on or off at once.
5. **The indicator is a new shared component** in the row's trailing slot
   ([R14](./research.md#r14--the-indicator-a-shared-component-in-the-rows-trailing-slot)); a row
   without a pull request is unchanged.

Only the window that holds a project (feature 010) reads and shows its status, so readings need no
coordination between windows
([R6](./research.md#r6--several-windows-only-the-window-that-holds-the-project-reads)).

## Technical Context

**Language/Version**: Rust, edition 2021, workspace MSRV (unchanged)

**Primary Dependencies**: `iced` 0.14 (`time::every`, `Task::perform`), `serde`/`serde_json`,
`tokio` (`spawn_blocking`), `micold-core` modules `github` (`GhCli`, `GithubRepo`, `choose_remote`,
`locate_gh`, `classify`), `git`, `process::run_bounded`, `settings`, `protocol`. External runtime
tool: the GitHub CLI `gh` (user-installed, any 2.x; probed with 2.54.0). **No new crate.**

**Storage**: `settings.json` gains `pr_status_enabled` (additive, `#[serde(default)]` = false;
`SETTINGS_VERSION` unchanged). Pull request status is held in client memory only (FR-032).

**Testing**: `mise run test-core` for the render-free rules; `mise run gate` for the workspace;
reducer tests under `crates/micold-client/tests/`; a daemon RPC test under
`crates/micold-daemon/tests/`; source gates; layout gates; `quickstart.md` §B for render glue and
the real `gh`.

**Target Platform**: Linux, macOS, Windows desktop. No new `cfg` arm: `gh` is located and run by
034's code, the browser is opened by the existing `LinkOpener`.

**Project Type**: desktop application (three-crate Cargo workspace).

**Performance Goals**: a reading never runs on the update loop (SC-005); at most 1 request per
reading for up to 50 worktrees (SC-006); indicators within 10 s of opening a project (SC-001).

**Constraints**: nothing sent while the switch is off, without a GitHub remote, or for a project no
window shows (FR-026, FR-018); requests carry only `owner/name` and branch names (FR-031); titles
and addresses never stored or logged (FR-032); no error surface of any kind (FR-025).

**Scale/Scope**: one new core module (`pull_request`), three extended (`git`, `settings`,
`protocol`); one protocol bump (20 → 21) for one RPC and one settings field; one daemon arm; one
client feature module and one shell module; one shared component and eight `Icon` variants; one
Settings control; the sidebar row, tooltip and row menu extended; user guide and three docs pages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every decision lives in tested render-free code
  before its caller: `micold-core` (`status_query`, `split_response`, `parse_status`,
  `select_pull_request`, `reduce_checks`, `reading_failure`, `rate_limit_pause`, `is_stale`,
  `containment`, settings round trip), the client reducer `features/pr_status.rs` and the tooltip
  builder, the daemon arm. Parsing is tested against recorded `gh` output (the spec's carried
  constraint). The GUI exception is claimed only for `src/ui/` composition and the shell's task
  glue, covered by quickstart §B. `speckit.tdd.plan` derives `tdd/test-list.md`.
- [x] **II. Multi-Session Support**: PASS. No session state; the indicator belongs to the worktree
  row and session rows are untouched. Status is per shown project in one window's memory, dropped on
  project switch; nothing leaks between projects or windows (R6).
- [x] **III. Worktree Integration**: PASS. Nothing creates, changes or removes a worktree: the
  removal suggestion leads to the existing Delete action and its confirmation (FR-016). The new git
  reads are read-only and go through the `Git` trait in the daemon. The "Default" entry gets no
  indicator (FR-007).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. GitHub is contacted only after the user
  turns on a switch that says what is read, how often and what is sent (FR-029, FR-030), and only
  with the repository identity and branch names (FR-031). Offline, every existing function works and
  the rows simply show no indicator (FR-025). Status is never persisted.
- [x] **V. Rust + iced Stack**: PASS. Rust and iced only. Types narrow the states: `CheckStatus` is
  only present on an open or draft `PullRequestStatus`; the schedule is an enum (`Idle` /
  `Reading`), so two concurrent readings are unrepresentable; `PullRequestStatus` has no
  `Serialize`.
- [x] **VI. Cross-Platform Parity**: PASS. No OS-specific code is added. `locate_gh`,
  `run_bounded` and `SystemLinkOpener` already carry the per-OS arms and their tests; all new logic
  is in `micold-core` (tested on all three OSes in CI) or platform-neutral reducers.
- [x] **VII. Documentation First-Class**: PASS. The user guide's worktree chapter gains the
  indicator and the switch with story 1, the tooltip lines and **Open pull request** with story 2,
  the removal suggestion with story 3, and the refresh behaviour with story 4 — each in the
  milestone that ships it (FR-035). `settings.md`, `component-library.md` and `architecture.md` are
  updated with the code they describe.
- [x] **VIII. Reusable UI Component Foundation**: PASS. `PullRequestIndicator` is a new shared
  component with the builder API, posed in the showcase (FR-033). The row, menu, tooltip, chip and
  Settings control reuse `TreeItem`, `MenuItem`, the tooltip string, the tag chip slot, `Checkbox`
  and `field_note`.

### Re-check after Phase 1 design

All eight still **PASS**. Recorded rather than waved through:

- **A second wire change in the family of 034's** changes `SCHEMA_HASH`; client and daemon of
  different builds refuse each other at the handshake, as intended. The RPC and the settings field
  ship in one milestone, with one bump.
- **A timer subscription** is added, guarded so that the idle window with the switch off has none
  (R8); the idle-subscription gate is extended to hold that.

No entry in Complexity Tracking.

## Requirement → design map

| Requirement | Where |
|---|---|
| FR-001, FR-007 | Projection joins rows to statuses by branch; no branch or "Default" → nothing ([data-model §4](./data-model.md)); `PullRequestIndicator` in the trailing slot (R14) |
| FR-002, FR-004, FR-005, FR-006 | `select_pull_request` (R3); only the project's own `GithubRepo` is ever queried ([pull-request-source §2–3](./contracts/pull-request-source.md)) |
| FR-003, FR-008 | `reduce_checks` over counts by state (R4) |
| FR-009, FR-033 | Seven distinct glyphs, fixed size, stale form; showcase entry ([pull-request-ui §1](./contracts/pull-request-ui.md)) |
| FR-010, FR-011, FR-012 | `worktree_tooltip` extended; byte-identical without a status; built from state only ([pull-request-ui §3](./contracts/pull-request-ui.md)) |
| FR-013, FR-014 | `worktree_menu_items` entry → `WorktreeMsg::PullRequestOpenRequested` → `LinkOpener` (R15) |
| FR-015, FR-016, FR-017 | `MergedBranchCheck` RPC + `containment` (R11); chip and tooltip line only; Delete path untouched |
| FR-018, FR-018a | Named start events, only in the window that holds the project (R6); branches from the current listing; refresh trigger is the list refresh's end (R7) |
| FR-019 | `Passing` failures keep statuses; `is_stale` at 600 s; stale form and `Read:` line (R10, R13) |
| FR-020 | Statuses live in their own state field; the reducer writes nothing else ([reading-and-wire §2](./contracts/reading-and-wire.md)) |
| FR-021 | `spawn_blocking` + `run_bounded` 10 s; daemon steps bounded 10 s in the client |
| FR-022 | `Phase::Reading { again }` (R8); holding the project is a start condition (R6) |
| FR-023 | One request per 50 branches; counts, not check nodes (R2) |
| FR-024 | `rate_limit_pause` from `--include` headers; starts refused while paused (R9) |
| FR-025, FR-026 | `ReadingFailure::Unavailable` clears; no remote → `gh` not run; no notification anywhere (R10) |
| FR-027 | Reading starts after `RefreshFinished` settled the control (R7) |
| FR-028 | `gh` owns the sign-in; no anonymous path exists (034 R1); the arguments carry no credential ([pull-request-source §2](./contracts/pull-request-source.md)) |
| FR-029, FR-030 | `pr_status_enabled`, default false, service-owned; Settings control and its text (R12) |
| FR-031 | Arguments are `owner`, `name`, branch names ([pull-request-source §2](./contracts/pull-request-source.md)) |
| FR-032 | No `Serialize`, redacting `Debug`, source gate (R15) |
| FR-034 | No new platform code (Constitution VI above) |
| FR-035 | User-guide tasks per story (Documentation, below) |

## Test strategy by layer

| Layer | What it holds | Requirements |
|---|---|---|
| `micold-core` unit (`mise run test-core`) | `status_query`/`status_args`, `split_response`, `parse_status` against recorded `gh --include` output in `tests/fixtures/gh/`, `select_pull_request`, `reduce_checks`, `reading_failure`, `rate_limit_pause`, `is_stale`, `containment`, `parse` of `MergedBranchCheck` git answers, settings round trip and default, protocol round trip and schema pin | FR-002 to FR-006, FR-008, FR-019, FR-023 to FR-026, FR-028 (the arguments name no token and no anonymous fallback exists), FR-030 to FR-032, SC-002 |
| `micold-daemon` integration | `MergedBranchCheck` against a temp repository (equal tip, behind, ahead, missing branch, missing object); non-repository rejection; `SettingsSet { pr_status_enabled }` persisted and broadcast | FR-015, FR-017, FR-018a, FR-029, FR-030 |
| `micold-client` reducer | `features/pr_status.rs`: start events, one reading at a time, `again`, pause (kept across a project switch), failure kinds (each leaves no notice, dialog or error line), switch off clears, project switch drops answers, a window that is refused or displaced clears and reads nothing, a take-over reads once; sidebar projection; `worktree_tooltip`; menu items; settings draft | stories 1 to 4, FR-001, FR-007, FR-010 to FR-022, FR-024, FR-025, FR-027, SC-004, SC-007 |
| `micold-client` source gates | `pr_status_is_read_only_on_named_events.rs` (new); `idle_subscriptions.rs` (extended); `no_concrete_implementations`; `features_are_render_free`; `material_builder_api`; `showcase_completeness`; `icons_font`; `settings_sections`; a gate that `PullRequestStatus` has no `Serialize` and no derived `Debug` | FR-018, FR-026, FR-032, FR-033, SC-006, SC-008, SC-011 |
| Layout/geometry gates | new covered states in `tests/support/covered_states.rs` (row with indicator, with indicator and chip, stale, narrow sidebar; Settings GitHub section with the switch) and `tests/fixtures/layout_snapshot.txt` regenerated | FR-001, FR-009, FR-011, FR-029 |
| quickstart §B (`visual-pass`) | light and dark theme, every state in the showcase, the real `gh`, opening the browser, the delete confirmation, `gh` missing and no remote, a second window, desktop launch, sandbox placement | FR-009, FR-013, FR-016, FR-034, SC-001, SC-003, SC-004, SC-005, SC-007, SC-009, SC-010 |
| Documentation (CI's user-guide gate, review B) | the user guide's worktree chapter and `settings.md`, updated in the milestone that ships each behaviour | FR-035 |

## Project Structure

### Documentation (this feature)

```text
specs/040-worktree-pr-ci-status/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── pull-request-source.md     # query, parsing, selection, check reduction, failures
│   ├── reading-and-wire.md        # schedule, start events, MergedBranchCheck, the setting
│   └── pull-request-ui.md         # indicator, row, tooltip lines, menu entry, Settings text
└── tasks.md                       # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/pull_request.rs            # NEW: types, query, parse, select, reduce, failures, source trait + fake
├── src/github.rs                  # GhCli gains the pull request source impl (shared runner)
├── src/git.rs                     # Git::branch_tip, Git::is_ancestor; pure containment()
├── src/settings.rs                # Settings.pr_status_enabled
├── src/protocol/messages.rs       # MergedBranchCheck pair; pr_status_enabled on DaemonSettings, SettingsSet
├── src/protocol/version.rs        # 20 → 21
└── tests/                         # pull_request_*.rs, fixtures/gh/pr_*.txt, schema_hash.rs, protocol_roundtrip.rs, settings_roundtrip.rs

crates/micold-daemon/
├── src/server.rs                  # MergedBranchCheck arm (copy of RemoteList's); SettingsSet field
├── src/catalog.rs, src/state.rs   # pr_status_enabled, as tool_server_enabled
└── tests/merged_branch_check.rs   # NEW

crates/micold-client/
├── src/features/pr_status.rs      # NEW: state, schedule reducer, messages
├── src/features/sidebar.rs        # projection carries the row's status; worktree_tooltip extended
├── src/features/worktree.rs       # Msg::PullRequestOpenRequested
├── src/features/settings.rs       # draft field + Msg::PrStatusToggled; section title "GitHub"
├── src/shell/pr_status.rs         # NEW: RemoteList → gh → MergedBranchCheck task glue
├── src/shell/capabilities.rs      # IssueTooling gains the pull request source factory
├── src/shell/subscriptions.rs     # the guarded 300 s interval
├── src/shell/daemon_sync.rs       # PendingOp::MergedBranchCheck; settings mirror
├── src/shell/persist.rs           # SettingsSet field
├── src/icons.rs                   # seven indicator Icon variants + OpenInBrowser (menu entry)
├── src/ui/material/pull_request_indicator.rs   # NEW shared component
├── src/ui/sidebar.rs, src/ui/mod.rs            # trailing slot, chip, menu entry
├── src/ui/settings/github.rs      # the switch
├── src/showcase/catalogue.rs, sections/atoms.rs
└── tests/                         # features_pr_status.rs, pr_status_is_read_only_on_named_events.rs, …

docs/user-guide/worktrees-and-sessions.md, docs/user-guide/settings.md,
docs/development/ (component library, architecture)
```

**Structure Decision**: the existing three-crate workspace. Rules in `micold-core`, state in a
client feature module, effects in a client shell module, one read-only arm in the daemon — the same
split feature 034 used.

## Delivery order

1. **Foundation**: `pull_request` core module with fixtures; the setting and the wire change;
   daemon arm.
2. **US1**: reducer with the open-project and switch-on readings, the indicator component, the row;
   user guide for the indicator and the switch.
3. **US2**: tooltip lines, **Open pull request**; user guide.
4. **US3**: `MergedBranchCheck` use, chip and tooltip line; user guide.
5. **US4**: interval, refresh trigger, `again`, rate-limit pause, stale form; user guide.
6. **Polish**: architecture and component docs, quickstart §B.

## Risks

| Risk | Mitigation |
|---|---|
| `gh` or GitHub changes a field or a state name | Parsing is pinned by recorded fixtures; an unknown check state reads as pending, an unparseable answer is a passing failure that keeps the last status (R4, R10) |
| Another feature takes protocol version 21 first | Take the next free number when the milestone is implemented (R11) |
| The interval timer wakes an idle window | Subscribed only while the switch is on and a project is shown; gate extended (R8) |
| A branch name shared with many fork pull requests hides the worktree's own | Two connections of 10; bound recorded (R2) |

## Complexity Tracking

No violations.
