## Close the spec — daemons dial the orchestrator (specs/1725-daemons-dial-orchestrator)

All milestones merged: #1971 (M1), #1975 (M2), #1976 (M3), #1979 (M4), #1989 (M5), design #1969. This PR closes the spec.

- Plan drift: none for M1-M5. Converge skipped: every milestone has a CLEAN review B.
- `speckit-tdd-verify`: verdict FAIL, only because 6 of 15 behaviors are test-after and squash merges hide the order; no HIGH smell, 22 of 23 criteria covered, 3 of 3 deliberate mutants caught. `tdd/verification.md` and tasks T041-T048.
- Done here: T042 (`ssh_real_link`: 6 passed against a real sshd), T047 (`status_is` shared in `orchestrator_dial/main.rs`; 25 passed).
- Deferred to follow-ups (ledger): T041, T043-T046, T048.
- Behaviour decision for jaro: SSH hosts with their own forwards, or where `ssh -G` fails, now fail to connect (T028); revert is one match in `provision::admit_link`.
- Spec Status set to Closed.

## Agent review
- Close review (sonnet, headless): 1 BLOCKER (unused imports after the helper move), 2 MINOR; all fixed.

## Local gate
Local gate: fmt and clippy (core, workspace, -D warnings) green via scripts/gate.sh; `cargo nextest` is missing in the cloud sandbox, so the workspace test run is left to CI. Ran `orchestrator_dial` (25 passed) and `ssh_real_link` against a real sshd (6 passed).

Refs #1725
