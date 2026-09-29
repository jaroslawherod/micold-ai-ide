# Implementation Plan: Create a Worktree from a GitHub Issue

**Branch**: `feat/allow-to-create-worktree-from-github-issue` | **Date**: 2026-09-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/034-github-issue-worktree/spec.md`

## Summary

Add a third branch source, **GitHub issue**, to the create-worktree form. Choosing it lists the open
issues of the project's github.com repository; picking one fills the ticket with the issue number,
the name with the (shortened) title, and the type from an application-wide, ordered label-to-type
mapping edited in Settings. The worktree is then created by the unchanged new-branch path.

The design turns on three placement decisions:

1. **GitHub is reached through the user's own GitHub CLI** (`gh api graphql`), not an HTTP client
   of ours ([R1](./research.md#r1--how-github-is-reached-the-github-cli-gh-as-a-subprocess)). That
   is how "use the existing sign-in, never handle credentials" (FR-022) is true on all three OSes —
   `gh` owns the keychain / Credential Manager / Secret Service question — and why no anonymous
   fallback can exist. The workspace gains **no new dependency**.
2. **The fetch runs in the client, on the host; the daemon only reads remotes**
   ([R4](./research.md#r4--where-the-fetch-runs-in-the-client-on-the-host--never-in-the-daemon),
   [R5](./research.md#r5--finding-the-repository-a-new-read-only-remotelist-rpc)). Feature 027 can
   put the daemon in a container without `gh`, the keychain or the user's network; FR-026 says the
   sandbox must neither enable nor block the list. The daemon answers a new read-only `RemoteList`
   RPC (a copy of 016's `BranchList`), because under a Windows-host sandbox the client must not run
   git itself (`Capabilities::without_local_git`). It reads the repository-local remote URLs
   only (`git config --local`), so a global `insteadOf` that reaches a sandboxed daemon only
   through an opt-in credential share cannot change the answer.
3. **`gh` is located the way the user's terminal would find it**, then by well-known install
   directories ([R3](./research.md#r3--finding-gh-when-the-app-is-launched-from-the-desktop-principle-vi)):
   the environment-include `PATH` (feature 011's existing answer to "launched from the Dock"), then
   the process `PATH`, then a per-OS table — a pure function over an explicit `HostOs` and an
   injected existence probe, tested for all three OSes on every host.

Two existing mechanisms are extended rather than copied: feature 011's bounded subprocess runner
moves to `micold_core::process` and learns to drain its pipes while waiting (a GraphQL page is
larger than a pipe buffer — [R6](./research.md#r6--timeouts-and-killing-a-hung-gh)), and the wire
protocol moves to version 16 for the new RPC.

Everything with a decision in it — remote choice, URL parsing, locating `gh`, paging and the 1,000
cap, failure classification, the search merge, `name_from_title`, the label mapping and its
validation, and the form's state machine with its staleness rule — is render-free and tested first.
The UI reuses feature 021's `Typeahead` for the picker and the New-branch inputs for type/ticket/name;
the only shared-component change is `ToggleChip::disabled` ([R13](./research.md#r13--the-source-switchs-disabled-state-extend-togglechip)).

## Technical Context

**Language/Version**: Rust, edition 2021, workspace MSRV (unchanged)

**Primary Dependencies**: `iced` 0.14, `serde`/`serde_json`, `tokio` (client `Task::perform` +
`spawn_blocking`), `micold-core` modules `git`, `naming`, `typeahead`, `settings`, `process`,
`provider`, `env_include`. External runtime tool: the **GitHub CLI `gh`** (user-installed, any 2.x),
invoked as a subprocess. **No new crate.**

**Storage**: `settings.json` gains `issue_label_types` (additive, defaulted; `SETTINGS_VERSION` stays
4). Issues are never persisted (FR-023, SC-006).

**Testing**: `mise run test-core` for the render-free rules; `mise run gate` for the workspace;
reducer tests under `crates/micold-client/tests/`; daemon RPC test under
`crates/micold-daemon/tests/`; a release-build rank budget; `quickstart.md` §B for render glue.

**Target Platform**: Linux, macOS, Windows desktop. The only OS-specific data is the well-known `gh`
directory table, selected by an explicit `HostOs` value; the only process-level difference is
`run_bounded`'s existing Unix/Windows kill arms.

**Project Type**: desktop application (three-crate Cargo workspace).

**Performance Goals**: local filtering of 1,000 issues within one frame budget class (< 50 ms release,
test-held; SC-003's 1 s is the user-facing bound); network failure reported within 10 s per request
(SC-004, FR-007).

**Constraints**: no network contact except after the user chooses the source or searches beyond the
cap (FR-003, Principle IV); nothing but `owner/name` and typed search text sent (FR-025); new-branch
and existing-branch creation untouched and offline-capable (FR-024); GitHub search API budget of
30/min protected by a 300 ms debounce on the network leg only
([R9](./research.md#r9--staleness-fr-007a-and-the-search-debounce)).

**Scale/Scope**: two new core modules (`github`, `issue_types`) + two extended (`naming`,
`settings`) + `process::run_bounded` promoted; one protocol variant pair; one daemon arm; the form
feature extended with ~10 messages; one Settings section; one shared-component method; three
`Icon` variants on existing Material Symbols glyphs; four documentation pages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every decision is in tested render-free code before
  its caller: `micold-core` (remote choice, URL parse, `locate_gh`, `load_listing`, `classify`,
  `merge_searched`, `name_from_title`, `type_for_labels`, `validate_mapping`, settings round trip),
  the form and settings reducers (`issue_source_state.rs`, `features_settings.rs`), the daemon arm
  (`remote_list.rs`). `GhCli` itself is a thin `Command` builder over `run_bounded`, exercised by a
  fixture-driven classify test and the §B pass; the GUI exception is claimed only for `src/ui/`
  composition (quickstart §B). The test-list is derived by `speckit.tdd.plan` into
  `tdd/test-list.md`.
- [x] **II. Multi-Session Support**: PASS. No session state. The form is per-open-form; the request
  counter is client-process state; the mapping is global by requirement (FR-016) and read per pick,
  so projects open side by side see the same, current mapping (SC-005) and leak nothing to each other.
- [x] **III. Worktree Integration**: PASS. An issue-sourced worktree is created by the existing
  new-branch path with its pre-flight and conflict prompt (FR-012). `RemoteList` is read-only git
  metadata through the existing `Git` trait.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. The mapping is local (`settings.json`).
  GitHub is contacted only after the user's explicit, informed opt-in — choosing the source, whose
  notice names the repository and says it contacts GitHub (FR-003, FR-025) — and only with the
  repository identity and typed search text. Offline, every other creation path works (FR-024), and
  the issue source fails within 10 s with a retry (SC-004). Remote discovery at form open is local.
- [x] **V. Rust + iced Stack**: PASS. Rust and iced only. Invalid states narrowed by types:
  `GithubRepo` is constructible only from a github.com URL; `IssueList`/`SearchState` enums make
  "loading and failed" or "searching a complete list" unrepresentable; `ConventionalType` stays a
  closed enum in the mapping; `Issue` has no `Serialize`, so it cannot be persisted by accident.
- [x] **VI. Cross-Platform Parity**: PASS. `gh` is the same program with the same outputs on all
  three OSes and owns the per-OS credential store; `locate_gh`'s per-OS tables are pure and tested
  for every `HostOs` on every CI host against a fake existence probe (separator and executable name
  come from `HostOs`, not the host); `run_bounded` already carries the Unix/Windows kill arms and now
  drains pipes so large output behaves the same on every OS; `no_window` suppresses the console on
  Windows. Remote discovery reads repository-local config only, so it answers the same on host and
  sandbox placements. Desktop-launch and sandbox parity (FR-026) are §B10 and §B11. All
  platform-sensitive logic is in `micold-core`, whose whole suite CI runs on all three OSes; the new
  `micold-client` reducer tests are platform-neutral and run in the Linux workspace job, like
  `branch_search_state.rs`.
- [x] **VII. Documentation First-Class**: PASS. User guide "From a GitHub issue" ships with the source
  (US1), the type-from-labels section with US2, the Settings page with US3; `component-library.md`
  gains `ToggleChip::disabled`; `architecture.md` records where the fetch runs. Each lands in the
  milestone whose behaviour it describes (CI's user-guide gate).
- [x] **VIII. Reusable UI Component Foundation**: PASS. The picker is feature 021's shared
  `Typeahead`, unchanged; the source switch stays `ToggleChip`, extended with a chainable
  `.disabled(bool)` and posed in the gallery; the Settings rows compose `TextField`, `Select`,
  `IconButton`, `Button`. No new widget.

### Re-check after Phase 1 design

All eight still **PASS**. Two things worth recording rather than waving through:

- **A new external-tool dependency (`gh`)** is not a crate, so the dependency-vetting rule does not
  formally apply, but its spirit does: `gh` is GitHub's own, MIT-licensed, actively maintained CLI,
  and the feature degrades to a plain "not installed" message without it — nothing else in the app
  depends on it ([R1](./research.md#r1--how-github-is-reached-the-github-cli-gh-as-a-subprocess)).
- **A new protocol message** changes `SCHEMA_HASH`, so mismatched client/daemon builds refuse each
  other at the handshake — the intended behaviour, and why both ship in one milestone.

No entry in Complexity Tracking.

## Requirement → design map

| Requirement | Where |
|---|---|
| FR-001 | `BranchSource::Issue`; source switch ([issue-picker-ui §1](./contracts/issue-picker-ui.md)) |
| FR-002 | `RemoteList` RPC + `choose_remote` + `GithubRepo::from_remote_url` ([remote-list-rpc](./contracts/remote-list-rpc.md)); disabled chip with reason |
| FR-003 | Reducer invariant 1 ([data-model §5](./data-model.md)); `issues_are_requested_only_on_named_events.rs` |
| FR-004 | GraphQL `issues(states: OPEN, orderBy UPDATED_AT DESC)`, `load_listing` cap 1,000, `complete` + caption ([github-issue-source §3–4](./contracts/github-issue-source.md)) |
| FR-005 | `typeahead::rank` over `Issue::row_text`; 021 keyboard rule ([issue-naming-and-typing §5](./contracts/issue-naming-and-typing.md)) |
| FR-005a | `SearchState`, debounce, `search_open`, `merge_searched`, invariants 4–6 |
| FR-006 | `IssueList::Loading` + progress line; other inputs live |
| FR-007 | `run_bounded` 10 s; `classify`; `IssueLoadError::message`; Retry |
| FR-007a | `issue_request_seq` outside the form ([R9](./research.md#r9--staleness-fr-007a-and-the-search-debounce)) |
| FR-008 | `Loaded` with zero issues → caption |
| FR-009, FR-010, FR-010a, FR-011 | `IssuePicked` reducer + `naming::name_from_title` |
| FR-012 | `preview()`/`can_submit()`/create path treat `Issue` as `New` |
| FR-013, FR-014, FR-015 | `issue_types::type_for_labels`; pick replaces or clears type |
| FR-014a | Shell reads mapping from the settings store at the pick |
| FR-016, FR-017, FR-020, FR-021 | `Settings.issue_label_types`, `default_mapping()` ([issue-naming-and-typing §2–3](./contracts/issue-naming-and-typing.md)) |
| FR-018, FR-019 | Settings → GitHub issues section; `validate_mapping` → `FieldError` |
| FR-022 | `gh` owns the sign-in; `NotSignedIn`; no anonymous path exists |
| FR-023 | Issues only in `WorktreeForm`; `Issue` not `Serialize` |
| FR-024 | New/Existing paths untouched; issue failures confined to the source body |
| FR-025 | Notice with `owner/name`; request arguments limited ([github-issue-source §3](./contracts/github-issue-source.md)) |
| FR-026 | Client-side fetch (R4); `locate_gh` (R3) |

## Test strategy by layer

| Layer | What it holds | Requirements |
|---|---|---|
| `micold-core` unit (`mise run test-core`) | URL parse, remote choice, `parse_remote_list`, `locate_gh`, GraphQL parsing, paging/cap, classify (fixtures), messages, search merge, `name_from_title`, mapping lookup/validation, settings round trip, `run_bounded` timeout | FR-002, 004, 005, 005a, 007, 010, 013, 014, 016–022, 026 |
| `micold-core` release budget | 1,000-row rank < 50 ms | SC-003 |
| `micold-daemon` integration | `RemoteList` against a temp repo; non-repo rejection | FR-002 |
| `micold-client` reducer | form state machine, staleness, pick, settings draft/validation | US1–US3 scenarios, FR-003, 006, 007a, 008, 010a, 014a, 023 |
| `micold-client` source gates | only-on-named-events; `no_concrete_implementations`; `features_are_render_free`; `material_builder_api` (ToggleChip); `showcase_completeness`; `icons_font` | FR-003, VIII |
| Layout/geometry gates | new covered states in `tests/support/covered_states.rs` (disabled chip, Loading, Failed, Loaded + cap caption, Searching, Settings GitHub issues with an offending entry) and `tests/fixtures/layout_snapshot.txt` regenerated ([issue-picker-ui §4a](./contracts/issue-picker-ui.md)) | FR-001, FR-002, FR-006, FR-007, US3 AS1, AS5 |
| quickstart §B (visual-pass) | render glue, real `gh`, desktop launch, sandbox placement | FR-026, SC-001, SC-004 |

## Project Structure

### Documentation (this feature)

```text
specs/034-github-issue-worktree/
├── plan.md              # This file
├── research.md          # Phase 0 — R1–R14
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1 — §A automated, §B recorded manual pass
├── contracts/
│   ├── github-issue-source.md    # locating gh, the trait, queries, parsing, classification
│   ├── remote-list-rpc.md        # Git::remote_list, RemoteList RPC, remote choice
│   ├── issue-naming-and-typing.md# name_from_title, mapping, settings schema, the pick, matching
│   └── issue-picker-ui.md        # form UI, shell effects, Settings section, docs
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/
│   ├── lib.rs                  # + pub mod github, issue_types
│   ├── github.rs               # NEW — GithubRepo, choose_remote, HostOs, locate_gh, Issue*, IssueSource, GhCli, FakeIssueSource, load_listing, classify, merge_searched
│   ├── issue_types.rs          # NEW — LabelTypeEntry, default_mapping, type_for_labels, validate_mapping
│   ├── naming.rs               # + name_from_title; ConventionalType serde
│   ├── settings.rs             # + issue_label_types
│   ├── process.rs              # + run_bounded, RunOutcome, kill arms (moved from env_include.rs); drains pipes
│   ├── env_include.rs          # uses process::run_bounded
│   ├── git.rs                  # + Git::remote_list, GitRemote, parse_remote_list, FakeGit::with_remote
│   ├── protocol/messages.rs    # + ClientMsg::RemoteList, OperationResult::RemoteList
│   └── protocol/version.rs     # PROTOCOL_VERSION 15 → 16
└── tests/
    ├── github_remote.rs  git_remotes.rs  github_locate.rs  github_parse.rs  github_load.rs
    ├── github_classify.rs  github_search.rs  process_run_bounded.rs
    ├── typeahead_budget.rs     # + 1,000-issue-row case (existing release-build CI step)
    ├── schema_hash.rs          # protocol-version pin → 16
    ├── naming_from_title.rs  issue_types.rs  settings_issue_mapping.rs
    └── fixtures/gh/            # captured gh stderr/stdout per failure kind

