# Autopilot ledger — #1150 diff-full-file-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1150 "Show the full file and browse the repo's code while reviewing a diff in the Changes view" (full text via gh issue view 1150); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1150
- **Worktree branch**: feat/1150_show-the-full-file-while-reviewing-a-diff-in-the
- **Started**: 2026-10-20
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1151 | Design PR (spec, plan, tasks) | merged | df019910c4b525b25df8b08f3fd1073a11b28f72 |
| #1156 | M1 expand hidden lines | merged | 25e95c33ff20a6a07f72abaa4d0aad4aee683cd2 |
| #1162 | M2 whole-file toggle | merged | 8109df50677a9a0635067277aa0df3137e3c3c48 |
| #1215 | M3 pull request head version | merged | a608e0fcb7713377ed9abe3426b958462b91a9db |
| #1221 | M4 pick revealed lines | merged | df514cb373e4e93a20c4294b2518f2c689f03c89 |
| #1253 | M5 browse repository files | merged | 2dddbef617146c535dadbc0dd5687169836fa50a |
| #1282 | M6 browse at the PR head | merged | 47cfcc3c58d5f371ad5dfb0afa73e3a2ee83d5d6 |
| #1291 | M7 follow references from the diff | merged | 93a720fbdd2f462287624d7bf4405eb19b810f85 |
| (close) | Close the spec | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Expand hidden lines in a diff (MVP) | #1156 | merged |
| M2 | T018–T022 | light | Whole-file toggle | #1162 | merged |
| M3 | T023–T031 | full | Pull request head version | #1215 | merged |
| M4 | T032–T035 | light | Pick revealed lines | #1221 | merged |
| M5 | T036–T046 | full | Browse repository files | #1253 | merged |
| M6 | T047–T053 | full | Browse at the PR head | #1282 | merged |
| M7 | T054–T062 | full | Follow references from the diff | #1291 | merged |

## Decisions

- M1 has 17 tasks (Setup + Foundational + US1): Foundational alone is no milestone, and no acceptance-scenario split exists below the gap marker. M5 has 11 tasks, kept whole: its tree, picker and back share one reducer state and no acceptance scenario splits them. Polish (T063-T064) changes no code: left to the close unit.
- analyze C1/C2: browsed-file pick pin moved to M5 (T038); M4 mirrors the existing line-pick handling.

