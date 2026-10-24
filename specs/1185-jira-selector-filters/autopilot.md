# Autopilot ledger — #1185 jira-selector-filters

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1185 (issue #1185: Filter the Jira issue selector by reporter, label, component and fix version); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1185
- **Worktree branch**: claude/project-thread-xwkotj
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T022 | full | Component filter + filter mechanism (row, active line, Clear filters) | — | pending |
| M2 | T023–T030 | full | Fix-version filter, AND with component | — | pending |
| M3 | T031–T039 | full | Reporter filter (own user, typed people search, by account) | — | pending |
| M4 | T040–T045 | full | Label filter from loaded issues; all four in one row | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Overlap with #1186 (advanced JQL in the same selector, worked by another thread in parallel)? | This spec covers only the four basic filters (reporter, label, component, fix version); free-form/advanced JQL is listed out of scope and left to #1186. Both touch `search_jql`/`list_jql` in crates/micold-core/src/jira.rs and the Jira picker in worktree_form.rs: expect a merge-order rebase. | agent-resolved | spec.md#Out of Scope; orchestrator prompt |
| D2 | spec | Cloud environment: `gh` unauthenticated, `mise` missing | Skipped scripts/autopilot/branch-start.sh (needs gh); branch already at origin/main 0e5a1e9b. Gate is scripts/gate.sh. | agent-resolved | orchestrator prompt |
| D3 | clarify | Which people does the reporter filter offer? | Own user first, then project users with browse permission, searched by typed name | agent-resolved | issue #1185 body ("its … users"); spec.md#FR-014 |
| D4 | clarify | Where do label choices come from? | Labels on the not-Done issues of the unfiltered listing (≤1,000), alphabetical, no extra request; notice when the listing is capped | agent-resolved | issue #1185 body ("from the connected Jira project"); Jira `GET /rest/api/3/label` is site-wide only |
| D5 | clarify | Label choices once a filter is chosen? | Taken at the unfiltered load, kept for the dialog | agent-resolved | spec.md#FR-005, #FR-007 |
| D6 | plan | How do the filter clauses compose with #1186's JQL work? | Separate `micold_core::jira_filter` (`clauses`, `narrowed`); `list_jql`/`search_jql`/`list_open`/`search_open` unchanged; additive `JiraSource` methods with defaults | agent-resolved | research.md#R1, #R10 |
| D7 | plan | Filter controls in the 520 px, tallest dialog? | One row of four `Typeahead`s (Any row clears), one "Filtered by … / Clear filters" line while active; no new widget | agent-resolved | research.md#R8; SC-001 |
| D8 | tasks | US1 split? M1 over the ~800-line guide | Split US1 along AS3: M1 = mechanism + component, M2 = fix version; the mechanism has no smaller observable slice. T046 (quickstart, no code) left to the close unit | agent-resolved | tasks.md#Notes; milestones.md rule 3, 4 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | faf2dbc7e88487e7ed788cc777e08ac3a01f4774:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CHANGES: 1 MAJOR (FR-010 trim/cut vs exact match), 2 MINOR (SC-003 wording, Debug redaction + me-lookup failure); all fixed |
| Spec review | 2 | 2450b706dff7ee9368b016e9a03188feaffe296b:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CLEAN (2 MINOR: FR-006 Debug wording, SC-003 control-char clause; fixed, prose only) |
| Plan review | 1 | 82673589e84328975ec6437b6506b889ef2b6dcd:93883c9e001cea6910868956ff2d695bd1be1984 | CHANGES: 1 MAJOR (search_jql edited or not: contradiction); fixed, search_jql untouched |
| Plan review | 2 | 8caad6fb6b5b539a182b4c5d336b8e96b8a4debe:93883c9e001cea6910868956ff2d695bd1be1984 | CLEAN (sonnet) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
