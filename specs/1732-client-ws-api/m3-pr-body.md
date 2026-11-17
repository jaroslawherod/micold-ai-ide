## Milestone M3 of 6 — client WS API (specs/1732-client-ws-api)

**Deliverable**: `scripts/api-types.sh` regenerates the checked-in JSON Schema and TS types from the Rust types; the gate and CI fail when stale; shared sidebar-order vectors pass in Rust and jest.
**Verify**: `scripts/api-types.sh --check` (fails after editing a type without regenerating) and `pnpm --filter @micold/protocol test`
**Tasks**: T020–T026 · **Satisfies**: US4 scenarios 1–3; FR-009, FR-010, FR-012; SC-004

- `micold-core`: `client_api::schema` (draft-07, root `anyOf` ClientRequest | ServerMessage, every wire type under `definitions`) and the `micold-api-schema` bin.
- Checked in: `packages/protocol/schema/client-api.schema.json`, `packages/protocol/src/generated/client-api.ts` (json-schema-to-typescript 16.0.0, new dev dependency of `@micold/protocol`), `packages/protocol/tests/vectors/sidebar-order.json`.
- Drift: Rust tests `client_api_schema` and `client_api_vectors`; `scripts/api-types.sh --check` in `scripts/gate.sh`; `--check --ts-only` plus `scripts/tests/api-types.test.sh` in CI's web client job; `mise run api-types`.
- FR-012: vectors (grouping and nesting, daemon offline and back, remove-then-upsert) built with the orchestrator's `diff`, replayed by `order-vectors.test.ts` through `applyChanges`; `src/wire.ts` maps the snake_case wire types to the client's types.
- Docs: `docs/development/api-types.md`.

## Agent review
- Review A (code-review): run by the orchestrator.
- Review B (conformance): run by the orchestrator.

## Local gate
GATE_LINE

Refs #1732
https://claude.ai/code/session_01NCdYvPnNa4tmPz4FzNinb7
