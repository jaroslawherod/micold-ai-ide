# Tasks: MCP create_worktree accepts the New worktree form's inputs

**Input**: `specs/550-mcp-create-worktree-form-inputs/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)
**Tests**: required (Constitution I): each test task comes before the code it covers and must be seen failing first.

Format: `- [ ] T### [P?] [US?] Description with file path`

## Phase 1: Setup

- [x] T001 Run the shipped `create_worktree` tests as the SC-004 baseline (`cargo test -p micold-daemon --test mcp_create_worktree`, `-p micold-core --test mcp_tools_catalog --test mcp_policy`) and note which pass, in `specs/550-mcp-create-worktree-form-inputs/autopilot.md`.

## Phase 2: Foundational (blocks every story)

- [x] T002 Write failing tests in `crates/micold-core/tests/mcp_tools_catalog.rs` and `crates/micold-core/tests/mcp_policy.rs` for the reshaped tool: schema lists `type` (enum of `ConventionalType::ALL`), `ticket`, `github_issue` (integer, minimum 1; the description says it is not yet available until M3), `required` is empty, the description states the two alternatives; policy audit target is the branch, the name, or `#<issue>` (FR-015).
- [x] T003 Reshape `Operation::CreateWorktree` into `CreateWorktreeRequest::{Literal, Derived}` in `crates/micold-core/src/mcp/tools.rs` per data-model.md (a `Derived` has at least one of name/type/ticket/issue and never `branch`/`mode`/`remote`); update the schema, description and audit target; `parse_call` keeps its current argument handling until T006; port the daemon match arm in `crates/micold-daemon/src/mcp/tools.rs` so literal calls compile and behave as before.

## Phase 3: User Story 1 - type, ticket and name (P1) + User Story 4 - literal calls unchanged (P1)

**Goal**: derived requests create the form's branch, directory and tags; literal calls are untouched.
**Independent test**: `mcp_create_worktree.rs` derived scenarios and the replayed 034 scenarios.

- [x] T004 [P] [US1] Write failing parser tests in `crates/micold-core/tests/mcp_tools_catalog.rs`: type+ticket+name parses to `Derived`; `branch`/`name` alone is `Literal`; `name` with no `branch` is `Derived`; `branch`, `mode` or `remote` with `type`/`ticket`/`github_issue` is `invalid_input` stating they are alternatives (US4 s2, FR-009); an empty call is `invalid_input` saying what to provide; `mode` or `remote` without `branch` is `invalid_input` (they require `branch`); a valid `github_issue` is refused as `invalid_input` ("github_issue is not supported yet") until M3 replaces it, with nothing created; `github_issue` of 0, negative, non-integer or above 2147483647 is `invalid_input`.
- [x] T005 [P] [US1] Write a failing SC-001 table test in `crates/micold-core/tests/naming.rs`: for all ten `ConventionalType`s, with and without a ticket (including `#123` and a blank ticket), a Windows-reserved suffix in the name, and a very long name, the branch and directory the tool path would use equal `naming::derive`'s.
- [x] T006 [US1] Implement `parse_call("create_worktree", args)` for the request in `crates/micold-core/src/mcp/tools.rs` (rules in contracts/create-worktree-tool.md; `type` via `ConventionalType::from_token`, refusal lists `ConventionalType::ALL`, FR-001). Makes T004 and T005 pass.
- [x] T007 [P] [US1] Write failing daemon integration tests in `crates/micold-daemon/tests/mcp_create_worktree.rs` against a scratch repo: type `fix`, ticket `#123`, name `login crash` gives branch `fix/123_login-crash`, directory `fix-123_login-crash`, sidebar type `fix` and issue tag `123` (US1 s1); `feat` + `Dark mode` gives `feat/dark-mode` with no ticket segment and no issue tag (s2); result carries `branch`, `directory`, `type`, `ticket` (omitted when none) beside the `list_worktrees` row (s3, FR-012); tags equal the form's for the same inputs (SC-003, FR-013); a derived branch that already exists as a local branch, as a remote-only branch (no worktree created at HEAD) or in use elsewhere is refused with a `conflict` and creates nothing (message wording is pinned in T011); literal call shapes replayed unchanged (US4 s1); a call from the Default session is allowed.
- [x] T008 [US1] Implement the derived path in `crates/micold-daemon/src/mcp/tools.rs`: policy check on the parsed request, `naming::derive` (`NamingError` text is the refusal, FR-004), `GitCli::check_branch_name`, accept only `BranchSituation::Free` (never `mode.is_compatible_with`), `ops::create_worktree(..., NewBranch, None)` (FR-014), result fields (FR-012); literal path keeps its code, texts and order (FR-010).
- [x] T009 [P] [US1] Update the `create_worktree` row and add a short section on derived inputs in `docs/user-guide/agent-tools.md` (Constitution VII; this milestone ships the behaviour).

