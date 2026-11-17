## Milestone M5 of 5 — kube-e2e (specs/1981-kube-e2e)

**Deliverable**: `network` and `ui` suites exist as documented placeholders that skip naming #1732 and #1954; selecting a suite runs only it.
**Verify**: `scripts/tests/e2e-kind.test.sh`; `MICOLD_E2E=1 cargo test -p micold-e2e --test ui -- --nocapture` prints the skip reason
**Tasks**: T031–T032 · **Satisfies**: US4 acceptance scenarios 1–2; FR-013, SC-005

What changed:
- `crates/micold-e2e/tests/network` and `tests/ui`: placeholder suites (own `[[test]]` targets) whose one test prints "skipped ...: blocked on #1732 / #1954".
- `scripts/e2e-kind.sh`: `PLACEHOLDERS` list; a placeholder suite is reported as `suite <name>: skipped, blocked on #<issue>` and runs no cargo test.
- `e2e/ui/README.md`: where the future Playwright suite goes and how it gets the base URL.
- `scripts/tests/e2e-kind.test.sh`: placeholder skip, single-suite selection, and every listed suite is a cargo test target.
- `docs/development/kube-e2e.md`: steps to turn a placeholder into a real suite.

Checks run here: `scripts/tests/e2e-kind.test.sh` (39 cases), `cargo clippy/test -p micold-e2e`, `cargo fmt --check`. The full gate is CI's.

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvMEuR3VFJvJPBEUTJ3
