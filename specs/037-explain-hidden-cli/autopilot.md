# Autopilot ledger — 037-explain-hidden-cli

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: continue with next issue use autopilot
- **Kind**: feature
- **Issue**: #434
- **Worktree branch**: fix/github-issues
- **Started**: 2026-10-01
- **Phase**: 2-clarify
- **Next step**: Clarify round 1 is waiting on the user's answer to the open escalation (form of FR-010). When answered: record it under `## Clarifications` in spec.md as `_(decided by user)_`, apply it to User Story 3 scenario 1, FR-010, SC-007 and the *availability rule* assumption, tick the checklist, commit (no push; ships in PR 2).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #515 | Spec | merged | 063fa77affbcbcc20a842bffb71dce22212582d6 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Is #434 a bug against a Closed spec, or new behaviour? | New behaviour, own spec. 027 FR-023b already names a missing CLI under Default AI CLI, but no closed spec requires a reason or an action. | agent-resolved | specs/027-sandboxed-daemon-runtime/spec.md#FR-023b; specs/029-pi-cli-provider/bugs/BUG-001.md#spec.md ("no requirement that a hidden CLI be explained") |
| D2 | spec | Feature number | 037. 036 is taken by `036-tooltip-follow-cursor-delay` in the `fix-issue-430` worktree. | agent-resolved | `ls <worktree>/specs/03[6-9]*` on 2026-10-01 |
| D3 | spec | Do the environment reasons apply under container placement? | Yes. Environment-include applies where sessions run under both placements, so "the image lacks it" is true only when the environment resolved in full. Refines 027 FR-023b. | agent-resolved | specs/029-pi-cli-provider/spec.md#FR-003b; Spec review round 1, F5 |
| D4 | spec | Is the reply an AI session gets for a missing CLI in scope? | Yes (FR-009a). It says "is not installed", the claim FR-002 forbids. | agent-resolved | crates/micold-daemon/src/mcp/tools.rs:470; Spec review round 1, F4 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 39923bcd771235ee0060a1f1f6330830a153df7d:65fbe60069a3f9b434d1550233c450000099604e | CHANGES: 5 MAJOR, 3 MINOR — all fixed |
| Spec | 2 | b460803fa1576b80357bd110538b491691e8f6c3:65fbe60069a3f9b434d1550233c450000099604e | CLEAN: 1 MINOR, fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, 2026-10-01. Category 1 (product decision the repo does not settle). One question;
the scan found no other ambiguity worth a question.

**Q1. On a sidebar row, where is a CLI that is not offered there explained when fewer than two CLIs
are available (so the row has no chevron today)?**

- **B (Recommended)** — Keep the chevron rule. The reason appears only in a row list that already
  opens (two or more CLIs available, another one missing). A row with one CLI stays exactly as it
  is; that user gets the reason from the Settings note (Story 1) and the start messages (Story 2).
  No closed spec is amended.
- **A** — Show the chevron whenever a supported CLI is not offered on that row, listing it as a
  non-selectable entry with the reason. Amends 026 FR-006.
- **D** — As A, but only when the environment was not applied in full (the first five states of
  FR-001: off, blank path, unreadable, failed, timed out). A CLI that is simply not installed adds
  no chevron. Amends 026 FR-006 for those states.
- **C** — No change at the row. Drops User Story 3, FR-010 and SC-007.

What was checked:

- `specs/026-multi-provider-sessions/spec.md` Clarifications 2026-08-16 and FR-006: the chevron is
  absent with one CLI "so a single-CLI user sees exactly today's interface". Its Story 3 test still
  allows a missing CLI to be "absent or clearly marked unavailable", so 026 does not rule A out.
- `specs/033-directory-aware-start-affordance/spec.md` Clarifications: the user chose a
  directory-aware chevron over "always draw the chevron"; the visual form was left unchanged.
- `specs/011-*/spec.md` FR-004: environment-include defaults to on. So under A nearly every user
  with one CLI (script succeeded, Copilot and Pi not installed) gets a chevron on every row. Under D
  only users whose script was not applied do.
- Issue #434 names both the Settings selector and the per-session list as "candidates". It does not
  choose.
- The BUG-001 reporter had one CLI available. B does not reach them at the row; A and D do.

Why B is recommended: it keeps the interface 026 and 033 decided for the majority, and Stories 1
and 2 already tell the BUG-001 reporter the reason and the action.

Paused: the marker in User Story 3 scenario 1 and FR-010 stays until the answer. Nothing was
changed in spec.md this round.

## Token usage

## Follow-ups not done

- The issue text says nothing names a hidden CLI. Settings already does (027 FR-023b): "*<name>*
  isn't installed on this computer, which is where sessions run." The spec treats that sentence as
  the thing to correct.
