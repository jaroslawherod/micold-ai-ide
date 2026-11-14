# Autopilot ledger — #1536 daemon-config-file (Issue 16)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1536` (SSH key, signing key, CA file and session files for sandboxes; part of #1521); labels: `flow:feature`, `effort:low`. Spec, Clarify, Plan skipped; design on main. Scope: the "Issue 16" task group (T101–T106), FR-036, FR-037, FR-038, contract `#git`, `#session`, `#network`.
- **Kind**: feature
- **Effort**: low
- **Issue**: #1536
- **Worktree branch**: feat/1536_ssh-key-signing-key-ca-file-and-session-files-for
- **Started**: 2026-11-06
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1748 | design: M1 milestone cut | merged | 3dafe3fdeff78991721162255acdeb460c724f98 |
| #1756 | M1 | merged | f6c8a1c6604e7ed460df121db260504a7c161a7c |
| #1844 | M2 | merged | 8d86299527ffd3011afd845c30a8f28b76029cee |
| (close) | close: tdd-verify remediation T164-T167, spec Status | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T101–T106 | full | SSH key, signing key, CA file, session files reach sandbox sessions | see PR table | merged |
| M2 | T148–T156 | full | The extras are delivered to container, pod and SSH daemons (M1 rendered client-local paths; `deliver_container`, pod and `ssh_delivery` did not deliver them); tdd-verify I16-1..I16-8 tests | #1844 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | New spec for #1536? | No. FR-036..038 and the Issue 16 tasks already live in specs/daemon-config-file/; spec.md unchanged, so no spec review | agent-resolved | spec.md FR-036..038; tasks.md "Issue 16" |
| D2 | spec | Blockers #1524, #1527, #1528 | All CLOSED | agent-resolved | gh issue view |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| A | 1 |  |  | 4d1bf96f235f6da809d05e60b6aa11fd789669c2:5337e2a4cfb10b235ded086dcc01df9bf0c5d775 | CLEAN (3 MINOR) |
| B | 1 |  |  | same | CLEAN (2 MINOR; Verify green) |
| A (M2) | 1 |  |  | 8ef42345b2ed20d23f979180bd088090087a30c4:60bb9f0a1a831f79d23699d806880f89a2de258c | CHANGES: F1 a leftover mutant in `deliver_file` (fixed, restored); F2 pod key mode (follow-up) |
| B (M2) | 1 |  |  | same | CHANGES: F1/F2 cycle-log entries missing (fixed in the log; mutation run left to close unit's tdd-verify); F3/F4 MINOR to follow-ups; scoped gate green stood in for Verify |
| A | 2 | 1844 | e76339bd8894d8f87c9dcfe227589e69ffe70ef4 | 109d5824af9fb44fdc5cc730b1bfd7fdd39de921:e76339bd8894d8f87c9dcfe227589e69ffe70ef4 | CLEAN (2 MINOR: stale remote key copies, delivery write comment says rename; follow-ups) |
| B | 2 | 1844 | e76339bd8894d8f87c9dcfe227589e69ffe70ef4 | 109d5824af9fb44fdc5cc730b1bfd7fdd39de921:e76339bd8894d8f87c9dcfe227589e69ffe70ef4 | CLEAN (3 MINOR; Verify by CI green on head, targeted run not started) |
| A | 3 | 1844 | 99825b74517607de40b166cbf02135ca66fc131a | 2bcfdff2dd60d54a3ffb7231b0b3cb831a170f2d:99825b74517607de40b166cbf02135ca66fc131a | CLEAN (rebase onto main: code patch-ids identical, docs conflicts kept both sides; gate green 12838 tests) |
| B | 3 | 1844 | 99825b74517607de40b166cbf02135ca66fc131a | 2bcfdff2dd60d54a3ffb7231b0b3cb831a170f2d:99825b74517607de40b166cbf02135ca66fc131a | CLEAN (rebase onto main: code patch-ids identical, docs conflicts kept both sides; gate green 12838 tests) |
| A | 4 | 1844 | 6017c87a9389b5dd17a4434143f8d4c595ef6a0f | 5fe37646c018536c95ec1295237631b11b0cee2d:6017c87a9389b5dd17a4434143f8d4c595ef6a0f | CLEAN (rebase onto moved main plus call-site fix 6017c87a9; patch lines unchanged; gate green 12888 tests) |
| B | 4 | 1844 | 6017c87a9389b5dd17a4434143f8d4c595ef6a0f | 5fe37646c018536c95ec1295237631b11b0cee2d:6017c87a9389b5dd17a4434143f8d4c595ef6a0f | CLEAN (rebase onto moved main plus call-site fix 6017c87a9; patch lines unchanged; gate green 12888 tests) |
| A | 5 | 1854 | ecfbe32df4107e9062dd10ff06c7a0fae4b2140e | b516856df9ae864e99ceea047c63c84187522f7d:ecfbe32df4107e9062dd10ff06c7a0fae4b2140e | CLEAN (close review of 4884ffcbc; 4884ffcbc..ecfbe32df is ledger/format-only; 3 MINOR in follow-ups; full gate green) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F2 gpg.format not set | T102 keeps gpg.format from the user config by design |
| M1 | A | F1, F3 copy-then-chmod, specific roots | MINOR, not fixed; the source is user-owned, bundle must be complete |

## Handover

None. Close unit done: tdd-verify re-run with mutation pass (53 mutants), I16-9..I16-16 closed by tests (T164-T167), spec Status set, review CLEAN, full gate green; close PR opened.


## Open escalation

None.

## Follow-ups not done

- Settings -> Daemons has inputs for the four path extras (ssh key, known hosts, signing key, CA file); `[[session.files]]` entries have no UI row yet (kept from the daemon file on save, render and delivery work). Add a list editor later.
- Pod: the startup secret mounts at `defaultMode` 0440, so OpenSSH may refuse the delivered SSH key (group-readable); check in the real-cluster test or give the key items mode 0400. The pod size limit is per file, not a total of the Secret's 1 MiB (review A F2, B F3/F4, M2).
- Delivery: `write_secret` doc comment claims temp file and rename but writes in place (daemon_config_delivery.rs ~390); an SSH host keeps `ssh-key`/`signing-key` copies sent by an earlier version (review A M2 round 2 MINOR).
- Close review MINORs: the `HOME` fallback test (`supervisor.rs` `session_files_fall_back_to_the_daemons_home_variable`) sets HOME under a mutex only it takes and restores it off a drop guard; `supervisor.rs:1562` still iterates `CA_VARIABLES`; no real TLS handshake test of the daemon's CA trust (needs a certificate generator dev-dependency).
