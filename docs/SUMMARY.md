# Summary

[Micold AI IDE](README.md)
[Installing Micold AI IDE](install.md)
[Licences](licences.md)

---

# User guide

- [The web client and the local stack](user-guide/web-client.md)
- [Tools for the AI in your sessions](user-guide/agent-tools.md)
- [The daemon configuration file](user-guide/daemon-configuration.md)
- [Running the session service in a container](user-guide/sandboxed-daemon.md)
- [Running the session service on another computer over SSH](user-guide/remote-daemons.md)
- [A cheap remote daemon on a private Tailscale network](user-guide/remote-daemon-tailscale.md)
- [Orchestrator mode: one registry of your daemons](user-guide/orchestrator-mode.md)
- [Orchestrator storage: local mode or PostgreSQL](user-guide/orchestrator-storage.md)
- [Running the session service in a Kubernetes cluster](user-guide/kubernetes-daemons.md)

# The session service

- [The Micold session daemon](daemon.md)

---

# Developing Micold AI IDE

- [The CI pipeline](development/ci-pipeline.md)
- [Diff guardrails: coverage and mutants](development/diff-guardrails.md)
- [The shared target directory and build lock](development/shared-build.md)
- [End-to-end tests on kind](development/kube-e2e.md)
- [The duplication budget](development/duplication.md)
- [Duplication triage](development/duplication-triage.md)
- [The documentation site](development/docs-site.md)
- [The client theme tokens](development/client-theme.md)
- [The client API types and drift check](development/api-types.md)
- [The web client](development/client.md)
- [Autopilot token usage](development/autopilot-tokens.md)
- [Autopilot evals](development/autopilot-evals.md)
