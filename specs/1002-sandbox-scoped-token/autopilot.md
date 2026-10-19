# Autopilot ledger — #1002 sandbox-scoped-token

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1002 — "Sandbox container with daemon should use a fine-grained token, not the user's GitHub token and SSH key" (body in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1002
- **Worktree branch**: claude/project-thread-2cdrmk
- **Started**: 2026-10-19
- **Phase**: design PR
- **Next step**: orchestrator opens the design PR (title/body in the unit's scratchpad), waits on CI and merges; then milestone M1 (T001–T013). Checklist requirements.md: all 16 items already ticked. Specs-only gate passed. D7 still provisional: if B/C, rework FR-005/008/009/010, SC-002/004 and M5 (plus FR-009 cap in M4 if credentials outlive 1 h).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | No user git identity in a sandbox (FR-001–003, FR-014) | — | planned |
| M2 | T014–T022 | full | Sanitized shared git config, helper reset (FR-004) | — | planned |
| M3 | T023–T029 | full | In-container helper + in-memory per-repo file (FR-006/007/015) | — | planned |
| M4 | T030–T038 | full | Protocol delivery, Schedule, revocation with FakeSource (FR-008–010, 012) | — | planned |
| M5 | T039–T049 | full | [source] GitHub App source + client wiring (FR-005) | — | planned |
| M6 | T050–T055 | full | Status rows + PR/issues toggles (FR-011, FR-013) | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which runtimes are in scope? | Container and Kubernetes daemons; remote SSH (user's own machine) out | agent-resolved | spec.md#Context |
| D2 | spec | Non-GitHub remotes? | No git credentials in a sandbox; old opt-ins not kept for them (least privilege) | agent-resolved | spec.md#Assumptions |
| D3 | spec | Default permissions? | contents rw + metadata r; PRs and issues per-daemon opt-ins, off by default | agent-resolved | spec.md FR-011 |
| D4 | spec | One credential per daemon or per repo? | Per repository | agent-resolved | spec.md#Assumptions |
| D5 | clarify | Keep old opt-ins for non-GitHub remotes? | No; no credentials, status says host unsupported (confirms D2) | agent-resolved | issue #1002 body ("rather than falling back") |
| D7 | clarify | Where does the scoped credential come from (FR-005)? | (A) user-registered GitHub App; client holds ID + key on device, mints 1 h single-repo installation tokens | provisional, pending jaro's answer on the decision card | spec.md#Clarifications; constitution.md#IV |
| D8 | clarify | Round 2 | No critical ambiguities; App key storage and setup UX deferred to plan | agent-resolved | spec.md |
| D9 | plan | Where does the App private key live on the device? | Sibling of data_dir (`micold-ai-ide-secrets/`), owner-only; data_dir is mounted whole into the legacy default sandbox | agent-resolved | research.md R5 |
| D10 | plan | Who renews while no client is connected? | Nobody: tokens lapse within 1 h, pushes fail closed, next connect re-grants (US3-4) | agent-resolved | research.md R8 |
| D6 | clarify | May a project-hosted service mint credentials? | No; client mints/holds on the device | agent-resolved | constitution.md#IV, #Technology constraints |
| D11 | tasks | Milestone cut vs plan's 3 | 6 milestones: plan's M1 split by FR (US1 removal / FR-004 sanitizer), plan's M2 split into git side (M3) and delivery (M4); the [source] GitHub App is confined to **M5** (was "M3" in the plan); M6 = status + FR-011 toggles. M1 (13 tasks) not split: the enum removal forces the client items to change in the same compile, and the `#[serde(skip)]` removal flag means the notice cannot lag the removal. M5 (11 tasks) not split: App settings without the wiring would be half-wired UI | agent-resolved | tasks.md#Milestones; milestones.md rules 3, 6 |
| D12 | tasks | speckit-analyze (14 findings, 2 HIGH) | All fixed: Verify/Satisfies aligned with quickstart steps; fixture placed in `sandbox/hostile_host.rs`; kept items enabled in FR-014 tests; old container git becomes `NoCredentialReason::GitTooOld` via `GitCredentialTargets.git_too_old` (plan Risks, contracts/protocol.md, data-model §6, spec Key entity updated — plan fix); `MAX_LIFETIME_SECS` const; two-renewal test; quickstart step 12 for FR-011; notice visual pass moved to M1 | agent-resolved | tasks.md, plan.md#Risks |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 98d24270a31995887551b4940385e98d06de202d:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | pending |
| Plan | 1 | c9fa31d9664b54c8a2795dccf16d38b1abf14079:6d822ec17c37bcde07f294ff2509e6682eda33e8 | CLEAN (4 MINOR, fixed, prose only) |
| Tasks | 1 | b6f5094100888a88679ecc6fe8c89bc1dc627695:3bbb1d8d627753e4926daa27fa7e7c7c12d3df48 | CHANGES (1 MAJOR: SourceNotSetUp App wording in M4, fixed via SOURCE_SETUP_HINT; 2 MINOR fixed) |
| Tasks | 2 | b248659e6d8d9fd04545078d5e6bf2c4668c4bb7:402875090f6540516eb724bb15368a2067a33956 | CLEAN (sonnet) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None in this unit. D7 awaits jaro's answer on the orchestrator's decision card.

## Follow-ups not done

- Jira API tokens live under `data_dir()/jira`, which the legacy default container daemon mounts whole as its state dir: readable inside that sandbox. Out of #1002's scope (not a git credential); worth its own issue.
