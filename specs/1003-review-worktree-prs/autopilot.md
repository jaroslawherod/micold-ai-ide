# Autopilot ledger — #1003 review-worktree-prs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Review changes: allow navigating between all PRs of the worktree (only the first PR's diff is reachable) — issue #1003 (body: Problem / Expected / Acceptance criteria as in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1003
- **Worktree branch**: claude/project-thread-c3olht
- **Started**: 2026-10-19
- **Phase**: design-pr
- **Next step**: orchestrator opens the design PR (title/body in the scratchpad); after merge: milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Every PR of the worktree's branch set listed (number, title, state), each diff reachable | — | pending |
| M2 | T015–T021 | full | Check mark on every entry, one mapping shared with the sidebar | — | pending |
| M3 | T022–T031 | full | Header + previous/next; selection and header hold across refreshes (US2 + US3) | — | pending |
| M4 | T032–T036 | full | Loading, empty and unavailable lines | — | pending |

## Decisions

- Clarify round 1: FR-002 branch set = current branch + branches created in the worktree; branches only checked out there do not count (agent-resolved from issue #1003 acceptance criteria and the spec's edge case). Records an existing rule, so no further round.
- Tasks: US1 split (21 tasks) along its scenarios: 1, 2, 4 in M1, scenario 3 (check status) in M2. US3 adds no code (tests pinning behaviour M1 and M3 deliver), so it rides in M3 instead of a milestone with no deliverable. Polish (T037, quickstart results) has no code: close unit.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 42282bddf03c394c74f6c803c06440fd7abe2179:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | CHANGES: 1 MAJOR, 3 MINOR |
| Spec | 2 | fddeb540b934ab3d57ea2fc9d3a6deb18973bed2:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | CLEAN (2 MINOR, fixed prose only) |
| Plan | 1 | c266bc56ee3e8390242f9f9ac5837d2be0fac6a9:074c8404d1ad2213236588c1fda9287d199b7d25 | CHANGES: 1 MAJOR (F1 DWIM remote-branch copy counted as created; fixed: R2 rule 3 drops a same-name remote copy, narrower than suggested so `switch -c x origin/main` still counts), 1 MINOR (F2 source on Err(Unavailable); fixed) |
| Plan | 2 | 10adbf465724a6cc47c51564250b6a409eeadb77:78604c31bcfb0b9e25d7865a5e0193932fee29a6 | CLEAN (1 MINOR wrap, fixed prose only) |
| Tasks | 1 | 2e2d92e5e664b0fd615696e35713ac0ca7c21346:190120bc88951f940720a69dafb552a7213128ef | CLEAN (1 MINOR: T027 visual pass covers §B step 4; fixed prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

