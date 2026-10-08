# Implementation Plan: MCP create_worktree accepts the New worktree form's inputs

**Branch**: `feat/550_mcp-create-worktree-should-support-the-same-inputs` | **Date**: 2026-10-08 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/550-mcp-create-worktree-form-inputs/spec.md`

## Summary

`create_worktree` (daemon MCP tool, feature 034-daemon-mcp-server) gains `type`, `ticket`, `github_issue`
beside its literal `branch`/`mode`/`remote`. The two input shapes are alternatives. A derived request
resolves to a `WorktreeNaming` and goes through the one `micold_core::naming::derive` the form already
uses, then through the same `ops::create_worktree` the form uses.

**Where the GitHub work runs (the open planning question).** The premise "issue lookup and label mapping
exist only in micold-client" is only half true. Everything that matters is already in `micold-core`, which
the daemon depends on: `github::{GhCli, IssueSource, GithubRepo, choose_remote, locate_gh_on_host,
IssueLoadError::message}`, `issue_types::type_for_labels`, `naming::{derive, name_from_title}` and
`settings::Settings::issue_label_types`. Only the form's glue (paging, debounce, form state) is in
`micold-client`. So the lookup runs **in the daemon process**, in `micold-daemon/src/mcp/tools.rs`, using
those core types; nothing moves between crates and the daemon never calls the client (a client may not be
running, and several windows may be open). What is missing in core is a single-issue read (the
`IssueSource` trait only lists, searches and describes), so one is added next to `IssueSource`. The
label mapping is read at call time from the settings file the client writes (the daemon already holds a
`SettingsStore`), with the default mapping when the file is absent. Full reasoning: [research.md](research.md) D1-D3.

## Technical Context

**Language/Version**: Rust (workspace toolchain, `rust-toolchain.toml`), edition per workspace.

**Primary Dependencies**: existing only: `micold-core` (naming, github, issue_types, settings, mcp::tools, git), `micold-daemon` (mcp::tools, ops, catalog), `serde_json`, `tokio`. No new crate.

**Storage**: the settings JSON file (read only, for the label mapping). No new state.

**Testing**: `cargo test` through `mise run test-core` and `mise run gate`; daemon integration tests in `crates/micold-daemon/tests/mcp_create_worktree.rs` against a scratch git repository; GitHub stubbed through an injected lookup (no real `gh`, no network).

**Target Platform**: Linux, macOS, Windows (Principle VI): no shell-script stubs; the stub is an in-process trait object.

**Project Type**: desktop app with a daemon (workspace of three crates: `micold-core`, `micold-daemon`, `micold-client`).

**Performance Goals**: no GitHub call unless `github_issue` is passed (SC-005); one bounded `gh` run (10 s, `GH_TIMEOUT`) otherwise.

**Constraints**: works offline and signed-out for every call without `github_issue` (FR-008); no credential handled by the code (the user's `gh` does the auth).

**Scale/Scope**: one tool, one new core read, one shared naming helper, schema and user-guide text.

## Constitution Check

*GATE: passes before research and after design.*

| Principle | Verdict | Note |
|---|---|---|
| I. Test-First | PASS | Every task pair is test, then code; the test list comes from the tdd-plan step. Layers in the Test Strategy below. |
| II. Multi-Session Support | PASS | No new state. The caller session is resolved as today (`resolve_caller`); policy check unchanged. |
| III. Worktree Integration | PASS | Creation goes through `ops::create_worktree` (gate, preflight, submodules, rollback, provenance, broadcast). No manual git step. |
| IV. Local-First Storage | PASS | The only network use is the user's own `gh`, run only when `github_issue` is passed, i.e. an explicit opt-in per call; nothing is stored or sent beyond owner/repo/number (FR-008). |
| V. Rust + iced Stack | PASS | Rust only. The request is a sum type (`CreateWorktreeRequest::{Literal, Derived}`) so branch plus derived inputs is unrepresentable after parsing. No UI change. |
| VI. Cross-Platform Parity | PASS | Derivation is the existing Windows-safe `slugify`; `gh` is located by `locate_gh_on_host` (per-OS); tests use an in-process fake, no shell stubs. |
| VII. Documentation First-Class | PASS | `docs/user-guide/agent-tools.md` row and a short section updated in the same change; the tool description and schema carry the rules (FR-015). |
| VIII. Reusable UI Component Foundation | PASS (n/a) | No UI. |

Complexity Tracking: none; no violation.

## Project Structure

### Documentation (this feature)

```text
specs/550-mcp-create-worktree-form-inputs/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── create-worktree-tool.md
│   └── issue-lookup.md
└── tasks.md            # next unit
```

### Source Code (existing files, plus new items marked new)

```text
crates/micold-core/src/
├── naming.rs           # new: naming_for_issue(), WorktreeNaming::overridden_by(); derive/slugify/name_from_title unchanged
├── github.rs           # new: IssueLookup trait, IssueSnapshot, IssueLookupError, lookup_args(), parse_lookup(); impl for GhCli and FakeIssueSource
├── issue_types.rs      # unchanged (type_for_labels)
└── mcp/tools.rs        # Operation::CreateWorktree reshaped; create_worktree schema/description; arg parsing and exclusion rules
crates/micold-daemon/src/
├── mcp/tools.rs        # create_worktree(): derived branch, issue resolution, FR-011 refusal, result fields
├── catalog.rs          # new: label_mapping() reading the settings store at call time
└── state (DaemonState) # new: injectable issue-lookup factory, production default GhCli via locate_gh_on_host
crates/micold-client/src/features/worktree_form.rs
                        # issue_picked() calls naming::naming_for_issue (one shared path, FR-016); behaviour unchanged
