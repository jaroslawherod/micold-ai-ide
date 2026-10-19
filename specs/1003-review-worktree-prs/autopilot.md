# Autopilot ledger — #1003 review-worktree-prs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Review changes: allow navigating between all PRs of the worktree (only the first PR's diff is reachable) — issue #1003 (body: Problem / Expected / Acceptance criteria as in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1003
- **Worktree branch**: claude/project-thread-c3olht
- **Started**: 2026-10-19
- **Phase**: plan
- **Next step**: plan unit: plan review round 2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Clarify round 1: FR-002 branch set = current branch + branches created in the worktree; branches only checked out there do not count (agent-resolved from issue #1003 acceptance criteria and the spec's edge case). Records an existing rule, so no further round.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 42282bddf03c394c74f6c803c06440fd7abe2179:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | CHANGES: 1 MAJOR, 3 MINOR |
| Spec | 2 | fddeb540b934ab3d57ea2fc9d3a6deb18973bed2:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | CLEAN (2 MINOR, fixed prose only) |
| Plan | 1 | c266bc56ee3e8390242f9f9ac5837d2be0fac6a9:074c8404d1ad2213236588c1fda9287d199b7d25 | CHANGES: 1 MAJOR (F1 DWIM remote-branch copy counted as created; fixed: R2 rule 3 drops a same-name remote copy, narrower than suggested so `switch -c x origin/main` still counts), 1 MINOR (F2 source on Err(Unavailable); fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

