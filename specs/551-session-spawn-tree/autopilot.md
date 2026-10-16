# Autopilot ledger — #551 session-spawn-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Show sessions spawned by an agent as a tree under their origin session (issue #551); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #551
- **Worktree branch**: feat/551_show-sessions-spawned-by-an-agent-as-a-tree-under
- **Started**: 2026-10-12
- **Phase**: milestone-M2
- **Next step**: M2 reviews, full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #787 | Design PR | merged | 8e1cdcce24199789f214b1ae153a9199863cb2e4 |
| #824 | M1 | merged | 6f578693808230b11552a7a47db3bc7214a40e84 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016, T025, T038 | full | Origin recorded by the daemon's `record_origin` (nothing sets it until M3), persisted, exposed on wire and MCP; spike T025 outcome recorded | #824 | merged |
| M2 | T017–T024, T039 | full | Sidebar nests children with child/pending/removed markers | | pending |
| M3 | T026–T031, T040 | full | Provider readers for spike-confirmed providers, discovery, provider table | | pending |
| M4 | T032–T037, T041 | full | Fold, count, attention rollup, persisted fold | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Fold state persisted across restarts? | Persisted; never-folded origin starts expanded (FR-017) | agent | specs/017-material-component-architecture/spec.md#FR-016 |
| 2 | clarify | Children of a stopped or deleted origin? | Stopped: stay nested. Deleted: top-level "origin removed", never deleted (FR-018) | agent | spec.md FR-006, FR-008, US2.3 |
| 3 | clarify | MCP create_session child of caller? | Yes, same worktree only (FR-019); SUPERSEDED by 5 and 7 | agent | specs/034-daemon-mcp-server/contracts/binding.md#2, crates/micold-daemon/src/mcp/tools.rs |
| 4 | tasks | Force-with-lease push of the rebased feature branch (denied by classifier) | Allowed; pushed by the orchestrator | user | AskUserQuestion; origin at f68aa511 |
| 5 | spec/plan | Feature 776 makes a worktree hold one session and refuses a second `create_session`; what is a child now? | "There should be one user session created. The sesssion created by claude agents should be listed as one level in tree deeper" | user | AskUserQuestion; specs/776-one-session-per-worktree/spec.md FR-001/FR-002; spec.md Clarifications 2026-10-13 |
| 6 | spec/plan | Make the one-session rule soft (confirm, then run in parallel)? | "you can record that as another github issue" (issue #802; out of scope, 776's refusal unchanged) | user | AskUserQuestion; issue #802 |
| 7 | plan | Where does an agent-spawned session's origin come from under 776? | Only what its provider reports (readers in M3); tool server records none (FR-019 amended); OpenCode and Codex readers on synthetic fixtures built from the provider's schema or source; Claude Code stays a documented gap | agent | research.md R2, R5; contracts/provider-signals.md; repo has no other signal (hooks are per-session, `--settings` file) |
| 8 | implement | Allow a fourth part of M1 (rebase, re-review, push, PR) | Allowed (decided by user) | user | orchestrator prompt: M1 part 4 |
| 9 | plan | Claude Code `claude --bg` has no recorded parent signal; what then? | Document Claude Code as a provider gap; no user-supplied record (decided by user) | user | orchestrator prompt; research.md R2; Handover open question |
| D-part5 | M1 | Allow a fifth M1 part to merge main again and re-gate (PR #824 CONFLICTING) | Allowed | user | orchestrator prompt |
| D-part5b | M1 | Linearize the branch (wait-merge: cannot rebase merge commits) | Rebase and force-with-lease push allowed; the user ran the rebase and squash themselves; protocol 47 (main took 46 for feature 552) | decided by user | orchestrator prompt |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 551 | 1 | b598212212983763962acb3bdf6d453c5d13095c:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 1 MAJOR (F1 origin pending vs removed, fixed: FR-016), 3 MINOR fixed |
| Spec 551 | 2 | 72380bb32aa8cae88205f35b4fcae1ce1459a190:13165b3a9e4938a42528aae5df288475aa63ddae | CLEAN (2 MINOR fixed, prose only) |
| Plan 551 | 1 | d39d189620904732240bb92b8325ded71e3e99d0:66c580d0da56f9eb86256315e6aec379967b5a15 | CHANGES: 2 MAJOR (F1 child removal emit, F2 fold prune), 4 MINOR; all fixed |
| Plan 551 | 2 | d3b3e472cd18ecf87fe544cdb57b74a377bf70c4:66c580d0da56f9eb86256315e6aec379967b5a15 | CHANGES: 1 MAJOR (single-delete path missing, fixed; prose only, paths verified by grep), 2 MINOR fixed |
| Tasks 551 | 1 | e04f39460adaad4619f835ff528eb2909e11e215:f602c37c08334f421b8583446a4439c4bde3f094 | CHANGES: 2 MAJOR (F1 U4 test placement, F2 unknown-candidate case), 3 MINOR; all fixed |
| Tasks 551 | 2 | 53e6be0a87d9891999753144cad487cb7714f632:00b47e5aab03d843624242c92786bfd6cfac0b61 | CLEAN (2 MINOR: R4/data-model signature fixed, prose only; fold steps 3-4 are in M4 Verify, declined) |
| Code A M1 551 | 1 | 0849fd1ec639484525393da0e74f885a01b8150a:4f46048b8c795ca1cd6190e8268158cae396c4b7 | CLEAN (1 MINOR fixed: queue removed events after persist) |
| Code A M2 551 | 1 | fe9b22b40979eeead557479411a0a381e6b14760:e64d3ead2296bc91006e592335d1d272b59ba7a5 | 0 BLOCKER/MAJOR upheld; indent clamp fixed |
| Code B M2 551 | 1 | fe9b22b40979eeead557479411a0a381e6b14760:e64d3ead2296bc91006e592335d1d272b59ba7a5 | CHANGES: 1 MAJOR (no FR-007 message test; fixed: a_child_row_acts_like_a_top_level_row), 1 MINOR fixed; visual pass found the client never copied `origin` from the catalog (fixed in catalog_sync, tested) |
| Code B M1 551 | 1 | 0849fd1ec639484525393da0e74f885a01b8150a:4f46048b8c795ca1cd6190e8268158cae396c4b7 | CHANGES: 1 MAJOR (no red-phase output; recorded honestly in cycle-log, red by absence of symbols; not re-run), 2 MINOR noted in cycle-log |
| Amendment 551 (spec, plan, tasks) | 3 | a68e498d5b9c43aaea5c6bc312397d7d3a0cea7f:64875b6b7a9a5fa719e5c3448f02c144d22709dd | CHANGES: 3 MAJOR (F1 quickstart seeding, F2 M1 Verify fixtures/T025, F3 one-level vs depth), 2 MINOR; all fixed in the next commit, not re-reviewed |
| Amendment fix 551 | 4 (sonnet) | a68e498d5b9c43aaea5c6bc312397d7d3a0cea7f:64875b6b7a9a5fa719e5c3448f02c144d22709dd | CHANGES: 1 MAJOR (quickstart seeded projects.json, not the project state file), 1 MINOR; fixed (wording of seeding instructions and ledger row, prose only); round 3 F1-F3 confirmed fixed |
| Code A M1 551 (rebased) | 2 (sonnet) | 45c07d7d1709df3cfdffcb90075eeb6db86ba71e:35a170e47eea873fd8569b79ef57ab11de1f4baa | CHANGES: 2 MAJOR (F1 removed-child event lacked live overlay, F2 dead worktree_ref_of), 1 MINOR (F3 origin rollback on persist failure); all fixed in 35a170e4 |
| Code B M1 551 (rebased) | 2 (sonnet) | 45c07d7d1709df3cfdffcb90075eeb6db86ba71e:35a170e47eea873fd8569b79ef57ab11de1f4baa | CHANGES: 1 BLOCKER (4 test call sites kept a dropped 4th create_session argument; fixed), 1 MAJOR (Verify unobserved: lock held 15 min; the full gate's cargo test --workspace covers it), 1 MINOR (protocol 42 in artifacts, fixed); no re-run of B for the fixes |

## Declined review findings

- A M2 (1 high): archived origin shown as pending - M1's daemon overlay marks an archived origin `removed` (session_origin_removal), so the client never sees `known` for one; cycle-cut member without marker - it is a root by spec, only corrupt data reaches it; main session with an origin - one user session per worktree, the main has none; `agent, ` prefix split between files, 7-argument item, duplicated wrappers, O(n^2) lookups - MINOR taste, not changed. Fixed: indent clamped at 4 levels (MAX_NESTING).
- B F1 (M1, MAJOR): per-behaviour red output not recoverable from a single WIP commit; accepted with the by-absence-of-symbols explanation in tdd/cycle-log.md.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Done: M2 implemented (T017-T024, T039 ticked), committed on the branch (3 commits, rebased on origin/main). Review A (0 upheld) and B (fixed) done and recorded; scoped gate green. Visual pass 1 found the client never copied `origin` (fixed in catalog_sync.rs with a test); the pass has NOT been re-run on the fixed tree (showcase pose "sessions started by an agent", then the client with a seeded project state file).
Full `mise run gate` on the rebased tree: red on one test, `tests::pr_status::pr_status_tick_and_refreshes_during_a_reading_add_one_call_when_it_ends` (main_tests.rs:3119, unrelated to this diff, timing-looking). Next: re-run `mise run gate` (detached, hold.sh); if that test fails again alone, run it on origin/main to see if it is a flake and report; then optional visual-pass re-run via autopilot-worker, rebase (branch-start.sh 824), push, open the PR per tasks/pr.md (title `feat(551): show agent-started sessions as a tree in the sidebar (#551)`, body from pr.md template, Review A: 10 findings, none upheld, indent clamp fixed; Review B: 1 MAJOR fixed). No PR open yet.

## Open escalation

None.

## Follow-ups not done

Companion issue #552 (replace name-based hiding with native hooks) is out of scope.

Tasks unit: speckit-analyze fixed before review: spike moved to M1 (T025), `origin_rejected` tombstone added (data-model, T005, T030), FR-011 count pinned to direct children, spec Assumptions and Clarification 1 corrected to research R2/R6. Polish (quickstart Part B, ticks) left to the close unit.
