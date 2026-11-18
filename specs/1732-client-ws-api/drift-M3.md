# Drift M3 (a1c9514b3..a7d7a3c12)

- `crates/micold-core/src/client_api/mod.rs`, `crates/micold-core/src/client_api/schema.rs`: schema generation code behind the T021 binary.
- `crates/micold-core/tests/client_api_vectors.rs`, `crates/micold-core/tests/main.rs`: FR-012 vectors, in the existing core test binary.
- `packages/protocol/package.json`, `packages/protocol/tsconfig.json`, `packages/protocol/src/generated/client-api.ts`, `packages/protocol/src/index.ts`, `packages/protocol/src/types.ts`, `packages/protocol/src/wire.ts`, `packages/protocol/tests/order-vectors.test.ts`, `pnpm-lock.yaml`: the generated TS types, their package wiring and the jest side of the vectors.
- `docs/development/api-types.md`, `docs/SUMMARY.md`: how to regenerate the types.
- MISSING `micold-tokens-ts.rs`: named in T021 as the pattern to copy, not a file to change.
- `.github/workflows/ci.yml`: the schema/TS drift gate of T024 runs as a CI step.
