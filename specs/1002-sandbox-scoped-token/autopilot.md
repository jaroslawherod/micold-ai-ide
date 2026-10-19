# Autopilot ledger — #1002 sandbox-scoped-token

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1002 — "Sandbox container with daemon should use a fine-grained token, not the user's GitHub token and SSH key" (body in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1002
- **Worktree branch**: claude/project-thread-2cdrmk
- **Started**: 2026-10-19
- **Phase**: tasks
- **Next step**: tasks unit (source-specific work is marked [source] in plan.md and grouped in milestone M3; if jaro picks B or C on D7, rework FR-005/008/009/010, SC-002/004 and M3 only)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 98d24270a31995887551b4940385e98d06de202d:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | pending |
| Plan | 1 | c9fa31d9664b54c8a2795dccf16d38b1abf14079:6d822ec17c37bcde07f294ff2509e6682eda33e8 | CLEAN (4 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None in this unit. D7 awaits jaro's answer on the orchestrator's decision card.

## Follow-ups not done

- Jira API tokens live under `data_dir()/jira`, which the legacy default container daemon mounts whole as its state dir: readable inside that sandbox. Out of #1002's scope (not a git credential); worth its own issue.
