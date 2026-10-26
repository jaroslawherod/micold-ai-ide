# Autopilot ledger — #1422 container-name-min-length

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #1422 "Container name: enforce a minimum length (Docker needs 2+ characters)"; labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1422
- **Worktree branch**: feat/1422_container-name-enforce-a-minimum-length-docker
- **Started**: 2026-10-25
- **Phase**: milestone
- **Next step**: M1 PR #1489 CI and merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1433 | design | merged | 48cda33fe6d376055f88aa7ad672842fa3aa8c6a |
| #1489 | M1 | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Container names under 2 characters refused at Save; placeholder hidden; legacy loads | #1489 | PR open |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | milestone M1 gate | `a_24_character_name_keeps_its_room_at_every_depth` fails on plain origin/main here: run the gate elsewhere or fix the test? | Fix the test first (merged as PR #1482, #1462); rebased onto origin/main | decided by user | gate red on unmodified origin/main |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks review | 1 | c100ddba0515c7c28900f913580f41407520fd2d:ab6e6b6e7fe761534a06c67a4cf8183dfd268da6 | CLEAN (3 MINOR, fixed, prose only) |
| M1 review A (code-review high) | 1 | feefce5d3f513eb09b1aef3350ad78fc9e14d616:dc42fcc829c22d8bf3748cb07201d402ffc182d7 | no BLOCKER/MAJOR; MINORs declined or deferred |
| M1 review B (conformance) | 1 | same | CLEAN |
| Spec review | 1 | a56d0f73c3192bb09fcf2e27967a43a6c3e7cdeb:357d48d5be4eedc191d9a5b91cc25f19270f23ef | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

- M1 A: registry-level enforcement, ASCII/leading-char validity, edit of legacy `a` blocked, and stored-name check: out of spec (FR-001 Save form only; FR-006 states legacy `a` is refused on save). Declined.
- M1 A: `required_fields` marker for the container name when the default is unusable: not in the spec; see Follow-ups.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

- plan review round 1 (snapshot 01753217e849eb44dd1aaa33bae1e7f99bf2ef49:f853de0a598b491abae7626493d6ee444c72c3ec): CLEAN, 3 MINORs fixed, prose only

- `required_fields` could mark the container name required when the daemon name gives no usable default (M1 review A, MINOR).