crates/micold-daemon/
├── src/server.rs               # + RemoteList arm
└── tests/remote_list.rs        # NEW

crates/micold-client/
├── src/
│   ├── features/worktree_form.rs   # BranchSource::Issue, GithubAvailability, IssueList, SearchState, messages
│   ├── features/settings.rs        # GithubIssues section, GithubDraft, messages, validation
│   ├── features/window.rs          # FieldId::IssueMappingLabel(usize)
│   ├── icons.rs                    # + IssueMapping (`label`), MoveUp, MoveDown — existing font glyphs
│   ├── shell/capabilities.rs       # + issue-source factory
│   ├── shell/daemon_sync.rs        # + PendingOp::RemoteList, load/search tasks
│   ├── shell/persist.rs            # mapping saved with the other client-owned fields
│   ├── main.rs                     # route the new form messages to the shell where they have effects
│   ├── ui/material/toggle_chip.rs  # + .disabled(bool)
│   ├── ui/worktree_form.rs         # third chip, issue body, shared naming_inputs
│   ├── ui/settings/github.rs       # NEW — the mapping editor
│   └── showcase/                   # ToggleChip disabled pose
└── tests/
    ├── issue_source_state.rs                       # NEW
    ├── issues_are_requested_only_on_named_events.rs # NEW
    ├── features_settings.rs                        # extended
    ├── support/covered_states.rs                   # + issue-source and GitHub-issues-section states
    └── fixtures/layout_snapshot.txt                # regenerated

