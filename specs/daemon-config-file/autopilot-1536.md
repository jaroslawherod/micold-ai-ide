# Autopilot ledger — #1536 daemon-config-file (Issue 16)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1536` (SSH key, signing key, CA file and session files for sandboxes; part of #1521); labels: `flow:feature`, `effort:low`. Spec, Clarify, Plan skipped; design on main. Scope: the "Issue 16" task group (T101–T106), FR-036, FR-037, FR-038, contract `#git`, `#session`, `#network`.
- **Kind**: feature
- **Effort**: low
- **Issue**: #1536
- **Worktree branch**: feat/1536_ssh-key-signing-key-ca-file-and-session-files-for
- **Started**: 2026-11-06
- **Phase**: implement M1 (review)
- **Next step**: after design PR merges: milestone M1 (T101–T106)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1748 | design: M1 milestone cut | merged | 3dafe3fdeff78991721162255acdeb460c724f98 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T101–T106 | full | SSH key, signing key, CA file, session files reach sandbox sessions | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | New spec for #1536? | No. FR-036..038 and the Issue 16 tasks already live in specs/daemon-config-file/; spec.md unchanged, so no spec review | agent-resolved | spec.md FR-036..038; tasks.md "Issue 16" |
| D2 | spec | Blockers #1524, #1527, #1528 | All CLOSED | agent-resolved | gh issue view |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| A | 1 | 4d1bf96f235f6da809d05e60b6aa11fd789669c2:5337e2a4cfb10b235ded086dcc01df9bf0c5d775 | CLEAN (3 MINOR) |
| B | 1 | same | CLEAN (2 MINOR; Verify green) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F2 gpg.format not set | T102 keeps gpg.format from the user config by design |
| M1 | A | F1, F3 copy-then-chmod, specific roots | MINOR, not fixed; the source is user-owned, bundle must be complete |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Settings -> Daemons has inputs for the four path extras (ssh key, known hosts, signing key, CA file); `[[session.files]]` entries have no UI row yet (kept from the daemon file on save, render and delivery work). Add a list editor later.
