# Drift M6 (1db9333f1..8a2eaba41)

- `apps/client/e2e/stack/stack.spec.ts`, `apps/client/e2e/stack/stackProcess.ts`, `apps/client/playwright.stack.config.ts`, `apps/client/playwright.config.ts`, `apps/client/package.json`, `.gitignore`: the 3-browser stack e2e of T038 and its config.
- `.github/workflows/ci.yml`, `scripts/local-stack-kind-live.sh`, `crates/micold-client/tests/kube_real_support/mod.rs`: CI job `client-e2e-stack` (T039) and the kind live script it shares.
- `apps/client/src/components/ConnectionBanner.tsx`, `apps/client/tests/components/ConnectionBanner.test.tsx`: the unbound Retry the visual pass found, fixed with a test.
- `docs/development/client.md`, `docs/user-guide/web-client.md`: docs of T041.
