# Autopilot ledger — #1185 jira-selector-filters

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1185 (issue #1185: Filter the Jira issue selector by reporter, label, component and fix version); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1185
- **Worktree branch**: claude/project-thread-xwkotj
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: clarify unit: resolve the two [NEEDS CLARIFICATION] markers in spec.md (reporter choices, label choices)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Overlap with #1186 (advanced JQL in the same selector, worked by another thread in parallel)? | This spec covers only the four basic filters (reporter, label, component, fix version); free-form/advanced JQL is listed out of scope and left to #1186. Both touch `search_jql`/`list_jql` in crates/micold-core/src/jira.rs and the Jira picker in worktree_form.rs: expect a merge-order rebase. | agent-resolved | spec.md#Out of Scope; orchestrator prompt |
| D2 | spec | Cloud environment: `gh` unauthenticated, `mise` missing | Skipped scripts/autopilot/branch-start.sh (needs gh); branch already at origin/main 0e5a1e9b. Gate is scripts/gate.sh. | agent-resolved | orchestrator prompt |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | faf2dbc7e88487e7ed788cc777e08ac3a01f4774:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CHANGES: 1 MAJOR (FR-010 trim/cut vs exact match), 2 MINOR (SC-003 wording, Debug redaction + me-lookup failure); all fixed |
| Spec review | 2 | 2450b706dff7ee9368b016e9a03188feaffe296b:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CLEAN (2 MINOR: FR-006 Debug wording, SC-003 control-char clause; fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
