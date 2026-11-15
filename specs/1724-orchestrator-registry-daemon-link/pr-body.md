Closes the spec for #1724 (part of #1722); all milestones M1 to M6 are merged.

- Plan drift recorded for M1, M2, M4 and M5 (`drift-M*.md`); `plan-drift.sh` exits 0 for every milestone. Converge skipped: every milestone has a CLEAN review B.
- `speckit-tdd-verify` returned FAIL (test-after M1 and M2, surviving mutants). Fixed without new behaviour: timeout and loop-exit tests in `tests/orchestrator/link.rs`, a guard for the client `LinkPing` arm, store.rs test cleanup, deterministic heartbeat tests, unused `preamble_expired` removed. Retroactive reds and accepted deviations are in `tdd/cycle-log.md` and `tdd/verification.md`.
- The netns proof now fails where `strace` or user namespaces are missing only when `CI` is set; elsewhere it skips with a warning.
- Spec `Status` set to Closed.

Local gate: scoped only (fmt, clippy -D warnings on core, daemon and client; orchestrator tests: core 80, daemon 49, client daemon_sync 66), full gate left to CI.

Closes #1724

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvKLn5kiQKJmWy61ZPt
