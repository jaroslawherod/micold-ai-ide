## Milestone M6 of 10 — orchestrator PostgreSQL store (specs/1948-orchestrator-postgres-store)

**Deliverable**: No private key in any table
**Verify**: the key-material scan finds nothing on both backends; a PEM block is refused as a cert serial or token hash (tests in micold-daemon orchestrator suite and micold-core auth)
**Tasks**: T036–T038 · **Satisfies**: FR-006

## Agent review
- Review A (fresh-context, opus, round 1): APPROVE; 2 nits, both fixed (PEM[..64] slice; key_material scan limit documented).
- Review B (conformance, opus, same pass): APPROVE, clean.

## Local gate
`scripts/gate.sh` passed at ad4e9f7 (fmt, clippy, nextest with local PostgreSQL and MICOLD_REQUIRE_POSTGRES=1, script tests). No sandbox or macOS change.

Refs #1948
🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvM5tvrFC2G3xCa7faM