## Phase 4: User Story 2 - reject what the form rejects (P1)

**Goal**: every invalid input is refused with the form's reason and leaves nothing behind.
**Independent test**: one call per invalid input in `mcp_create_worktree.rs`, then check no branch or directory exists.

- [x] T010 [US2] Write daemon tests in `crates/micold-daemon/tests/mcp_create_worktree.rs`. Refusal characterisation tests, expected to pass already because T006/T008 implement them, pinning the reasons: type `feature` (invalid input naming the allowed types, s1); name `???` ("Enter a name (letters or digits)", s2); name with no type ("Select a type", s3); a derived branch failing the ref check ("The resulting branch name is not valid"); each leaves no branch or directory (SC-002).
- [x] T011 [US2] Write failing collision tests in `crates/micold-daemon/tests/mcp_create_worktree.rs`: the same derived inputs twice give a `conflict` that names the branch and says to retry with `branch` and `mode` `existing_local` or `track_remote` (s4, FR-011); with the directory also taken the branch message wins (probe with `Git::branch_exists`), with only the directory taken the directory-taken text; two concurrent identical requests: one wins, no partial worktree; a failure during creation rolls back to nothing.
- [x] T012 [US2] Implement the FR-011 refusal in `crates/micold-daemon/src/mcp/tools.rs`: `LocalAvailable`, `RemoteOnly` and every `BlockReason` give the FR-011 message; `DirectoryTaken` probes the branch first (plan Design 5). Makes T010 and T011 pass.

## Phase 5: User Story 3 - start from a GitHub issue (P2)

**Goal**: `github_issue` fills ticket, name and type as the form's issue pick does.
**Independent test**: fake lookup scripted with an open `bug` issue; compare with `issue_picked`.

