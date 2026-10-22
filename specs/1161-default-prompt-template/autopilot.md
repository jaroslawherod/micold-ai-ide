# Autopilot ledger — #1161 default-prompt-template

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1161 Default prompt template in settings for a new worktree's initial session prompt; labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1161
- **Worktree branch**: feat/1161_default-prompt-template-in-settings-for-a-new
- **Started**: 2026-10-20
- **Phase**: milestone M3
- **Next step**: M3 PR CI and merge, then the close unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1163 | Design | merged | 7406e4cdc61cbf75bc88c92a2bee08913b6bdf71 |
| #1245 | M1 | merged | d16d74c143b73ad6af5d341e32872e3711ce8dc6 |
| #1247 | M2 | merged | 420b051e5b3be4610a19347b44f37099bed72ccb |
| #1251 | M3 | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Template in Settings, pre-fills the dialog's first prompt | #1245 | merged |
| M2 | T015–T019 | light | Validation in Settings; bad stored template renders empty | #1247 | merged |
| M3 | T020–T024 | full | MCP create_session/create_worktree render the template | #1251 | in review |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | create_worktree with neither ai_cli nor prompt starts a session when a template exists? | No | agent-resolved | spec.md FR-009 |

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

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