docs/
├── user-guide/worktrees-and-sessions.md   # From a GitHub issue; labels choose the type
├── user-guide/settings.md                 # GitHub issues section
├── development/component-library.md       # ToggleChip::disabled
└── development/architecture.md            # where the issue fetch runs
```

**Structure Decision**: the existing three-crate workspace, unchanged. Decision logic goes to
`micold-core` beside `git`, `naming` and `typeahead`, where this codebase keeps it; the daemon gains
one read-only arm; the client gains a capability, reducer states and composition. No new crate,
binary or manifest dependency.

## Delivery order

Sliced by story priority; each slice ships something observable (milestones in `tasks.md`).

| Slice | Delivers | Gate |
|---|---|---|
| **1a (US1, P1)** | Remote discovery RPC; locate `gh`; load + list + local search; pick fills ticket/name; loading/empty/failure/retry; staleness; the third chip with disabled reason; the notice; user-guide section; `ToggleChip::disabled` | `mise run gate`; quickstart §B1–B5, B7–B9 |
| **1b (US1 AS10)** | Search beyond the cap: debounce, `search_open`, merge, searching/failed states | §B6 |
| **2 (US2, P2)** | Mapping type + default + settings field (read-only path); pick sets/clears type; user-guide section | §B4 (type) |
| **3 (US3, P3)** | Settings → GitHub issues editor, validation, restore defaults, icons; settings guide | §B12 |
| **Polish** | architecture doc; §B10–B11, B13 recorded | quickstart §B complete |

## Risks

| Risk | Handling |
|---|---|
| `gh` stderr wording changes between versions | Classification is fixture-driven; unknown text is `Other` with the raw line and a retry, never a crash or a wrong "not signed in". |
| Keyring access from a desktop-launched process on Linux (no D-Bus session) | `gh` itself falls back to `hosts.yml`; if it still fails it reports "not logged in" — shown as `NotSignedIn` with the `gh auth login` remedy. §B10 records it. |
| Search API rate limit (30/min) | Only the beyond-cap search uses it, debounced 300 ms; `RateLimited` is a recoverable outcome. |
| The mixed client/daemon refusal after the protocol change | Both halves land in one milestone; the handshake refuses a mixed pair by design. |
| macOS/Windows desktop-launch pass cannot run on this Linux host | `locate_gh` is unit-tested for all three tables, separators and executable names on every CI host with a fake probe; the §B10 arms are recorded in the Polish milestone on those OSes, or escalated as missing access (category 4) if no host is available. |
| A GraphQL page larger than a pipe buffer stalls the child and reads as a timeout | `run_bounded` drains stdout/stderr on reader threads while it waits; a 1 MiB-output test holds it ([R6](./research.md#r6--timeouts-and-killing-a-hung-gh)). |
| A remote named through a global `insteadOf` alias is not recognised | Deliberate, for placement parity (FR-026); documented in the user guide ([R5](./research.md#r5--finding-the-repository-a-new-read-only-remotelist-rpc)). |

## Complexity Tracking

No constitutional violation requires justification. Left empty deliberately.
