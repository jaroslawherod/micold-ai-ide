# Feature Specification: Micold as Paperclip plus kagent (vision, gap analysis, MVP)

**Issue**: #2046 · **Created**: 2026-11-18 · **Status**: Draft, research only (no source change)

**Input**: jaro, 2026-11-18: "I would like the micold app to evolve as combination of papercli and
kagent", "to allow easy of use deployment in k8s cluster + AI abilities that paperclip provides",
"try to find mvp", "and set it as a current target", "analyze what kagent has what abilities
paperclip has".

Reading: "papercli" = **Paperclip** (paperclipai/paperclip, paperclip.ing), an open-source
orchestrator for AI-agent "companies". **kagent** = the CNCF Kubernetes-native agent framework
(kagent-dev/kagent, Apache 2.0). If jaro meant something else by "papercli", sections 2 and 5 change.

## 1. Current target (MVP)

> **Current target: "one-command company on kind".** `mise run local-stack` (or `helm install` on any
> cluster) brings up the orchestrator, Postgres and the web client. In the web client the user creates
> a **Company** with a mission, defines **Agents** (role, reports-to, prompt, repo, monthly budget),
> and files **Tickets**. Each agent is a Claude Code session in its own worktree in a daemon pod,
> woken by a **heartbeat** that hands it its next ticket. The agent can create sub-tickets for its
> reports and close its own through Micold's MCP tools. Spend is metered per agent and the agent is
> paused at its budget cap. The user (the "board") can pause, resume or stop any agent.

Later work is measured against this target. Out of scope for the MVP: kagent CRDs and controller,
A2A, non-Claude harnesses beyond what the daemon already runs, multi-company tenancy and per-company
permissions, approval workflows beyond pause/stop, agent snapshots/suspend-resume, mobile build,
real-cluster production hardening (HA, autoscaling).

## 2. What each project offers

