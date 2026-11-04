# Autopilot ledger — #1529 daemon-config-file (Issue 8)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1529` (One GitHub token for many repositories); labels: `flow:feature`, `effort:high`. Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526 and #1534: design on main (PR #1509, `specs/daemon-config-file/`: FR-007, FR-009, SC-002, `contracts/daemon-config.md#githubtokens`); the tasks unit only cuts milestones for the "Issue 8" group (T040–T045).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1529
- **Worktree branch**: feat/1529_one-github-token-for-many-repositories
- **Started**: 2026-11-04
- **Phase**: milestone M1 (implemented, in review)
- **Next step**: review A, gate, PR for M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1681 | Design (milestone for Issue 8) | MERGED | e42ca1e77c49dc2cf7273a45f610601b3fc261bc |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T040–T045 | full | tokens stored by name, one-repository refusal gone, Settings lists tokens in file order with reach, used and shadowed token per repository; existing per-repository tokens migrate unchanged | — | in progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1529? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-007/FR-009/SC-002. Skipped as for #1524/#1525/#1526/#1534. | agent-resolved | specs/daemon-config-file/autopilot-1525.md |
| D2 | tasks | Split Issue 8? | No: 6 tasks, one story; store, refusal and Settings view serve one scenario (milestones.md rule 3). Tier full: persistence format and security check. T045 travels in M1 (rule 4). | agent-resolved | milestones.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks | 1 | 07cb9a5b6f2547974183bbc6c1458e3919838de0:70539f472a111826ae2b4480391002acaf5b7df2 | CLEAN; 2 MINOR fixed (T042 path, render assertion), prose only |
| M1 A (code-review high) | 1 | 59d20c7229e75aba83fb136c566b6384dfab399e:4fe0bd618e8f6c0760e91e865be39f44d3a8d564 | 10 findings: 5 fixed (migrate loop on long names, once-per-run migration, dropped tests restored, set_order failure reloads, dedup), 5 declined |

## Declined review findings

- M1 A: `GithubTokenSetting::from_store` is not wired into `DaemonSettings::from_entry`. Delivering a token file path to a daemon needs the mounts the delivery issues (5-7) create; wiring host paths now would write unreachable paths into container configs. Recorded as a follow-up.
- M1 A: an unfiltered token always wins over a later unfiltered one. The contract (`[[github.tokens]]`, D6) says the first in file order wins among equally specific tokens; the reach cache (`github-reach.json`) is not part of M1.
- M1 A: an expired specific token blocks a valid less specific one. The contract chooses by specificity and file order only; Settings warns on the expiring row.
- M1 A: push permission is probed on the first exact repo only. `pat::check` probes the actual repository at every issue, so a token that cannot push there is refused with its reason.
- M1 A: `store.list()` per issue. Tokens are few; no measured cost.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Wire `GithubTokenSetting::from_store` into `DaemonSettings::from_entry` when the delivery issues mount token files into containers (render and `[[github.tokens]]` are done and tested).
- Persist the reach in `<data_dir>/github-reach.json` by token fingerprint (contract); Settings now shows reach only for the filter, or the listing of the token pasted this session.
