# Autopilot ledger — worktree-parent M1, M1b (#1858), M2 (#1859), M3 (#1860)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Milestone M2 (Phase 5: T015-T019) of specs/1857-worktree-parent/tasks.md; M1 (#1858) and M1b merged earlier; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1859
- **Worktree branch**: claude/project-thread-lcys26
- **Started**: 2026-11-13
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1861 | Design (spec, plan, tasks; merged content, treated as reviewed) | not merged, content on this branch | — |
| #1873 | M1 | merged | eb27a76d639c793f396177c4e53c203c26e13058 |
| #1912 | M1b | merged | 95fa459da7e2b4381424c66ba66f40454159a70b |
| #1917 | M3 | merged | 2c3e7fd77e9305e8a801bcd8c2a61541480c23e1 |
| #1921 | M2 | merged | c53be1dc9c4c6a1770519b31489a693cc8c316ed |
| close | Close (spec Closed, analyze fixes, TDD verification) | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T020–T022 | full | Stable worktree ids and parent links in per-project state; create_worktree records the parent; list_worktrees/list_sessions/get_session report it; removed tombstone; user guide | #1873 | merged |
| M1b | T030–T032 | full | One visibility filter: a caller outside default sees and addresses only own worktree, parent, siblings and children (FR-015), names only those as `parent` (FR-016), relays through the parent (FR-017); user guide | #1912 | merged |
| M2 | T015–T019 | full | Sidebar parent label (current name, marked removed) and a parent filter that narrows and composes with the issue/type filter; no nesting; shared chip; user guide; visual pass of quickstart Part B | #1921 | merged |
| M3 | T023, T024, T033, T025, T026 | light | Supervisor ownership: `runs-of-parent.sh` with fixture test; ownership and relay rules in the supervisor skill; `Parent` ledger column; issue notes | #1917 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | milestones | Where do the new daemon tests live? | In `crates/micold-daemon/tests/mcp/` as module `mcp_worktree_parent` (one binary, #1175), not new top-level files; catalog tests are in the same module | agent-resolved | tests/mcp/main.rs |
| D2 | milestones | Pure id/link rules | In `micold-core/src/worktree_parent.rs` (unit-tested), applied by `Catalog` | agent-resolved | plan.md |
| D3 | milestones | Relocation test (T011) | No relocation fixture exists for per-project state (state is keyed by project path); covered by the store round trip and daemon restart tests | agent-resolved | store.rs `project_state_path` |
| D4 | milestones | Where does T030's test live? | `crates/micold-daemon/tests/mcp/mcp_worktree_parent_visibility.rs`, a module of the `mcp` binary (D1), not the top-level file T030 names; tasks.md updated | agent-resolved | tests/mcp/main.rs |
| D5 | milestones | `default` as the target of `rename_worktree`/`delete_worktree` from a restricted caller | Stays `invalid_input` (argument validation in core, before any scope; reveals nothing); all other out-of-scope targets are `not_found` | agent-resolved | core mcp/tools.rs `not_default` |
| D6 | milestones | Relay test (FR-017) | A message to the parent passes the scope check (fails only with `conflict`, no running process in the fixture); the typed-in path is covered by mcp_cross_session | agent-resolved | mcp_worktree_parent_visibility.rs |
| D7 | milestones | Existing tests with a worktree caller listing unrelated worktrees | Fixtures changed: callers moved to the `default` session, or parent links added; behaviour under test unchanged | agent-resolved | mcp_read_tools.rs, mcp_lifecycle_tools.rs |
| D8 | milestones | The parent filter and the existing OR filters | Parent chips are a second dimension: a worktree must be a child of ANY chosen parent AND match ANY chosen type/issue/untyped chip (when any is on); this is what "composes" and "narrows" mean. Documented in the user guide | agent-resolved | features/sidebar.rs `matches_filters`, tests/sidebar_parent.rs |
| D9 | milestones | Filter key and label | `ParentLabel { name, removed }`, name being the parent's display name (the daemon resolves rename overrides); the filter keys on it, so a removed parent is its own chip. Wire: `WorktreeSnapshot.parent`, PROTOCOL_VERSION 82. `TagFilter` lost `Copy` since it carries the label | agent-resolved | core worktree.rs, protocol/version.rs |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| A | 1 | origin/main...HEAD | 11d635f5 | d7c7f1d4eb34:11d635f5ac23 | 9 findings, none fixed (declined, see below; F1/F2 are real narrow races) |
| B | 1 | origin/main...HEAD | 11d635f5 | d7c7f1d4eb34:11d635f5ac23 | CHANGES: T011 no test (task text updated, drift-M1.md), T004/T007 path drift (drift-M1.md), mutant evidence (daemon mutants added to cycle log) |
| A | 1 | M1 | 337f121e09c146f3f3eaf9a369088dabb4bcffd4 | 6575cda573337d5cac492ec5d86db857ba0283ff:337f121e09c146f3f3eaf9a369088dabb4bcffd4 | MINOR-ONLY after triage (findings declined with reasons, F1/F2 follow-up) |
| B | 1 | M1 | 337f121e09c146f3f3eaf9a369088dabb4bcffd4 | 6575cda573337d5cac492ec5d86db857ba0283ff:337f121e09c146f3f3eaf9a369088dabb4bcffd4 | CHANGES fixed (docs, drift record, daemon mutants, test fixes); not re-run |
| B | 2 | M1 | 73605a1b51507be22bd169b6576566de44db6462 | b198666fb3c888f7c13fa1797ce5011f87c6f28c:73605a1b51507be22bd169b6576566de44db6462 | CLEAN (round 2 sonnet on fix diff; daemon worktree_parent 13 passed) |
| A | 2 | M1 | 73605a1b51507be22bd169b6576566de44db6462 | b198666fb3c888f7c13fa1797ce5011f87c6f28c:73605a1b51507be22bd169b6576566de44db6462 | MINOR-ONLY (round 1 findings declined; no code change since, only rebase onto main with the #1863 daemon-lookup fix) |

| A | 1 | M1b | 7318d57 | (code-review skill, high, no subagent tool) | 7 findings: list_resumable_sessions and list_branches scoped, scope computed in the blocking closure, redundant retain removed, `Scope::unrestricted()`, list_branches test added; 2 declined |
| B | 1 | M1b | f8a65aedc8a930ce6de6b90bf29bc8e2b6c8c5b7 | 091e3648b65b331889996323fab008ad31a1ba0d:9949424349adb1a57a7b6ce87a8d75e9634a9bb8 | CHANGES (F1-F4): all fixed (cycle-log red runs, drift-M1b.md, relay test states coverage, Scope::explain with red-first tests); round 2 to run by orchestrator |
| B | 2 | M1b | 24e28dc124ca0b39051ae6b826275be5f3d80fb7 | 2b7105f8e940df91f685cfc09e6da5c31dfa8fce:24e28dc124ca0b39051ae6b826275be5f3d80fb7 | CLEAN (round 2 sonnet on fix diff; worktree_parent 22 passed; 2 MINOR) |
| A | 1 | M3 | 06e798571 | 1036173b0df044edbd6d9e5688f8d09e586f9e31:06e79857162a63dd6971f69df67a16423d6cbfc8 | CHANGES (F1-F6): F1 `ref` not `name`, F2 orphan scan on unfiltered list, F3 `--check` takes ref or path, F4 exit 2 on empty or bad rows, F5 8-column ledger refuses Parent, F6 relay text; all fixed test first (cycle-log M3); round 2 to run by orchestrator |
| B | 1 | M3 | 06e798571 | 1036173b0df044edbd6d9e5688f8d09e586f9e31:06e79857162a63dd6971f69df67a16423d6cbfc8 | CHANGES (F1-F4): F1 cycle-log M3 section with red runs, F2 = A F6, F3 T026 reworded to comments, F4 drift-M3.md; all fixed; round 2 to run by orchestrator |
| A | 2 | M3 | b2b6e0af3d1535e299bfdec02e9cfca59968ae14 | 4aed8e5fc120550f00e3254472dd2fbadfc597d2:b2b6e0af3d1535e299bfdec02e9cfca59968ae14 | CLEAN (round 2 sonnet on fix diff; ownership 39, supervisor 269 passed) |
| B | 2 | M3 | b2b6e0af3d1535e299bfdec02e9cfca59968ae14 | 4aed8e5fc120550f00e3254472dd2fbadfc597d2:b2b6e0af3d1535e299bfdec02e9cfca59968ae14 | CLEAN (round 2 sonnet on fix diff) |
| A | 1 | M2 | afd98b48f3ac0d200e992af167138e8e3330c0dc | by-hand review of origin/main...HEAD (no subagent tool) | CLEAN no BLOCKER or MAJOR |
| B | 1 | M2 | 68d2fd0f5 | conformance review of origin/main...HEAD | CHANGES (F1-F4) fixed: cycle-log M2 red run and two mutants (both killed), drift-M2.md and a `## Milestones` section in tasks.md, RunRow boxed instead of the allow, renamed-parent note in the user guide |
| B | 2 | M2 | e05d5e719 | fix diff 68d2fd0f5..e05d5e719 (sonnet) | CLEAN: F1-F4 resolved; plan-drift.sh M2 no drift; clippy client clean. Visual pass T018 PASS (both Part B checks, screenshots in project files issue-1859/) |

| Close | by hand | close diff (specs only) | working tree | no subagent tool in this unit; analyze C1/I1/I3/A1/T1/T2 fixed in spec, plan, tasks; docs only |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | record after create leaves a window with no parent (F1) | Design in plan.md records after the create returns; a window of milliseconds; passing parent into the create path is a follow-up |
| M1 | A | record retires a refresh-minted id, a concurrent child of the same parent turns removed (F2) | Same window; needs a concurrent create of a child within it; follow-up with F1 |
| M1 | A | caller's worktree missing from the cached snapshot gives no parent (F3) | Spec'd: an unresolvable caller creates an unparented worktree (test `an_unresolvable_caller_creates_an_unparented_worktree`) |
| M1 | A | store save does not merge on-disk ids/links (F4) | Only the daemon saves the workspace with these maps; follow-up if a client writer appears |
| M1 | A | transient absence tombstones children (F7) | The documented R3 design; guarded by directory-exists check |
| M1b | A | `list_worktrees` `parent` filter is not scope-checked | An out-of-scope name returns an empty list, the same as an unknown name (documented); no leak |
| M1b | A | a worktree literally named `default` collides with the root ref | Directory names are produced by `checked_dir_name`/slugs; the collision already breaks `WorktreeRef` parsing before this change |
| M1 | A | perf of views per read, clone in refresh, parent_choice ordering, 8-arg fn (F5, F6, F8, F9) | MINOR, taste |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- TDD verification verdict FAIL (test-after, `tdd/verification.md`): T035-T041 in tasks.md Phase 9 (message-delivery assertion, explicit `parent: default`, MCP delete tombstones children, FR-003/005/012 and relocation tests, four surviving mutants, sidebar rename test, supervisor-ownership word checks). Test-strength only, no new behaviour.
- Spec-analysis items left: U1 (scope of a child whose parent is removed, unresolvable caller), G1 (no story/SC for visibility), G2 (no FR-012 test, no relocation fixture).

- Tests that exercise tool behaviour use `full_view_credential` (tests/support/mcp.rs) or default callers; scope handling per tool is covered by `mcp_worktree_parent_visibility` only.
- Record the parent inside the create path (review A F1/F2): closes the window where a new worktree is visible without its link and a concurrent child of it can be tombstoned.
- An active parent filter whose parent is renamed keys on the old name and matches nothing until cleared (Clear filters in the empty message); same edge as any filter whose tag vanishes.
- Verification run here: fmt, clippy core+workspace (-D warnings), core `--all-targets` (only 4 root-permission tests fail, unrelated), client lib + main + features_sidebar, daemon mcp worktree_parent. Full gate left to CI.
