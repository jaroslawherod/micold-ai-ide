# Issues for #2046

Epic A: Prompt-started agents with roles on your own Kubernetes (MVP). Each milestone is run with
`/speckit-autopilot #<issue>`.

1. #2051 **M1: Control site and customer edge** - `micold-edge` chart, outbound mTLS tunnel (reuse #1725 link), MCP proxied from edge, memory-only token volume, Edge CRD, sign-in flow.
2. #2052 **M2: Agent CRDs and controller** - Role, ModelConfig, MCPServer, Agent; reconcile into worktree + session; owner admission webhook; status write-back; Postgres assignment.
3. #2053 **M3: Daemon adapter, plugin and agent tree** - context from Role + standing instructions, `micold-agent` plugin, `start_child`/`message_child`/`child_status`, limits, reset/stop, usage-limit waiting.
4. #2054 **M4: Supervised review and web tree** - goal-to-task tree view, attention list, milestone stop, diff feedback/approve, autoMerge nonProduction, kind e2e (SC-001..SC-004), docs.

Epic B (later): scale-out control site (pod per agent, node autoscaling), team access, more MCP servers.
