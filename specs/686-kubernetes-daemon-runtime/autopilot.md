# Autopilot ledger — #686 kubernetes-daemon-runtime

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #686 "Support a Kubernetes session daemon runtime" (with owner comment: align image source with #806); labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #686
- **Worktree branch**: claude/project-thread-xelisc
- **Started**: 2026-10-17
- **Phase**: clarify
- **Next step**: clarify round 1 awaiting owner answers (Open escalation: FR-004 image source + provider CLIs, FR-007 clone source); then apply, run next clarify round, then plan

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

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 7330b10a0e165a38d678df142737f8a7c1ea47d0:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 4 MAJOR, 3 MINOR — all fixed (F1 pod-gone → stopped + user Start; F2 FR-023 + multi-client edges, FR-016 identity; F3 kubeconfig/exec-plugin edges, FR-022; F4 per-phase bounds in FR-012; F5 Reconnect scenario; F6 provider CLIs in FR-004 marker; F7 version wording) |
| Spec | 2 (scoped, sonnet) | aed1af6291d0f65b2e21df970370031d92b506f6:171d36233aff56cab8399d2c2c08b7ed1c109dc2 | CLEAN — 1 MINOR fixed in clarify (SC-003, story 4.5, story 6.2 say "applicable phase bound (FR-012)") |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category: product decision only the owner can make; #806 has no spec yet and leaves both open):

1. FR-004 Kubernetes image source default. Options: (a) build locally per #806 (current daemon binary + selected provider CLIs) and push to a registry the user names, pull-an-image as the alternative (Recommended: keeps #806's version-matched default and its CLI install, one shared form; needs a registry field + push credential); (b) pull only, user names an image (simplest, but loses the version-matched default #806 asks for); (c) build inside the cluster (Kaniko/BuildKit job; needs privileges many namespaces deny, and a registry anyway).
2. Provider CLIs in the pod's image: (a) installed at build time in the image built under 1a (Recommended: same as #806); (b) installed at pod start into the volume (works with pull-only images, slower first start, needs egress from the pod); (c) the user's pulled image must already contain them.
3. FR-007 clone source. Options: (a) project's git remote, with an allowed push/pull credential; projects without a remote refused (Recommended: standard, resumable, no client upload; matches FR-008 credentials); (b) client's local repository sent over the tunnel (works without remote, carries unpushed commits, slower for big repos); (c) remote when present, else local transfer.

## Follow-ups not done

None yet.
