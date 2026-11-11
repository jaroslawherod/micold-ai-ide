# Quickstart: daemon-qualified refs and the routed envelope

## A. Automated

1. `scripts/build-lock.sh cargo test -p micold-core protocol_roundtrip handshake schema_hash routing project_identity`
   : refs, rows, envelope, mix-ups, refusal text, hash, identity rules.
2. `scripts/build-lock.sh cargo test -p micold-core contract_example`: the worked example frame in
   `specs/orchestrator/contracts/routed-envelope.md` decodes.
3. `scripts/build-lock.sh cargo test -p micold-daemon unwrapped_client`: a plain client still works
   and its rows carry `daemon`.
4. `mise run gate` before the push.

### Results (close unit, 2026-11-10, tree at origin/main after M4)

1. `protocol_roundtrip` (50 passed), `handshake` (10 + 13 passed), `schema_hash`, `routing` and
   `project_identity` (32 passed with `contract_example`, run as `--test main --test contract_example`).
2. `contract_example`: 2 passed (in the run above).
3. `micold-daemon --test unwrapped_client`: 2 passed. The filter form `unwrapped_client` matches no test
   name: `unwrapped_client` is a test target, so run it as `--test unwrapped_client`.
4. `mise run gate`: green in each milestone PR (#1753, #1767, #1789, #1790); the close PR's own gate result is in its body.

## B. Visual pass

None: no UI changes.

## C. Platforms

Tests are platform-neutral; CI runs all three.
