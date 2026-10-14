# Autopilot ledger — #533 jira-issue-worktrees

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Create worktrees from Jira issues (#533); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #533
- **Worktree branch**: feat/533_create-worktrees-from-jira-issues
- **Started**: 2026-10-12
- **Phase**: milestone M1
- **Next step**: M1 PR open; wait for CI and merge, then M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #785 | Design (spec, plan, tasks) | merged | a9ea34a2d7d101cbf3b9ce52a88ad9d6e59aeb61 |
| #819 | M1 | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T021, T034 | full | Jira switch, settings fields, protocol 42 and the opt-in gate | #819 | open |
| M2 | T022–T031 | full | Connect, change and remove a project's connection and token | | pending |
| M3 | T032–T033, T035–T053, T068–T069 | full | Start a worktree from a Jira issue (list, pick, notices); kept whole: core, adapter and dialog are one wired path (no half-wired UI) | | pending |
| M4 | T054–T058, T070 | full | Search Jira issues incl. beyond the loaded list | | pending |
| M5 | T059–T065 | full | GitHub / Jira source choice per project | | pending |

## Decisions

- Plan: switch and connections are service-owned settings (like pr_status_enabled), token a client-side owner-only file; spec Edge "Daemon and sessions" reworded from "not in a session or a daemon" to "no session depends on it".
- Plan: new crate `ureq` 3 (rustls, platform verifier) in the client only; loopback http accepted under cfg(debug_assertions) for the fake site.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | a2227c4095c7db1a63b01b0a70523db4e3314256:13165b3a | CHANGES: 4 MAJOR (all fixed; F5-F7 MINOR fixed) |
| Spec | 2 | 27281b4a37b70e88a0745093f36f29294f5df30f:13165b3a | pending |
| Plan | 1 | b464635f9860e95f65afac4e26f67e12622c7041:0fff8148e25af0f12869d0e5b4ec5db7ca6db297 | CHANGES: 2 MAJOR (F1 multi-daemon/spec wording, F2 loopback fake vs https-only), 4 MINOR; all fixed. F1 amended spec Edge "Daemon and sessions" wording; F2 and F4 add behaviour claims, so re-review (round 2) |
| Plan | 2 | b240cd6856a484a346554254053aaee983f769a8:0fff8148e25af0f12869d0e5b4ec5db7ca6db297 | CHANGES: 1 MAJOR (release test unwired), 2 MINOR; fixed |
| Plan | 3 | adf44a395a8299f73ed08629565c5b6f44af926b:0fff8148e25af0f12869d0e5b4ec5db7ca6db297 | CHANGES: 1 MAJOR (plan self-contradiction on persist_service_settings merge vs copy), 2 MINOR; fixed, prose only (plan/contract wording made to match the merge mechanism round 3 already read; no 4th round, round limit) |
| Plan | confirm (sonnet, tasks unit) | bae8666f588552c1c7a2447bfe65696f43efeeae:c779f5c824f46d2e9d3649c1647476dc206ab227 | CHANGES: 1 MAJOR (plan.md risk row still said persist_service_settings merges); fixed, prose only; rest consistent |
| Code A (M1) | 1 | 1a86b2ebe74b3bdc92bcb8e2f960d87581bde677:5bd6892383a878b47a76d49455fc3e8bf7809d3c | CLEAN (3 MINOR: port range check, empty project key, silent drop on read; declined for M1, see below) |
| Conformance B (M1, sonnet) | 1 | 0382303712ba5169e5a0b928a79cbcec442fa916:5bd6892383a878b47a76d49455fc3e8bf7809d3c | CHANGES: 1 MAJOR (no client test for FR-022 adopt/save), 2 MINOR declined; fixed with 2 tests |
| Conformance B (M1, sonnet) | 2 | e9271b767957cb77284ebf522020edafcff7229e:5bd6892383a878b47a76d49455fc3e8bf7809d3c | CLEAN |
| Code A (M1, sonnet, post-rebase fixes) | 2 | 5b7730bd1c00e525627e260e6bdb497674bc4c91:5f00fbbe33617bf46639612eba895030677f8d3c | CLEAN (3 MINOR: F2 wire-leniency test added, F3 version pin dropped from the 776 test; F1 write-order declined, same order as every other field) |
| Tasks | 1 | bae8666f588552c1c7a2447bfe65696f43efeeae:c779f5c824f46d2e9d3649c1647476dc206ab227 | CLEAN (2 MINOR fixed; speckit-analyze 2 HIGH, 4 MEDIUM fixed before review) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | MINOR F1-F3 (port u16 range; non-absolute project key; silent drop log) | MINOR only; port and project validation are covered by M2's connect form (T022+) which owns site/key input; no new round needed |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Rebased onto main which took PROTOCOL_VERSION 41 (feature 776): M1 is now 41 -> 42; spec artifacts updated.

- M1 had no test-first red-phase evidence (tests written after the code in part 1); no tdd/cycle-log.md; coverage is by the tests T003-T008, T015-T017 passing in the gate, not by red-then-green history.
- GitHub issue picker brought under an integrations switch (pending clarify answer).
