# Autopilot ledger — #1536 daemon-config-file (Issue 16)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1536` (SSH key, signing key, CA file and session files for sandboxes; part of #1521); labels: `flow:feature`, `effort:low`. Spec, Clarify, Plan skipped; design on main. Scope: the "Issue 16" task group (T101–T106), FR-036, FR-037, FR-038, contract `#git`, `#session`, `#network`.
- **Kind**: feature
- **Effort**: low
- **Issue**: #1536
- **Worktree branch**: feat/1536_ssh-key-signing-key-ca-file-and-session-files-for
- **Started**: 2026-11-06
- **Phase**: milestone (M2)
- **Next step**: milestone unit M2, then a new close unit (re-run tdd-verify, spec Status, follow-ups, close PR)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1748 | design: M1 milestone cut | merged | 3dafe3fdeff78991721162255acdeb460c724f98 |
| #1756 | M1 | merged | f6c8a1c6604e7ed460df121db260504a7c161a7c |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T101–T106 | full | SSH key, signing key, CA file, session files reach sandbox sessions | see PR table | merged |
| M2 | T148–T156 | full | The extras are delivered to container, pod and SSH daemons (M1 rendered client-local paths; `deliver_container`, pod and `ssh_delivery` did not deliver them); tdd-verify I16-1..I16-8 tests | — | pending |

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

Close unit 2 found unbuilt behaviour (claim verified): `deliver_container`, the pod delivery and `ssh_delivery` do not deliver `ssh_key_file`, `known_hosts_file`, `signing_key_file`, `ca_file` or `[[session.files]]` sources, so only a host daemon gets them; on SSH the rendered key path is client-local. #1536 acceptance criteria 1 to 4 do not hold for C, K and S. Cut M2 (T148-T156, in tasks.md) including tdd-verify findings I16-1..I16-8 (verification.md section Issue 16 is committed). Run M2, then a new close unit: re-run tdd-verify, add #1536 rows to test-list (T156 does), record follow-up (no Settings row for `[[session.files]]`), spec Status for the Issue 16 group, close PR `docs(daemon-config-file): close the spec (#1536)`. Review B for M1 was CLEAN, so skip speckit-converge.

## Open escalation

None.

## Follow-ups not done

- Settings -> Daemons has inputs for the four path extras (ssh key, known hosts, signing key, CA file); `[[session.files]]` entries have no UI row yet (kept from the daemon file on save, render and delivery work). Add a list editor later.
