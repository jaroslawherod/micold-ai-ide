# Autopilot ledger — #1725 daemons-dial-orchestrator

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1725 "feat: daemons dial the orchestrator (host directly; container, SSH and pod through the reverse forward)" (part of #1722; covers FR-003, FR-010; blocked by #1724, done). labels: flow:feature, effort:high
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1725
- **Worktree branch**: claude/project-thread-71llv0
- **Started**: 2026-11-15
- **Phase**: plan
- **Next step**: tasks unit (plan reviewed, 2 rounds, committed; no PR, ships in the design PR)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec review | 1 | spec | c5c4ee9eea8c5213009bd9e31ae289e13de2449d | 6580b0d5f69f5d16e0d94c289720723d0f9858a2:c5c4ee9eea8c5213009bd9e31ae289e13de2449d | CHANGES: 3 MAJOR fixed (pod path after N11 is mTLS, PROTOCOL_VERSION 82, FR-005 vs Q2 socket), 3 MINOR fixed; fixes are claim changes, not re-reviewed beyond self-check |
| Spec review | 1 | spec | c5c4ee9eea8c5213009bd9e31ae289e13de2449d | ce951a8ddb4e9f3252212e2563fb06d8ede5cca9:9f5fe99c4d2d09674f417cac5a47b1da6cfaa105 | CHANGES: 3 MAJOR, 3 MINOR, all fixed in 9f5fe99c4 (snapshot taken after the fixes) |
| Spec review | 2 | spec fix diff | 9f5fe99c4d2d09674f417cac5a47b1da6cfaa105 | ce951a8ddb4e9f3252212e2563fb06d8ede5cca9:9f5fe99c4d2d09674f417cac5a47b1da6cfaa105 | CLEAN: round-1 fixes hold (scoped, sonnet, diff against c5c4ee9ee) |
| Plan review | 1 | plan | 51d50295e396f5546030a0ebf185e54f3e96e8b6 | d346f6bd5384d30c245219fa412dd36f021b470f:edb515ca275e9528697839235b08037de5e4f1e7 | CHANGES: 4 MAJOR (ClearAllForwardings scope, stale live socket, encode_announcement NEW, prepare-link app side), 2 MINOR; all fixed |
| Plan review | 2 | plan fix diff | 51d50295e396f5546030a0ebf185e54f3e96e8b6 | f357f1489081de54ce559264732119f2099ffded:edb515ca275e9528697839235b08037de5e4f1e7 | CLEAN (scoped, sonnet); one MINOR comment fixed, prose only |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec review 1 | none | all six findings fixed |

## Handover

None.

## Open escalation

None. Plan mismatch with the revised epic (spec.md, "Plan mismatch") changes no goal; recorded, not escalated.

## Plan decisions (unit Plan 1725)

- sshd (packaging/basic): Unix-socket remote forward already allowed (AllowStreamLocalForwarding defaults to yes); plan adds `AllowStreamLocalForwarding remote` + `StreamLocalBindUnlink yes`. Without it: ssh fails the open with its message, no TCP fallback.
- Forward rides the bridge `ssh`; `remote prepare-link` (new) makes the remote dir, unlinks the old socket, prints the path. User-config hosts with configured forwards skip the link (ClearAllForwardings cannot be scoped).
