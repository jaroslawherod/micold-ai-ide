# Quickstart: daemon-qualified refs and the routed envelope

## A. Automated

1. `scripts/build-lock.sh cargo test -p micold-core protocol_roundtrip handshake schema_hash routing project_identity`
   : refs, rows, envelope, mix-ups, refusal text, hash, identity rules.
2. `scripts/build-lock.sh cargo test -p micold-core contract_example`: the worked example frame in
   `specs/orchestrator/contracts/routed-envelope.md` decodes.
3. `scripts/build-lock.sh cargo test -p micold-daemon unwrapped_client`: a plain client still works
   and its rows carry `daemon`.
4. `mise run gate` before the push.

## B. Visual pass

None: no UI changes.

## C. Platforms

Tests are platform-neutral; CI runs all three.
