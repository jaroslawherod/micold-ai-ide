# Drift M5 (5ebc01334..1db9333f1)

- `deploy/helm/micold/templates/_helpers.tpl`, `deploy/helm/micold/templates/deployment.yaml`, `deploy/helm/micold/templates/orchestrator-config.yaml`, `deploy/helm/micold/templates/postgres.yaml`, `deploy/helm/micold/values.yaml`: chart changes the local values file of T044-T045 needs.
- `scripts/tests/chart-conformance.test.sh`, `scripts/tests/local-stack.test.sh`: existing script tests updated for the new chart values and the local-stack mode dispatch.
- `docs/user-guide/web-client.md`: documents `mise run local-stack`.
- MISSING `chart-check.test.sh`, `e2e-kind.sh`, `config_load.rs`, `daemon.toml`, `index.html`, `research.md`: named in tasks as references or reasons (for example "chart-check.test.sh drives fixture charts, so a new file"), not files to change.
- MISSING `scripts/tests/install-kube-tools.test.sh`: T043's pins needed no test change; helm stays v3.19.0 (Follow-ups).
- `.gitattributes`, `.github/workflows/ci.yml`, `Dockerfile.web`, `apps/client/e2e/serve.mjs`, `deploy/helm/micold/README.md`: the web image (T053-T054), its static server, line-ending pin for its scripts, CI job `local-stack-kind`, and chart docs for the local values.