| Capability | Paperclip | kagent | Micold today | Take / integrate / build |
|---|---|---|---|---|
| Agent definition | Agent with role, prompt, adapter, in a company | `Agent` CRD (+ `AgentTemplate`) | Session (provider + worktree), no role | **Take Paperclip's model**; store in Postgres, CRD later |
| Org structure | Org chart, reporting lines, delegation down the tree | Subagents in a template; no org | Parent/sibling session tools (#1945) | **Build** org chart on #1945 parent links |
| Goals / mission | Company mission, tickets trace to it | none | none | **Build** (Company.mission, ticket → goal link) |
| Work items | Ticket board, sub-tickets | none | Worktrees from GitHub/Jira issues | **Build** tickets in Postgres; GitHub/Jira import later |
| Scheduling | Heartbeats: wake, check work, act, idle | Runs on request (chat/A2A) | Sessions run until stopped | **Build** heartbeat scheduler in orchestrator |
| Cost control | Monthly budget per agent, hard stop | none built in | Plan usage / session usage (#490, #944) | **Build** on existing usage metering |
| Governance | Human "board": pause/override/terminate | K8s RBAC on resources | owner/operator/viewer roles | **Take** pause/stop for MVP; approvals later |
| Model providers | Adapter per runtime (Claude Code, others) | `ModelConfig` CRD: OpenAI, Anthropic, Vertex, Ollama, gateways | Claude Code, Codex, OpenCode providers (#488) | **Keep** Micold providers; `ModelConfig`-like resource later |
| Tools | Through the agent runtime | `RemoteMCPServer` CRD, built-in k8s/helm/istio tools | Micold MCP server for sessions | **Integrate**: expose ticket/org tools over Micold MCP; MCP server registry later |
| Runtime / harness | Local process in working dir | Controller + pods; Go/Python ADK, Claude, Codex, custom | Daemon pods (#686), worktrees, terminals | **Keep** Micold daemon pods (real git worktrees are the differentiator) |
| Deployment | `npx paperclipai onboard`, embedded Postgres, local only | Helm chart, CRDs, in-cluster | Helm chart, kind local stack (#1981, #1732 M5) | **Keep** Micold Helm/kind; it is already the kagent-style half |
| Protocols | REST API | gRPC, A2A, MCP | WS client API (#1732), MCP | **Integrate A2A** later (agents callable by kagent agents) |
| Persistence | Postgres | Postgres (sessions/history) | Postgres store (#1948) | **Keep** |
| UI | Dashboard, org chart, ticket board | Web UI + CLI | RN web client (#1735) | **Build** company/agents/tickets screens in web client |
| Suspend/snapshot | none | Substrate: suspend, resume, snapshot | Session daemon keeps sessions alive | Later |

Summary: kagent brings **Kubernetes-native packaging** (CRDs, controller, Helm, A2A/MCP wiring);
Paperclip brings **the "AI company" abilities** (org, goals, tickets, heartbeats, budgets, board
control). Micold already has most of the kagent half (Helm, kind, orchestrator, daemon pods,
Postgres, MCP) and none of the Paperclip half. So the MVP is mostly Paperclip's domain built on
Micold's existing k8s stack, not an adoption of either codebase.

## 3. Adopt vs integrate vs build

- **Adopt code**: neither. Paperclip is a Node/TS local app with its own runtime model; kagent's
  runtime targets ADK agents, not interactive CLI sessions in git worktrees. Both would duplicate the
  orchestrator and daemon.
- **Adopt shape**: Paperclip's domain model (Company, Agent, Ticket, Heartbeat, Budget) and kagent's
  resource model (Agent / ModelConfig / MCP server as declarative resources, reconciled by a controller).
- **Integrate (post-MVP)**: A2A endpoint so kagent agents can hand tickets to Micold agents and back;
  optional CRDs (`micold.dev/v1 Company`, `Agent`) reconciled by the orchestrator so GitOps works.
- **Build**: everything in section 1.

## 4. Mapping of existing pieces

| Micold piece | Role in the new model |
|---|---|
| Orchestrator (+ Postgres #1948) | Company/agent/ticket store, heartbeat scheduler, budget enforcer |
| Session daemon in kube pods (#686, #1760) | Agent runtime: one agent = one session + worktree |
| Git worktrees | Agent's working directory per ticket (branch per ticket) |
| AI CLI providers (Claude Code, Codex, OpenCode) | Paperclip "adapters" |
| Parent/sibling session tools (#1945) | Delegation along reporting lines |
| Usage metering (#490, #944) | Budget accounting |
| Web API (#1732) + web client (#1735) | Board dashboard |
| Helm chart + kind stack (#1981, #1732 M5) | One-command deployment |
| iced desktop app | Being removed; not part of this design |

## 5. Proposed epics

**Epic A (MVP, current target): AI company on the cluster** — 4 milestones
1. Domain + store: Company, Agent (role, reports_to, prompt, repo, provider, budget), Ticket
   (status, assignee, parent, goal) in Postgres; WS API CRUD.
2. Runtime binding + heartbeat: orchestrator starts/wakes the agent's session in a daemon pod on a
   schedule, injects the next ticket; MCP tools `ticket_list/create/update/close` scoped to the org
   subtree.
3. Budgets + board control: per-agent monthly spend from usage metering, auto-pause at cap,
   pause/resume/stop in API.
4. Web client screens (org chart, ticket board, agent detail with spend), i18n, and a kind e2e that
   creates a company, files a ticket and sees an agent close it (mock provider in CI).

**Epic B: Kubernetes-native resources (kagent-style)** — 3 milestones
1. CRDs `Company`, `Agent`, `ModelConfig`, `MCPServer`; orchestrator reconciles them into the store.
2. Helm values to declare a company; GitOps example.
3. Remote MCP server registry: agents get tools from declared MCP servers.

**Epic C: Interop** — 3 milestones
1. A2A server endpoint per Micold agent. 2. A2A client: delegate a ticket to an external (e.g. kagent)
agent. 3. Optional: run a kagent deployment side by side in the kind stack, documented.

**Epic D: Governance and scale** — 3 milestones
1. Approval gates (e.g. merge/PR needs board approval). 2. Multi-company with per-company roles.
3. Goal tracking/reporting and cost dashboards.

## 6. Open questions for jaro

1. Is "papercli" Paperclip (paperclip.ing)? If not, which project?
2. Should one agent map to one long-lived session, or a fresh session per ticket (cleaner context,
   higher cost)?
3. Tickets: Micold-native only for the MVP, or synced with GitHub issues from the start?
4. Budget unit: dollars from API usage, or plan-usage percentage for subscription sign-ins?
5. Are CRDs (Epic B) wanted soon for GitOps, or is the web UI enough for now?
6. Should Micold interoperate with a real kagent install (A2A, Epic C), or only borrow its ideas?

## Sources
- Paperclip: https://github.com/paperclipai/paperclip , https://paperclip.ing/docs ,
  https://betterstack.com/community/guides/ai/paperclip-multi-agent/
- kagent: https://github.com/kagent-dev/kagent (README), https://kagent.dev