- [ ] T013 [P] [US3] Write failing tests in `crates/micold-core/tests/naming.rs` for `naming_for_issue` (ticket = issue number, name = title cut as the form cuts it (a title over 50 characters included), type from the first matching label mapping entry ignoring case, no match gives no type) and `WorktreeNaming::overridden_by` (explicit type, ticket, name replace the issue's, FR-005, FR-006), including a title longer than 50 characters.
- [ ] T014 [P] [US3] Write failing tests in `crates/micold-core/tests/github_issue_lookup.rs` (new): `lookup_args` sends only owner, name and number (`n` as typed `-F`), `parse_lookup` for open, closed, `issue: null`, `repository: null`, NOT_FOUND beside `issue: null`, rate limit, malformed; a `FakeIssueSource` returning `Load(TimedOut)` gives the 10 s text; the lookup is not made without a `github_issue`.
- [ ] T015 [US3] Implement `naming_for_issue` and `overridden_by` in `crates/micold-core/src/naming.rs`, and `IssueLookup`, `IssueSnapshot`, `IssueLookupError`, `lookup_args`, `parse_lookup` with impls for `GhCli` and `FakeIssueSource` in `crates/micold-core/src/github.rs` (contracts/issue-lookup.md).
- [ ] T016 [P] [US3] Write a failing client test in `crates/micold-client/tests/` that `worktree_form::issue_picked` equals `naming_for_issue` for the same issue (FR-016).
- [ ] T016b [US3] Make `issue_picked` in `crates/micold-client/src/features/worktree_form.rs` call `naming_for_issue` (form behaviour unchanged, existing form tests green). Depends on T015.
- [ ] T017 [P] [US3] Write failing tests for `catalog.label_mapping()` in `crates/micold-daemon/tests/`: a custom mapping saved after daemon start is read at call time; default mapping when the file is absent or corrupt.
- [ ] T017b [US3] Implement `Catalog::label_mapping()` and `DaemonState::label_mapping()` on the blocking pool in `crates/micold-daemon/src/catalog.rs` and `state.rs` (data-model.md).
- [ ] T019 [US3] Write failing daemon tests (each injects its fake through `DaemonState::set_issue_lookup`, added in T020) in `crates/micold-daemon/tests/mcp_create_worktree.rs`: open `bug` issue 123 "Login crash" gives `fix/123_login-crash` (s1); unmapped labels without `type` gives "Select a type", with `type` succeeds (s2); closed issue, pull request, missing issue, no GitHub remote, signed out, no access, rate limit and timeout each refuse with the form's text and create nothing (s3, FR-007); explicit `type`/`ticket`/`name` replace the issue's (s4); a call-counting fake sees zero lookups for calls without `github_issue` (SC-005); the policy check refuses a denied caller before any lookup.
- [ ] T020 [US3] Add the injectable issue-lookup factory and `set_issue_lookup` to `DaemonState` in `crates/micold-daemon/src/state.rs` (production default: `GhCli` via `locate_gh_on_host`, `ToolMissing` when none), and implement issue resolution in `crates/micold-daemon/src/mcp/tools.rs` (remote read via `GitCli::remote_list`, `parse_remote_list`, `choose_remote`; `IssueLookup::read_issue` on the blocking pool; `IssueLoadError::message` and not-open texts; not-open as `invalid_input`, load failures as `service_error`; then `naming_for_issue` and `overridden_by`). Makes T019 pass.
- [ ] T021 [P] [US3] Add the `github_issue` text and its failure reasons to `docs/user-guide/agent-tools.md`.

## Phase 6: Polish

- [ ] T022 Run `specs/550-mcp-create-worktree-form-inputs/quickstart.md` Part B with real `gh` and record the results (close unit).

## Dependencies

Phase 2 blocks everything. US1+US4 (Phase 3) before US2 (Phase 4) before US3 (Phase 5). Within a phase, a test task precedes its implementation; `[P]` tasks touch disjoint files or independent test files.

## Implementation Strategy

MVP is M1 (derived type/ticket/name + unchanged literal calls). Then the collision/validation hardening (M2), then GitHub (M3).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Derived type, ticket and name 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: `github_issue` is accepted by the schema but refused as not yet supported; `create_worktree` called with `type`, `ticket` and `name` creates the form's branch, directory and sidebar tags and reports them; literal calls behave as before; the schema and user guide describe the new inputs.
- **Satisfies**: US1 acceptance scenarios 1–3; US4 acceptance scenarios 1–2; FR-001, FR-002, FR-003, FR-009, FR-010, FR-012, FR-013, FR-014, FR-015; SC-001, SC-003, SC-004
- **Verify**: `cargo test -p micold-daemon --test mcp_create_worktree` and `cargo test -p micold-core --test mcp_tools_catalog --test naming` (via `mise run gate`); quickstart B1, B6, B7
- **Depends on**: —
- **Tier**: full

### M2 — Refusals and collisions

- **Tasks**: T010–T012
- **Deliverable**: invalid type, empty name, missing type, invalid branch and an existing derived branch or directory are refused with the form's reasons and the FR-011 retry hint, leaving nothing behind.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-004, FR-011; SC-002
- **Verify**: `cargo test -p micold-daemon --test mcp_create_worktree` (refusal and collision tests); quickstart B2
- **Depends on**: M1
- **Tier**: full

### M3 — Start from a GitHub issue

- **Tasks**: T013–T017b, T019–T021
- **Deliverable**: `create_worktree` with `github_issue` fills ticket, name and type from the issue as the form's pick does, with explicit values overriding, and refuses unusable issues with the form's reasons; no network use otherwise.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-005, FR-006, FR-007, FR-008, FR-016; SC-005
- **Verify**: `cargo test -p micold-core --test github_issue_lookup --test naming`, `cargo test -p micold-daemon --test mcp_create_worktree`; quickstart B3–B5
- **Depends on**: M1, M2
- **Tier**: full