docs/user-guide/agent-tools.md
crates/micold-core/tests/{naming_*.rs, github_issue_lookup.rs (new), mcp_tools_catalog.rs, mcp_policy.rs}
crates/micold-daemon/tests/mcp_create_worktree.rs
crates/micold-client/tests/ (form parity)
```

**Structure Decision**: no new crate and no move. Pure logic (resolution, parsing, schema) in `micold-core`; the effectful steps (settings read, `gh` run, git, create) in `micold-daemon`, as the other tools already split.

## Design

1. **Parse (core, pure).** `parse_call("create_worktree", args)` builds `CreateWorktreeRequest`. `Literal { branch, name, mode }` when `branch`, `mode` or `remote` is present (or only `name`+`branch`); `Derived { type_, ticket, name, github_issue }` when any of `type`/`ticket`/`github_issue` is present or `name` stands without `branch`. Both families present: `invalid_input` (FR-009). Neither: `invalid_input` stating what to provide. `type` outside `ConventionalType::from_token` : `invalid_input` listing `ConventionalType::ALL` (FR-001). `github_issue` not an integer >= 1: `invalid_input`.
2. **Policy.** `check_policy` runs on the parsed request before any network call, so a refused caller causes no lookup. The audit target is the branch, the name, or `#<issue>`.
3. **Resolve (daemon).** For `github_issue`: read remotes (`GitCli::remote_list`, `parse_remote_list`, `choose_remote`) on the blocking pool; no GitHub remote refuses with the form's text. Locate `gh`, call `IssueLookup::read_issue` on the blocking pool, refuse with `IssueLoadError::message` or the new not-open texts. Then `naming::naming_for_issue(issue, mapping)` and `overridden_by(explicit)`. Without `github_issue` none of this runs.
4. **Derive.** `naming::derive` gives `DerivedNames`; `NamingError` text is the refusal (FR-004). `GitCli::check_branch_name` as today.
5. **Collision.** `ops::branch_situation`, but a derived request does NOT use `mode.is_compatible_with` (it admits `RemoteOnly` for `NewBranch`, `worktree.rs:802`, which would silently create at HEAD). It accepts only `BranchSituation::Free`. `LocalAvailable`, `RemoteOnly` and `Blocked` (every `BlockReason`) get the FR-011 message (names the branch, says retry with `branch` and `mode` `existing_local` or `track_remote`). `DirectoryTaken` keeps its own text. `preflight` reports `DirectoryTaken` before any branch situation (`worktree.rs:952`), and the usual collision (the same inputs made twice) hits both, so on `DirectoryTaken` a derived request probes the branch alone with `Git::branch_exists` (`git.rs:24`): if the branch exists the FR-011 message wins, otherwise the directory text. A test pins both orders. Nothing is overwritten.
6. **Create.** `ops::create_worktree(..., NewBranch, None)` (FR-014: same gate, submodules, rollback, provenance, broadcast). Sidebar tags come from the directory name through `naming::parse_tags` (FR-013), so no extra write.
7. **Result.** The existing `list_worktrees` row plus `branch`, `directory` always, and `type`, `ticket` (omitted when none) for derived requests (FR-012).
8. **Literal calls** keep their code path, error texts and ordering (FR-010, SC-004); only the argument parser and the result's extra fields change.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-002, FR-009, FR-015 | core `mcp/tools.rs` parser, schema, description; contract create-worktree-tool.md |
| FR-003, FR-004, FR-016 | core `naming.rs` (`derive` unchanged, shared helper); data-model.md |
| FR-005, FR-006 | `naming_for_issue`, `overridden_by`; research D4 |
| FR-007, FR-008 | `IssueLookup` in core `github.rs`; contract issue-lookup.md; daemon lookup step 3 |
| FR-010 | Design 8; replay of 034 tests |
| FR-004 | Design 4; daemon integration (unknown-type, no-type, empty-slug refusals) |
| FR-011 | Design 5 |
| FR-012 | Design 7 |
| FR-013 | Design 6 |
| FR-014 | Design 6 |
| SC-001..SC-005 | Test Strategy |

## Test Strategy

| Layer | Covers |
|---|---|
| core unit (`mise run test-core`): naming | `naming_for_issue` and `overridden_by` (FR-005, FR-006, long titles, blank ticket); SC-001 table over all ten types with and without a ticket against `derive`; Windows-reserved suffix |
| core unit: github | timeout: `GhCli::with_timeout` tiny bound or a fake returning `Load(TimedOut)` gives the 10 s text (FR-007); `lookup_args` (only owner, name, number leave the machine; number as typed `-F`), `parse_lookup` for open, closed, issue null (PR/missing), repository null (no access), rate limit, malformed; `FakeIssueSource` lookup scripting |
| core unit: mcp tools | argument parser (exclusion rules, bad type, bad `github_issue`, empty call), schema/description lists new inputs and `branch` not required (`mcp_tools_catalog.rs`), policy audit target (`mcp_policy.rs`) |
| daemon integration (`mcp_create_worktree.rs`, scratch repo, fake lookup) | remote-only derived branch refused (not created at HEAD); FR-004 refusals; US1-US4 scenarios, collision refusal text, directory-taken, concurrent identical requests (one wins, no partial worktree), rollback leaves nothing, sidebar tags equal the form's for the same inputs (SC-003), no lookup without `github_issue` (SC-005 via call-counting fake), no GitHub remote refusal |
| daemon: mapping | `catalog.label_mapping()` reads a custom mapping saved after daemon start; default when file absent or corrupt |
| client | `issue_picked` result equals `naming_for_issue` for the same issue (guards drift between form and tool, FR-016); existing form tests unchanged |
| geometry gates / visual pass | none: no UI changes |
| quickstart | manual pass in [quickstart.md](quickstart.md) with real `gh` |
