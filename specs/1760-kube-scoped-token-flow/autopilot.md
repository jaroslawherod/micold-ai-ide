# Autopilot ledger — #1760 kube-scoped-token-flow

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1760: Scoped-token step 11: assert the credential flow in a kind (Kubernetes) pod; labels: none originally, now flow:feature (switched from chore)
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1760
- **Worktree branch**: test/1760_scoped-token-step-11-assert-the-credential-flow-in
- **Started**: 2026-11-12
- **Phase**: spec
- **Next step**: clarify: apply the user's FR-008 answer to spec.md, then plan

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
|---|---|---|---|---|---|
| Spec review | 1 | spec.md | c1af633993e0d08fce04690906bf0074fe12df1d | dc49940e4d330da0d81222db5812aa2e88ce4a13:c1af633993e0d08fce04690906bf0074fe12df1d | CHANGES: 1 MAJOR declined (open markers go to clarify), 3 MINOR fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | spec review r1 | F1 MAJOR: three open markers (FR-008..010) leave scope unsettled | They are the template's allowed three; the rubric sends each to clarify. FR-008 is escalated there. Fixed F2, F3, F4 (prose/edge case). |

## Handover

None.

## Open escalation

Category 2 (security boundary), clarify round 1, FR-008.

**Question**: May a host-stored GitHub token be placed in a Kubernetes Secret (readable by anyone with secret read access in the namespace, stored in etcd), and with what consent?

**Options**:
1. Per-daemon opt-in in the daemon's settings, off by default (spec's recommendation; matches the opt-in pattern of sign-in and pull credential secrets).
2. Delivered whenever a matching token is stored: pasting the token in Settings is the consent (what docs/user-guide/kubernetes-daemons.md l.149-153 promises: "Once you have [pasted a token], the pod pushes and pulls"). No new setting.
3. Never by default; test-only injection.

**Recommendation**: option 2 would match shipped docs; option 1 is the safer boundary but changes the documented contract and adds a setting. Orchestrator to relay: recommend 1 per spec unless the user prefers the documented behaviour.

**Evidence**: specs/1002-sandbox-scoped-token/spec.md FR-005/FR-006 (token only for the container's own project, readable only inside it; a cluster Secret is readable by namespace admins, not covered); docs/user-guide/kubernetes-daemons.md l.65-68 (existing credential secrets, per-daemon opt-in), l.149-153; the old ClientMsg memory-volume delivery (1002 research R6) was retired in #1530. FR-009 and FR-010 were resolved from the repo and applied to spec.md.

## Follow-ups not done

- docs/user-guide/sandboxed-daemon.md promises a sandbox pushes with a pasted token, but no client code delivers `[[github.tokens]]` to a container (deliver_container) either; separate follow-up.
