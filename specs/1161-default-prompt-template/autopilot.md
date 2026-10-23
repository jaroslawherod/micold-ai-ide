# Autopilot ledger — #1161 default-prompt-template

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1161 Default prompt template in settings for a new worktree's initial session prompt; labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1161
- **Worktree branch**: feat/1161_default-prompt-template-in-settings-for-a-new
- **Started**: 2026-10-20
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1163 | Design | merged | 7406e4cdc61cbf75bc88c92a2bee08913b6bdf71 |
| #1245 | M1 | merged | d16d74c143b73ad6af5d341e32872e3711ce8dc6 |
| #1247 | M2 | merged | 420b051e5b3be4610a19347b44f37099bed72ccb |
| #1251 | M3 | merged | 169ef801b357b40a4284f785d500e25916e18afc |
| #1284 | Close | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Template in Settings, pre-fills the dialog's first prompt | #1245 | merged |
| M2 | T015–T019 | light | Validation in Settings; bad stored template renders empty | #1247 | merged |
| M3 | T020–T024 | full | MCP create_session/create_worktree render the template | #1251 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | create_worktree with neither ai_cli nor prompt starts a session when a template exists? | No | agent-resolved | spec.md FR-009 |
| D2 | close | /home 100% full blocked the gate: free disk or approve SWEEP_FORCE=1? | Disk freed (456G free); no force needed | user | `df -h /home` 454G free |
| D3 | close | Build lock held 11h+ by a feat-1150 gate: kill it or bypass the lock? | No action; that run is not hung and the queue drained | user | coordinator relay 2026-10-23; ps shows queue moving |
| D4 | close | tdd-verify FAIL: no recorded red for M1-M3 (finding 1, unrecoverable); waive? | Waived as a documented gap: tests exist and pass, two mutants caught; test-first order cannot be recovered | agent-resolved | tdd/verification.md finding 1; tdd/test-list.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 2da4e91e074c34b2608429055921e3a617527a46:d9c2e378c1ef6e6a8f142a5369dd16c856e9324c | CHANGES: 3 MAJOR |
| Spec | 2 | 3399ba1e782776dc6bc887373536e91692f57891:e62f06e9aef3c487cbfc825ed1ce3e1a9e3a7437 | CLEAN (3 MINOR fixed, prose only) |
| Plan | 1 | 9c1a3ea4c4eafd8fac474ff9036262b2110d35ce:3f9fcba65f32e6fe61a0c04eb2b35443fbcb36f1 | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Plan | 2 | 67a3b302e232e0f97f90ca37f8ed269e5ed8e9c9:4bbf066bfe6165d2de73d7111a10784a402091ba | CLEAN |
| Tasks | 1 | 15866164d7f79a3d92d85e3fb046d236eedcf7c4:c4edd43e6fdac03c072a44bd4b8263fdc3df7b3b | CLEAN (2 MINOR fixed, prose only) |
| M2 Review A (high) | 1 | 149299858bf5c303889102a10e028a8747bb9b00:81aedcc51d63de8b9765c5fd8f52f682003d4162 | CLEAN (1 MINOR declined: cursor moves clear the error) |
| M2 Review B (sonnet) | 1 | same | CLEAN (doc wrap MINOR fixed; T016 is a pin, M1 already rendered empty) |
| M3 Review A (high) | 1 | 21739abdcec9137717faf4241050e429726a6132:802cec8ae6b7e1a908d844753c4e0f1382d2bffc | CLEAN (3 MINOR: F1 fixed, F2/F3 declined) |
| M3 Review B (sonnet) | 1 | same | CLEAN (1 MINOR fixed: name_words unit test; Verify: 30+42+8 tests ok) |
| M1 Review A (high) | 1 | ba211cda3dc12bd3d30cd91de2d6f81058dc21c2:b3829c225724e3d598cfabc541fe1660f5ce2ec0 | CHANGES: 1 MAJOR (body not in description pass; fixed), F2 fixed, F3 declined |
| M1 Review A (high) | 2 (full, stale snapshot after rebase) | 02e4fc1536c51d3b8070bbceff63dbcbc0ced9c5:bdf034b4716832911ac413339ce3eefff4777286 | CLEAN (1 MINOR: validation is M2) |
| M1 Review B (conformance) | 1 | same | CLEAN (MINOR: visual evidence lacks picked-issue shot; reducer tests cover US1 s1) |
| Close diff review (sonnet) | 1 | 1bde3b936669bc536ad53078cf1cfc6b28805572:8fda5012e52c64e4b1dac7230edb806ea072d7ea | CHANGES: F1 MAJOR "tests never run" was stale (reviewer saw wip commit): ran since, features_worktree_form 36 ok, shell test ok, mcp_default_prompt 11 ok, lib ok; mutations caught (refresh_default_prompt at worktree_form.rs:1221, github_issues gate in ops.rs). F2 MINOR passes. Docs reworded; no new round (prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A | F2 ops.rs repeats the remote-list/choose_remote read of the github_issue path | MINOR, left: two short call sites, no shared type worth adding |
| M3 | A | F3 lookup gated by github_issues_enabled | Intended: 550 FR-010 / 1102, nothing leaves the machine with GitHub issues off |
| M1 | A | F3 `{issue}` is the ticket text even with no picked issue | Intended (plan: issue = ticket.trim(); a Jira ticket has no picked GitHub issue) |

## Notes

- M3 deviation from plan wording: `{name}` over MCP is `naming::name_words(dir)` (the slugged descriptive words, lower case, new helper), not `display_name`, so the T002 shared fixture (`name: crash`) renders identically in the dialog and over MCP (FR-011). The Jira privacy test `jira_is_requested_only_on_named_events` mentions no `IssueSnapshot`, so T020 changed no Jira test.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- `git_credential_grant` fails locally only, under the global `url.insteadOf` rewrite (seen in M3); not fixed, outside this flow.
- Quickstart §B step 4 (MCP, live) not run by eye: covered by `mcp_default_prompt.rs` (real PTY, stand-in CLI); steps 1-3 have visual/c3,c5,c11.
- tdd/verification.md (second audit, FAIL on the unrecoverable test-first gap, waived as D4). Not done: T031 (run six unmeasured deliberate mutants: blank-prompt check, first_prompt_edited guard, name_words, settings-to-form path) and T032 (split the eager listing/pick/body test and use assert_eq!); both are test-strength polish, recorded unmeasured.