- Close waiver (decided by the autopilot close unit, no human decision; recorded here and in tdd/verification.md): TDD verify verdict FAIL on red-phase ordering only (no tdd/test-list.md or cycle-log.md for any milestone; milestones used conformance reviews A and B instead). Waived by the autopilot owner: history cannot be rewritten, and 4 of 4 hand-made mutants were caught, `test-core` green. Open: client visual pass (T066). A shallow speckit-converge pass at close found T001-T062 ticked and the main symbols present, no unbuilt behaviour found.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| M5 A (code-review high via autopilot-reviewer) | 1 | 5596f87c9a5a5da5818fb79f0e7a3e146b161b5f:f1581e5836e962c58e60a243a8729545282f5cc8 | MAJOR browsed file/tree not re-read on change, fixed (Effect::Batch); 2 MINOR fixed |
| M5 A | 2 | af74dbaa6bd7b15e5706473aaaad3328e4401f48:f1581e5836e962c58e60a243a8729545282f5cc8 | CLEAN |
| M5 B (sonnet) | 1 | af74dbaa6bd7b15e5706473aaaad3328e4401f48:f1581e5836e962c58e60a243a8729545282f5cc8 | 2 MAJOR declined (no tdd dir by design; Verify queued on lock), 2 MINOR |
| Spec | 1 | 61e32c1ce6afda347a1774dc7b2c2ebd349b7abd:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | round 1 CHANGES: 2 MAJOR (F1 reference undefined, F2 picking in untouched files); all fixed, 3 MINOR fixed |
| Spec | 2 | ebdf38fc38d5021afe13cc798bd796d24dabf88b:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CLEAN (1 MINOR fixed, prose only) |
| Plan | 1 | a062beaa985a9a6340ed2e978a69d8ea0fe6978c:6179267cd162fbbfa13ebaa171f85c55810ee0be | CHANGES: 3 MAJOR (PR trailing gap count, hunk-header conventions, test-layer map), 4 MINOR; all fixed |
| Tasks | 1 | 5266c05cc605449114f9bfbfc5ad2ee6617854f2:111e9d2846815d3132eb6a1358ea0f22e03e285a | CHANGES: 2 MAJOR (M5 deps, read_file tests placement), 3 MINOR; all fixed |
| Tasks | 2 | b4c0e78be90cc023ce70435de12ba29d12acbdfd:5c79030bf66c3416cdc958865266352c156ac6c0 | CLEAN (1 MINOR fixed) |
| M1 A (code-review high) | 1 | 14c45c7f7a26f35387a4d0b109d4b47ec909feaa:035ae6f1a54d7e33ee5b7c8c839cc1486b737d9c | CHANGES: 1 MAJOR (pick kept after run hidden) fixed + test; 1 MINOR (whole new side highlighted for non-revealable files) declined |
| M1 B (conformance) | 1 | same | CHANGES: 1 MAJOR (reshape before jump_offset on DiffRead) fixed; Verify green |
| M2 A+B (autopilot-reviewer, combined) | 1 | d46fca691da0d60a56a4503dcb86b060016d02d9:cd601b580040cabf5a3eb1eb5f57baac4bb3fe57 | CLEAN; Verify green; 1 MINOR fixed (toggle hidden for PR source until M3), 2 MINOR no action |
| Plan | 2 | 3213adb2c03edaa4431e840ab07b5a36c88de031:6179267cd162fbbfa13ebaa171f85c55810ee0be | CLEAN (3 MINOR fixed, prose only) |
| M3 A (code-review high) + B (autopilot-reviewer sonnet) | 1 | ba4b1ee6541e38c40fea2c613f9b159e0ef8dadb:6a15094430c1a28268aa59eb561f688cb798d3f8 | A: fixed size gate before local blob read, force_large on ShowLarge, stale head_text reset; B: MAJOR fixed (local head read no longer needs gh), MINOR no shell test for address parsing left |
| M4 A+B (autopilot-reviewer sonnet, combined) | 1 | 63ac6f9f17622829cba16564ff524073c9b47aae:a608e0fcb7713377ed9abe3426b958462b91a9db | MAJOR fmt fixed (cargo fmt); MINOR duplicate selected check fixed via pick_allowed; own gate run caught PR-source selection regression, fixed via selected_key |
| M6 A (code-review high) + B (autopilot-reviewer sonnet) | 1 | 964f9451e7ccc2c3b4b0b0c906a8e52392086d17:2dddbef617146c535dadbc0dd5687169836fa50a | A: fixed submodule entries in local head tree, Back frame uses selected_file for PRs, tree reset on source change, scroll messages skip snapshot, commit_is_local helper, PR tree on remote entry answers HeadUnavailable; B: CLEAN, MINOR fixed (test names contain `browse`). Visual pass not run in client (visual-pass/m6.md) |
| M7 A (code-review high) + B (autopilot-reviewer sonnet) | 1 | a8fdac7f:7a4ef5e7 | A: CLEAN, MINOR fixed (line kept while Reading), probe allocations declined; B: CLEAN, 3 MINOR (no cycle-log: declined as for M5; visual pass not run in client: visual-pass/m7.md absent; uncommitted edits: committed) |

## Declined review findings

- M5 review B F1 (no tdd/cycle-log.md): no milestone of this feature keeps one (M1-M4 merged without); tests were written with the code. F2 (Verify unobserved): the scoped/full gate runs the same tests. F3, F4 MINOR: not fixed.

- M1 A F2 (MINOR): the whole new side is highlighted for added/deleted/PR files too; a per-kind bound needs the change kind threaded into the highlight path, and the highlighter already has a time budget.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A | remote-daemon entries get no GitHub fallback | head_file needs a local checkout dir for the local-first read and the confinement; remote entries are out of scope (FR-014 local-first) |
| M3 | A | submodule/symlink JSON from contents API; backslash in Confined; head_note string compare; off_thread duplication; worktree symlink TOCTOU | MINOR or unused path (Worktree source not used yet); Confined backslash rule is spec'd (test a_relative_path_is_confined) |
| M6 | A | re-read of the browsed path after a source switch leaves no Back target | data-model §4 and the spec edge case say: stack cleared and the browsed path re-read (or shown missing) |
| M6 | A | tree_at timeout/rate limit have no distinct message; fake script queue duplicated | MINOR; spec maps every head failure to HeadUnavailable; duplication below budget |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

Client visual pass of Part B (quickstart, T066): needs a display, daemon and GitHub; M5-M7 client flows never seen on screen (only the showcase BrowseBar in M5). Mutation testing was by 4 hand-made mutants only.
