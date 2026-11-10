# Implementation Plan: Staged review for speckit-autopilot

**Branch**: `feat/1804_autopilot-staged-review-facts-by-code-plan-by` | **Date**: 2026-11-10 | **Spec**: [spec.md](spec.md)

## Summary

Add `scripts/autopilot/review-facts.py` (no model) that writes a per-snapshot facts file; describe a
four-stage review (facts, plan, probes, verdict) in `autopilot-reviewer.md`, `tasks/review.md` and
`tasks/review-rounds.md`, dispatched stage by stage by the unit (reviewer subagents cannot nest);
run it as a shadow beside the single-agent review for three code-diff reviews, recording the
comparison in `shadow.md`. Verdict format, rubrics and ledger stay unchanged.

## Technical Context

**Language/Version**: Python 3 (as `scripts/autopilot/brief.py`) plus bash test harness; no Rust change.
**Primary Dependencies**: `git` only (`git diff -U0`, `git grep -nw`); optional lcov file for coverage. No `rg`, `ctags` or `cargo-llvm-cov` required.
**Storage**: files in the session scratchpad, `<out-dir>/review-facts-<tree>-<head>.md`; shadow record committed in the feature dir.
**Testing**: `scripts/tests/autopilot.test.sh` (throwaway repos), run by `mise run gate`.
**Target Platform**: Linux and macOS (BSD/GNU safe: no GNU-only flags, python stdlib).
**Project Type**: repository tooling (skill scripts and markdown).
**Performance Goals**: facts for <=50 files under 60 s (SC-003).
**Constraints**: offline; the script never calls a model; coverage absence never fails.
**Scale/Scope**: one script, three markdown files, one test block, one shadow record.

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | PASS: script tests in `autopilot.test.sh` are written before the script (tasks order them). Markdown flow is checked by the shadow run and the SC-004 check. |
| II. Multi-Session | PASS: facts file keyed by `<tree>:<head>` and written to the per-session scratchpad; no shared state. |
| III. Worktree | PASS: script runs in the current worktree against `origin/main`; creates no worktree. |
| IV. Local-First | PASS: no network; no remote service. Model calls are the existing subagents. |
| V. Rust + iced | N/A: tooling only, no app code; no GUI. Justified, no violation. |
| VI. Cross-Platform | PASS: python stdlib and git only; BSD/GNU tolerant; coverage missing = unknown on every OS. |
| VII. Documentation | PASS: no user-facing behaviour; contributor docs are the three skill files, updated in the same change. |
| VIII. UI components | N/A: no UI. |

Re-check after design: unchanged. Complexity Tracking: none.

## Project Structure

```text
specs/1804-staged-review/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/facts-file.md        # script CLI and facts file format
├── contracts/staged-review.md     # stage prompts, limits, fallback, shadow record
└── shadow.md                      # created in the shadow phase (US3)
scripts/autopilot/review-facts.py  # new
scripts/tests/autopilot.test.sh    # extended
.claude/agents/autopilot-reviewer.md
.claude/skills/speckit-autopilot/tasks/review.md
.claude/skills/speckit-autopilot/tasks/review-rounds.md
```

## Design by requirement

| FR | Where |
|---|---|
| FR-001, FR-003 | `review-facts.py`: files and ranges from `git diff -U0 origin/main...HEAD` plus working tree (same tree as `review-snapshot.sh`); symbols, callers, coverage per contracts/facts-file.md |
| FR-002 | `--out-dir` (scratchpad); file name carries the snapshot |
| FR-004 | Stage 2 prompt in contracts/staged-review.md; <=6 probes |
| FR-005 | Stage 3: unit launches one `autopilot-worker` per probe, `model: "haiku"` or `"sonnet"`, one message, <=40 lines each |
| FR-006, FR-010 | Stage 4 prompt reuses the output contract verbatim; `review-rounds.md` rules apply to the staged result unchanged |
| FR-007 | The three markdown files; `autopilot-reviewer.md` gets a "Stage roles" paragraph |
| FR-008, FR-009, FR-012, FR-013 | Shadow section in `review.md`; `shadow.md` table; decision section |
| FR-011 | Fallback rules in `review.md`: script non-zero, missing/failed probe, stale snapshot -> single-agent review, never `CLEAN` |

Rebase constraint: the head-bound verdict work edits the same three files; edits here touch only the staged-flow and shadow paragraphs and leave the verdict format text alone.

Shadow runs on the next three code-diff PRs after merge, so US3 and SC-005 close after this feature merges: the tasks unit adds a final shadow-phase task and a follow-up issue that fills `shadow.md` and records the decision.

Round 2+ staged reviews use `review-snapshot.sh diff <snap>` as the diff the facts script reads
(`--base-snapshot <snap>` option), per the spec edge case. Artifact reviews stay single-agent.

## Test strategy

| Layer | Covers |
|---|---|
| `autopilot.test.sh` throwaway repo | FR-001..003, FR-011 script side (exit codes), US1 scenarios 1-4, empty and docs-only diff, no-origin/main failure, SC-003 timing on 50 files, concurrent keys |
| SC-004 check in the same file | feed a staged-format verdict to the existing `VERDICT:`/`F<n>` and ledger readers (`brief.py`, `checkpoint.sh`) |
| `autopilot.test.sh` grep assertions | the three markdown files contain the 6-probe and 40-line limits, the never-`CLEAN`-on-unanswered rule, the fallback and shadow text (FR-004, FR-005, FR-007, FR-010, FR-011, FR-013 wording); quickstart step 4 fixture run exercises FR-004 and FR-011 behaviour |
| Shadow run (US3) | FR-005..009, FR-012: three real reviews, recorded in `shadow.md` |
| Core unit / client / geometry / quickstart §B | not applicable: no Rust or UI |
