# Autopilot ledger — #806 container-daemon-ssh-provisioning

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 806 — issue #806 "Container daemon: configurable resources, Dockerfile or image source; build from current daemon binary by default", with the owner's scope change: "use basic image with ssh where you can copy daemon and install ai cli"; "should use same mechanism for local 'sandbox' container" (Dockerfile build dropped; shared SSH provisioning reused by #686). labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #806
- **Worktree branch**: claude/project-thread-uwrpqv
- **Started**: 2026-10-17
- **Phase**: plan
- **Next step**: plan unit. Clarify CLEAN after 2 rounds (D2-D4, agent-resolved). Plan must cover the first publish of the basic image before a client can pin its digest (publishing is an outward action: escalate category 3 if it needs a release or a manual push).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Dockerfile build or copy-in over SSH? | Basic image with sshd; client copies its daemon build over SSH and installs selected AI CLIs on start; same mechanism for the local sandbox container and #686 | user | owner decision 2026-10-17 (issue #806 comment, project thread) |
| D2 | clarify r1 | FR-002: where does the basic image come from? | Project-published (Containerfile minus daemon/CLIs plus sshd, existing release image jobs), version-independent, client pins by digest | agent | packaging/sandbox/Containerfile, .github/workflows/release.yml#image, spec Overview |
| D3 | clarify r1 | FR-010: default AI CLI selection? | New daemon: Claude Code only; every pre-upgrade container daemon: all supported CLIs | agent | Containerfile FR-023a, spec US4/FR-016, AiCli `#[default] ClaudeCode`, SC-002 |
| D4 | clarify r2 | Default AI CLI not selected / install failed? | Existing rule: start nothing, substitute nothing, say so where chosen | agent | spec 026 FR-002/FR-004 s4, unavailable_default_says_so.rs, spec 027 FR-023b |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 010544dfa6285dd3519917045f0d621c0e6d449e:70f5872667fb6e8e9126555f2573ce86e0a7bf80 | CHANGES: 3 MAJOR, 3 MINOR (all fixed; F4 fixed as `unreachable`, not `failed`) |
| Spec | 2 | 74cc41c423ab5d29a6175b050cf3e75df031fe9c:70f5872667fb6e8e9126555f2573ce86e0a7bf80 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| design | Spec r1 | F4 fix proposal: a new `failed` state | 489 FR-008 already defines `unreachable` as "a start that failed"; the spec now uses `unreachable` consistently instead of adding a state |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- specs/686-kubernetes-daemon-runtime FR-004, FR-012 and its clarifications still describe #806 as a Dockerfile build pushed to a registry; the owner's decision replaces that with SSH provisioning (806 FR-008). That spec belongs to the #686 flow.
