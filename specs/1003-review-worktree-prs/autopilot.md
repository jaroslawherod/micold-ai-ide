# Autopilot ledger — #1003 review-worktree-prs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Review changes: allow navigating between all PRs of the worktree (only the first PR's diff is reachable) — issue #1003 (body: Problem / Expected / Acceptance criteria as in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1003
- **Worktree branch**: claude/project-thread-c3olht
- **Started**: 2026-10-19
- **Phase**: implement
- **Next step**: M2 gate and reviews, then PR (orchestrator opens it)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1089 | design | merged | 6fbd754f11c2d0ed387dfb0ae886c0070ac2d573 |
| #1094 | M1 | merged | 577dc7141bc72365d0084961a184d55fb394d46a |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Every PR of the worktree's branch set listed (number, title, state), each diff reachable | #1094 | merged |
| M2 | T015–T020 | full | Check mark on every entry, one mapping shared with the sidebar | — | in progress |
| M3 | T022–T031 | full | Header + previous/next; selection and header hold across refreshes (US2 + US3) | — | pending |
| M4 | T032–T036 | full | Loading, empty and unavailable lines | — | pending |

## Decisions

- Clarify round 1: FR-002 branch set = current branch + branches created in the worktree; branches only checked out there do not count (agent-resolved from issue #1003 acceptance criteria and the spec's edge case). Records an existing rule, so no further round.
- M1: T014 (visual pass, needs `gh` signed in) moved to the close unit beside T037, after review B F1: M1's container has no `gh` auth and no `xdotool`. M1 rendering is unchanged since 775 (more rows only); the shell tests cover the rows.
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
| M1 A (code-review high) | 1 | bef59e95fbba721e36f6f54cbecf8cb2d371c0d4:89a7b0abec0a58022fce19560d375967f4976cc4 | CLEAN |
| M1 B (conformance) | 1 | bef59e95fbba721e36f6f54cbecf8cb2d371c0d4:89a7b0abec0a58022fce19560d375967f4976cc4 | CHANGES: F1 BLOCKER T014 ticked but not run (fixed: unticked, moved to the close unit beside T037); F2 MINOR T004 has no separate red run (not fixed: historical, cycle log says so); F3 MINOR reviewer could not run `pr_list_branch_set` (ENOSPC; unit's run: 16 + full client suite passed) |
| M1 B (conformance) | 2 | 74880d28be801857f9b50418d8c67b3ff0b819d7:ca62c70abdc3140bafc276b1afad54b7a30d433f | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- T021 (M2 visual pass, quickstart §B step 2 check marks) moved to the close unit beside T014 and T037: same reason.
- T014 (M1 visual pass, quickstart §B steps 1, 2, 6) moved to the close unit beside T037: no `gh` auth and no `xdotool` in M1's container (`visual-pass.md`).

