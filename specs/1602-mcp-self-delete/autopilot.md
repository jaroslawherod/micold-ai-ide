# Autopilot ledger — #1602 mcp-self-delete [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1602 (issue #1602: MCP: let a session stop itself and delete its own worktree, after confirmation); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1602
- **Worktree branch**: feat/1602_mcp-let-a-session-stop-itself-and-delete-its-own
- **Started**: 2026-10-29
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1611 | Design | merged | 8e3d084ba79f31d3db24ed3bf6e46b74590b087d |
| #1613 | M1 | merged (after red-CI attempt 1, see Follow-ups) | 4edaaa6af88c38edd080d3351da4e7fcfa3d655e |
| #1619 | M2 | merged | c5f574866db5b3fa94c867b526305caa8505a67f |
| #1621 | M3 | merged | 93ce38cfb0a1bb95fd2c7d471ab780b80e8e135d |
| #1622 | Close | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| C1 | clarify | Story 4 (stop/delete self) in scope? | Yes | user | spec.md Clarifications |
| C2 | clarify | Failed deferred removal reported how? | App notice naming W and reason, plus daemon log | user | spec.md Clarifications |
| M1 | T001–T016 | full | Own-worktree delete after confirmation (incl. wire bump 74, open-handle test) | #1613 | merged |
| M2 | T017–T025 | full | Delete prompt lists uncommitted files and unpushed commits | #1619 | merged |
| M3 | T026–T030 | full | Session stops or deletes only itself | #1621 | merged |

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
| Close review | 1 | a1f9099c:93ce38cf | CHANGES (1 MAJOR fixed: absence waits in decline tests) |
| Close review | 2 (scoped, sonnet) | working tree after fix | CLEAN |
| Plan review | 1 | 0b78d32b342988dd1f35378088cb36a84b3aa567:9dff441a2bbac0be6f7d913c2c0e5dc53484cc7b | CHANGES (3 MAJOR, 3 MINOR, all fixed, prose only: test rows, until_answered plumbing note, R6 wording) |

## Declined review findings

- Review A: dead `loss` protocol surface (declined: M2 populates it, wire bumped once). `Deferred`/tuple plumbing and `ask_user` arg count (declined: M3 adds variants). Duplicate failure-message mapping, leftovers+branch message ordering, second audit `error` line, 2 s peer-close wait, `scheduled` without recheck (MINOR, accepted by design).
- Review B F2 (branch_delete_failed shown as a notice): intended. F1(c) prompt naming other sessions: moved to M2 (T004 text amended).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## M2 review notes

- Review A round 1 (snapshot 6a557439:d7f3f461): fixed the unpushed-commits upstream bug (log now `HEAD --not --remotes`, no upstream branch), and the index lock (`--no-optional-locks`). Declined (MINOR, accepted): commits listed even with `delete_branch=false` (over-warns, safe direction); partial result when `git log` fails (whole read reports unreadable); no cancel/timeout on the git read (as other daemon git calls); `--untracked-files=all` cost on huge trees; `LossSummary` vs `WorktreeLoss` duplication and `ask_user` arg count (M3 adds variants).
- Security finding (coordinator, MEDIUM): fixed. File names and subjects are shown through `worktree::loss::sanitize_entry` (control and bidi/format chars become `?`, reusing `github::is_invisible`; 200-char cap); test `control_and_bidi_characters_in_names_and_subjects_cannot_fake_rows`.

- Review B (sonnet): CLEAN; Verify green. Visual pass (headless render, evidence `visual-pass-m2.md`): PASS. Cosmetic, accepted: "… and 10 more" sits below the fold until the list scrolls; the scrollbar thumb overlaps the last character of the longest rows.

- Security review of PR #1619 (MEDIUM x2). (1) Fixed: `sanitize_entry` also replaces U+2028 and U+2029; both added to `control_and_bidi_characters_in_names_and_subjects_cannot_fake_rows`. (2) Declined, all three: (a) ignored files: listing them puts build output (`target/`, `node_modules/`) in every worktree's prompt and defeats FR-006's "no list when nothing is lost"; a count line needs a `LossSummary` field (wire change) and a spec decision; a follow-up if wanted. (b) a session writing `refs/remotes/*` to hide its commits: the session can already run any git command in its own worktree, and the prompt is advisory for the user's own session; no git read can tell a forged remote ref from a real one. (c) re-reading the loss before the delete: the asking session is blocked on this very confirmation, and a second read would need a re-prompt flow in M1's `decide`; out of M2 scope.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M1 has 16 tasks (over the 10-task guide): it is Foundational (wire bump) plus two P1 stories sharing `decide` and `Deferred`; no acceptance-scenario split gives a observable deliverable. Accepted.
- R6 closed: the Windows job of #1613 was green, so T015 was ticked and `ops.rs` is unchanged.
- #1613 red-CI attempt 1: `a_self_delete_without_stop_sessions_is_the_live_sessions_conflict` started a Claude Code session, which CI lacks. Fixed in 0202ed576: it starts a shell session IDLE instead. Merged at 4edaaa6af.
- Spec FR ids renamed: local FR-014a to FR-017, FR-015a to FR-016 (034's FR-015a keeps its name).

M1 reviews: A (code-review high) round 1: fixed spawn-before-grace and own-session hint. B (conformance, sonnet) round 1: added hangup-while-pending test, shutdown inside the timeout. Visual pass PASS (specs/1602-mcp-self-delete/visual-pass-m1.md). No red-phase evidence (tests written beside code).

M2: T019's daemon tests are in `mcp_lifecycle_tools.rs` (the fixture lives there), not `mcp_confirmations.rs`. "Prompt names other sessions" (moved from M1 review B) is met by the headline's ", stopping its sessions" and a daemon test asserting `stop_sessions: true` in the broadcast. No red-phase evidence (tests written beside code).

M3: Review A round 1 (snapshot 1ebce221:c5f57486): fixed F1 (MAJOR): the stop no-op test counted PTYs and skipped a structured session; now `DaemonState::session_has_process`. No structured-session test (CI has no Claude Code CLI). Audit target of an omitted id is `self`. No red-phase evidence (tests written beside code).
Follow-up (declined in #1619, not done): the delete prompt does not count ignored files; needs a `LossSummary` field and a wire change.

Close: speckit-converge found no unbuilt behaviour. speckit-tdd-verify FAIL (7 findings, `tdd/verification.md`); T031-T037 fixed as tests plus a notice wording change (client notice now "Could not finish the requested action on …"). Windows job of #1613 and #1621 green (T035). Three non-absence sleeps remain in mcp_lifecycle_tools.rs on purpose. Mutation score from 8 hand-made mutants (profile has no tool).
