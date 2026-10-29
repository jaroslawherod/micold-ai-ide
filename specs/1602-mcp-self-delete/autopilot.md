# Autopilot ledger — #1602 mcp-self-delete [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1602 (issue #1602: MCP: let a session stop itself and delete its own worktree, after confirmation); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1602
- **Worktree branch**: feat/1602_mcp-let-a-session-stop-itself-and-delete-its-own
- **Started**: 2026-10-29
- **Phase**: tasks
- **Next step**: Design PR merges; then milestone M1.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1611 | Design | merged | 8e3d084ba79f31d3db24ed3bf6e46b74590b087d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| C1 | clarify | Story 4 (stop/delete self) in scope? | Yes | user | spec.md Clarifications |
| C2 | clarify | Failed deferred removal reported how? | App notice naming W and reason, plus daemon log | user | spec.md Clarifications |
| M1 | T001–T016 | full | Own-worktree delete after confirmation (incl. wire bump 74, open-handle test) | #… | pending |
| M2 | T017–T025 | full | Delete prompt lists uncommitted files and unpushed commits | #… | pending |
| M3 | T026–T030 | full | Session stops or deletes only itself | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | d0f9fbb3c51a39e109610a93625505f8ad347b7d:2d2d95f3fae526ce5fcff1f4f34c63d838d9004c | CLEAN (3 MINOR, fixed, prose only) |
| Tasks review | 1 | ea532f26d9ac361d50a41e909238a78bbe29234d:c73cbe1c7de743d075350c08c55a3ca9392d718b | CLEAN (3 MINOR fixed, prose only: T007 [P], M1 Verify step 7, T003 description test) |
| Plan review | 2 (scoped, sonnet) | d39a7dfcbc667917467e82b2b58c34ddfb9b6c65:8e4b6b4b0c4feab124dfc49f3c7d767bd6fcb81e | CLEAN (1 MINOR fixed, prose only: policy test path) |
| Plan review | 1 | 0b78d32b342988dd1f35378088cb36a84b3aa567:9dff441a2bbac0be6f7d913c2c0e5dc53484cc7b | CHANGES (3 MAJOR, 3 MINOR, all fixed, prose only: test rows, until_answered plumbing note, R6 wording) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 implement step, unit 1 (context over 150k). Branch reset to origin/main after #1611 (merge SHA 8e3d084ba79f31d3db24ed3bf6e46b74590b087d). WIP committed locally, nothing pushed, no PR, no review run, no task ticked.

Done in code (all compiles with `cargo check --workspace --all-targets`): wire (`LossSummary`, `LossCommit`, `loss` + `self_targeted` on `ConfirmationRequested`, `DeferredActionFailed`, version 74, tests pin 74, roundtrip cases); `policy.rs` (own-worktree refusal dropped, `is_self_targeted`); `core/mcp/tools.rs` description; daemon `confirm.rs`, `tools.rs` (`Deferred`, `call` returns `(Result, Option<Deferred>)`, own delete returns `scheduled`), `server.rs` (generic `until_answered`, `DELIVERY_GRACE` 2 s, shutdown + peer close then spawn `Deferred::run`); client (`Prompt.self_targeted`, `self_targeted_note`, dialog line, `DeferredActionFailed` to `notify_error`); fixtures updated.
Tests written: `mcp_lifecycle_tools.rs` (11 new, all 35 green incl. T007 open-handle, reply-first, hangup, failed deferred), `mcp_policy.rs`, `features_agent_confirm.rs`, client unit test in `daemon_sync.rs` (`a_deferred_action_failure_raises_a_notice...`, not yet seen to run: it sits in the bin target, run `cargo test -p micold-client --bin <bin> a_deferred_action` or find the right target).

Open, next steps:
1. `mcp_audit_log.rs`: new test `an_accepted_self_delete_is_audited...` fails at line ~540, and existing `a_prompt_never_reaches_the_log` (line ~420) now fails: likely its delete_worktree/own-worktree call became a self-delete (accepted, scheduled) or the 0o500 lock leaked across tests; diagnose with `cargo test -p micold-daemon --test mcp_audit_log`. `good_call` uses worktree `c` for delete_worktree; `failing_call` own-worktree entry was removed.
2. `mcp_policy.rs` description test: fixed by removing the "cannot be deleted;" assert; re-run `cargo test -p micold-core --test mcp_policy`.
3. T005 deviation: reply-before-stop, 2 s grace, hangup, failed-deferred tests live in `mcp_lifecycle_tools.rs` (the Fixture is there), not `mcp_confirmations.rs`. T006 decline audit line is the generic refusal line, covered by existing tests; accept + deferred `error` line in `mcp_audit_log.rs`.
4. T016 docs not done: `docs/user-guide/agent-tools.md`; amend `specs/034-daemon-mcp-server/spec.md` (FR-014, FR-015) and `contracts/mcp-tools.md` for the delete case. Also `grep -rn "own worktree" docs` for stale statements.
5. Remaining: tick T001–T014 and T016 in tasks.md (T015 only when Windows CI of the M1 PR is green; extend `ops.rs` retry only if T007 fails there), `mise run gate`, review A (`code-review` high) then review B (conformance) per tasks/verify.md, visual pass for the dialog line, PR via tasks/pr.md. No red-phase evidence was recorded (tests written beside the code): note that in the PR body.
6. Orchestrator: check the Windows job on M1's PR for T007/T015.

## Open escalation

None.

## Follow-ups not done

- M1 has 16 tasks (over the 10-task guide): it is Foundational (wire bump) plus two P1 stories sharing `decide` and `Deferred`; no acceptance-scenario split gives a observable deliverable. Accepted.
- R6: T007 open-handle test; T015 extends the `ops.rs` retry bound only if the Windows CI job of M1's PR fails. Orchestrator: check the Windows job on M1.
- Spec FR ids renamed: local FR-014a to FR-017, FR-015a to FR-016 (034's FR-015a keeps its name).
