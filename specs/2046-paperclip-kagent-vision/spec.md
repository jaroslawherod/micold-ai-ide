# Feature Specification: Prompt-started agents with roles on your own Kubernetes (MVP)

**Feature Branch**: `claude/project-thread-98230i`
**Issue**: #2046
**Created**: 2026-11-18
**Status**: Draft (design approved by jaro 2026-11-18; planning only, no source change)
**Design**: https://claude.ai/artifact/MixzAUfWyzum8bzTtjz8n4

**Input**: jaro, 2026-11-18: evolve Micold as a combination of Paperclip and kagent: easy deployment on
Kubernetes plus Paperclip-style agents with roles. Refined in the thread to: "Claude web in customer
infrastructure that runs with my personal subscription", navigating from high-level goals to low-level
tasks, supervised by default.

## Current target

**Prompt-started agents with roles, on Kubernetes.** A *control site* runs on jaro's own scalable
Kubernetes cluster (only he can access it): orchestrator, web client, daemons, unmodified Claude Code
with a memory-only token, worktrees. Each customer gets a small *edge* (`micold-edge` chart) in their
cluster that runs only MCP servers (Grafana, Kubernetes) and optional runners and dials out to the
control site over mTLS. The user writes a prompt, picks a Role and an item (GitHub/Jira issue or text),
and Micold starts an Agent. Agents form a parent/child tree (epic -> milestones -> items) that the user
navigates top-down. Later work is measured against this target.

**Out of scope**: heartbeats/automatic pickup, budgets, company/mission, a Micold ticket store, A2A,
kagent interop, multi-tenancy inside one install, team access (Epic B), production HA, confidential
containers, air-gapped customers.

## Clarifications

### Session 2026-11-18 (jaro)

- "papercli" = Paperclip. Take roles, delegation and the adapter idea; not company, mission, tickets, heartbeats, budgets.
- Kubernetes first: CRDs from day one, not deferred.
- Items come from GitHub/Jira as today. Assignment authority is Micold's database (agents may lack tracker write access); an identity label is an optional mirror.
- No kagent runtime or interop: it cannot use a Claude subscription. Borrow its CRD + controller model only.
- The daemon is the adapter. Micold never builds its own agent loop; it drives an unmodified AI CLI.
- Agents start only from the user's prompt. Sessions persist; context can be reset.
- Parent/child agents to any depth; the key feature is navigating from goals to tasks.
- Standing instructions at any node, inherited by the subtree; direct messages to one agent.
- Skills ship as a Claude Code plugin referenced by Roles.
- Supervised by default: an agent implements one milestone, then stops for review, feedback and approval. Automation only fixes CI failures, flaky tests, lint and conflicts within the milestone.
- Auto-merge is opt-in for non-production paths (CI config, tests, docs, tooling). Skill and plugin files are never auto-merged.
- Token and sessions run on the user's own host; any cluster several people can access is untrusted. Token stored memory-only; re-sign-in on restart.
- No usage cap: when the plan limit is reached, agents wait and resume after reset.
- Each person uses their own subscription; credentials are never shared. Micold stays free.
- Unsupervised work (e.g. developing Micold) stays in Claude on the web.

## User Scenarios & Testing

### User Story 1 - Start an agent on a customer item (Priority: P1)

jaro writes "fix #123" in the web client, picks the `engineer` Role and customer edge `acme`, and an
agent starts in a new worktree with the Role's context and acme's MCP servers.

**Independent Test**: on kind (control site + one edge), a prompt on a GitHub issue starts an agent
(mock provider in CI) that calls the Kubernetes MCP through the edge and opens a PR.

**Acceptance Scenarios**:
1. **Given** a Role and an Edge exist, **When** the user sends a prompt with an item, **Then** an Agent resource is created with `owner` = the user, a worktree on its own branch, and a running session.
2. **Given** a running agent, **When** it calls a tool of an edge MCP server, **Then** the call goes through the edge tunnel and the token never leaves the control site.
3. **Given** an agent, **When** the user sends a follow-up, resets context or stops it, **Then** the session reflects it and status is written back.

### User Story 2 - Break an epic into a tree and navigate it (Priority: P1)

jaro starts one agent on an epic; it plans and starts a child per milestone, which may start children
per item. jaro navigates the tree from the epic to any item's session.

**Acceptance Scenarios**:
1. **Given** a parent agent, **When** it calls `start_child(role, item, prompt)`, **Then** a child Agent with `parentRef` appears in the tree, within the Role's `maxDepth`/`maxChildren`.
2. **Given** a tree, **When** the user opens the epic node, **Then** each node shows rolled-up status (working, waiting for you, done, failed) and PR, with breadcrumbs back up.
3. **Given** standing instructions on a node, **When** a descendant runs its next turn, **Then** its context includes Role, ancestor and node instructions in that order.
4. **Given** a parent is stopped, **Then** its children stop.

### User Story 3 - Supervised milestone review (Priority: P1)

