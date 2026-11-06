# Autopilot ledger — #1542 github-api-base-override

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1542: Base-URL override for the GitHub API and git host so the scoped-token checks run against a local fake; labels: none (orchestrator added in-progress, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1542
- **Worktree branch**: feat/1542_base-url-override-for-the-github-api-and-git-host
- **Started**: 2026-11-06
- **Phase**: implement M2
- **Next step**: user decision on step 11 (kind arm), then full gate (not yet run: build lock contended; the real-runtime test has not been run locally), PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1751 | Design PR | merged | 3093ca085b47775be01ad3dc71649271940a31d4 |
| #1755 | M1 | merged | 80e93599ba5f1c267a103600c8cd92904cf23bb2 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | ApiBase parameter, fake GitHub API, tests for Part C steps 2, 3, 8, 10, 12 | #1755 | merged |
| M2 | T010–T011 | full | Runtime-gated container/kind test (steps 4-8, 10, 11) | | pending |

## Decisions

- M1: the token-check HTTP types (`PatSource`, `UreqTransport`, `check_and_store`) moved from the binary's `shell::scoped_credential` to the library (`micold_client::pat_source`), because an integration test cannot reach `shell` (binary only). `shell::scoped_credential` re-exports `PatSource`. `PatSource::with_api_base` is the only way to give a source another base; no shipped caller uses it.
- M1: the #1002 step 3 scenario "token reaching a second private repository is refused" is stale since #1529 (FR-009 there: any number of repos accepted, reach listed). The fake-API test asserts the current behaviour (stored, reach listed).
- M1: the "Pull requests sentence" of step 12 is covered by the existing help sentence "The sandbox can use everything the token can, and the client cannot narrow it."; the help has no sentence naming Pull requests, and no product text was added.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| #1751 | Design PR | open | |
| Plan | 1 | 55f713a4:7bd55840 | CHANGES: 1 MAJOR (test targets unregistered), 3 MINOR; fixed (Cargo.toml [[test]], .gitattributes, FR-009 row, git backend R6), structural doc edits, self-checked |
| Spec | 1 | 3ce1418f:c695061be | CHANGES: 3 MAJOR; fixed (spec reworded, FR-004 kept open, SC counts, checklist), prose only |
| Tasks | 1 | a08fafa1:91e6af16 | CHANGES: 1 MAJOR (T006 [[test]] before file), 2 MINOR; fixed (registration folded into T007, dr_case, transport, checklist text), self-checked |
| M1 review A | 1 | 6ad156f1e8d55f53b91ae4dac6d36b56876288f4:3093ca085b47775be01ad3dc71649271940a31d4 (before fixes) | high: 9 findings (no BLOCKER/MAJOR label); fixed 5 (stale doc links x2, fake thread per connection and case-insensitive header, dead-port race, fake renamed `FakeGithubApi`); declined 4 (below) |
| M1 review B | 1 | 05a49d47:b886f8de6 | CLEAN (sonnet; core tests unobserved by B for build-lock wait, covered by the full gate) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1751 | Design PR | open | |
| M1 | A | `.gitattributes` marks `fake_github.rs` runtime but no daemon test includes it yet | Required by T006; M2 (T010) adds the daemon test that includes it. |
| M1 | A | `ApiBase::new` should reuse `reach::api_base` | Its semantics differ (empty means the default here; `api_base("")` gives `https:///api/v3`); a core unit test pins the literal. |
| M1 | A | `ApiBase::new` / `with_api_base` are public, empty falls back to production, http is accepted | Empty-is-default is the specified edge case; an injected parameter is FR-004's design and an integration test cannot use `cfg(test)`; no shipped caller passes a non-default base. |
| M1 | A | `on_device` and `check_github_token` defaults not observed by a test | `on_device` calls `new`, which a test covers; `check_github_token` passes `ApiBase::default()` in one line. |

## Handover

None.

## Open escalation

Category 5 (review B MAJOR, spec conflict): FR-008/US3 scenario 4/SC-002 require a kind-daemon arm (step 11). No kind pod harness is reachable from micold-daemon tests (it is micold-client `kube_real_*`, CI job `kube-cluster`, with no credential/git helper wiring). M2 as built covers the container arm (steps 4-8, 10) and an `#[ignore]`d step-11 placeholder.
Options: (A, Recommended) accept M2 without the kind arm, amend FR-008/US3.4/SC-002 to say step 11 is covered by a follow-up issue and quickstart row 11 stays UNCOVERED, open that follow-up; (B) add a kind arm in `micold-client` `kube_real_scoped_token.rs` (new pod harness work, a new milestone M3).
Review A (M2): CLEAN, minors fixed (port 17743, placeholder ignored). Review B: CHANGES on this only.

## Follow-ups not done

None.