**Acceptance Scenarios**:
1. **Given** an agent finished its milestone and the quality gate passed, **Then** it stops in `Waiting` and the attention list shows it.
2. **Given** a waiting agent, **When** the user leaves line comments or feedback, **Then** the agent receives them and continues; **When** the user approves, **Then** it merges or continues to the next milestone.
3. **Given** CI fails or a test is flaky, **Then** the agent fixes it without asking and does not take on new scope.
4. **Given** `autoMerge: nonProduction` and a PR touching only declared non-production paths, **When** the gate passes, **Then** it merges; a PR touching skill/plugin files or any other path waits for the user.

### Edge Cases

- Plan usage limit reached: agents show `Waiting: usage limit` with the reset time and resume after reset.
- Control site restarts: token is gone (memory-only); the user is asked to sign in again; sessions resume after sign-in.
- Edge tunnel drops: MCP calls fail with a clear error; agent status shows the edge offline.
- A kubectl user creates an Agent with someone else's `owner`: the admission webhook rejects it.
- Tracker credentials are read-only: assignment is recorded in the database only; no label written.

## Requirements

### Functional Requirements

- **FR-001**: Micold MUST define CRDs `Role`, `ModelConfig`, `MCPServer`, `Edge` and `Agent` (`micold.dev/v1`), reconciled by a controller in the orchestrator with leader election and namespace-scoped RBAC.
- **FR-002**: CRs MUST hold desired state only; runtime data (assignments, transcripts, tree) lives in Postgres; only the controller writes `status`.
- **FR-003**: An admission webhook MUST set `Agent.spec.owner` from the requesting identity and reject a mismatch.
- **FR-004**: The daemon MUST run an unmodified AI CLI and build its context from the Role (prompt, CLAUDE.md/system-prompt append, MCP config, plugins), standing instructions, the user's prompt and the item.
- **FR-005**: The CLI's credential store MUST live on a memory-backed volume on the control site; a restart MUST require a fresh sign-in through Anthropic's own flow; Micold MUST NOT read, copy or relay the token.
- **FR-006**: The `micold-edge` chart MUST run only MCP servers and optional runners, dial out to the control site over mTLS, and expose MCP servers with read-only defaults and a per-Role ServiceAccount.
- **FR-007**: Agents MUST start only from a user prompt or from a parent's `start_child`; there is no scheduled pickup.
- **FR-008**: Micold MUST provide MCP tools `start_child`, `message_child`, `child_status` (extending #1945) and enforce `maxDepth`/`maxChildren` from the Role.
- **FR-009**: Standing instructions MUST be inheritable per node and injected on the next turn of every descendant.
- **FR-010**: Item assignment MUST be recorded in Postgres; a `micold-agent:<id>` label MAY be mirrored when tracker credentials allow.
- **FR-011**: Supervised mode (default) MUST stop each agent after one milestone for review; the user can comment, give feedback, approve or reject.
- **FR-012**: Within a milestone the agent MUST fix CI failures, flaky tests, lint and conflicts without asking.
- **FR-013**: Auto-merge MUST be opt-in (`autoMerge: nonProduction`) and limited to per-repository non-production path patterns; skill and plugin paths MUST never auto-merge.
- **FR-014**: The web client MUST show the agent tree as its main view with rolled-up status, attention markers and breadcrumbs, and open any node's session. UI strings MUST be internationalised.
- **FR-015**: On a plan usage limit, agents MUST wait with the reset time and resume after reset; the tree shows plan usage (#490).
- **FR-016**: Skills MUST ship as a Claude Code plugin from a Git marketplace, installed through Claude Code's own plugin mechanism.
- **FR-017**: The daemon MUST set `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` and `DISABLE_FEEDBACK_COMMAND=1` by default.

### Key Entities

- **Role**: prompt, modelConfigRef, mcpServerRefs, plugins, repos, limits {maxDepth, maxChildren}, autoMerge.
- **ModelConfig**: provider (claude-code|codex|opencode), model, credential (own sign-in | secretRef).
- **MCPServer**: url or image, headers secretRef, edgeRef, readOnly.
- **Edge**: customer name, client certificate, status (online, last seen).
- **Agent**: roleRef, owner, parentRef, edgeRef, item {source, ref}, prompt, repo, instructions, stopped, resetContext; status phase (Starting, Working, Waiting, Done, Stopped, Error), session, worktree, branch, pr.

## Success Criteria

- **SC-001**: On kind, `helm install micold` plus `helm install micold-edge` and one prompt produce a PR from an agent that used the Kubernetes MCP through the edge, in the CI e2e.
- **SC-002**: The Claude token never appears on any disk or in any edge pod (verified by test).
- **SC-003**: A three-level tree (epic, milestone, item) is navigable from the root to every session in at most three clicks.
- **SC-004**: No agent starts without a user prompt or a parent's `start_child` call (verified by test).

## Assumptions

- The control site is a Kubernetes cluster only the user can access (managed cluster or kind on one machine).
- Customers allow outbound access from the edge to the control site, and the user has their OK for data passing through Anthropic (training is off on jaro's account).
- Reuses: orchestrator + Postgres (#1948), daemon mTLS link (#1725), web client (#1735), Helm/kind (#1981, #1732 M5), parent/sibling tools (#1945), issue worktrees, plan usage (#490).
- Not legal advice; terms reading in the design's "Credentials and Anthropic's terms" section.
