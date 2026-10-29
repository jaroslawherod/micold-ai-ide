# Autopilot ledger — #1523 daemon-config-file (Issue 2)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1523` (Daemon configuration schema and `micold-daemon config check`); labels: `flow:feature`, `effort:high`. User decision 2026-10-27: Spec, Clarify, Plan skipped; design already on main (PR #1509); tasks unit only cuts milestones for the "Issue 2" group (T005–T013, T087).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1523
- **Worktree branch**: feat/1523_daemon-configuration-schema-and-micold-daemon
- **Started**: 2026-10-27
- **Phase**: milestone M1
- **Next step**: wait for CI on the M1 PR, merge (orchestrator)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1551 | Design (milestones for Issue 2) | merged | 2b6335441e102258eb42b66f6cb607dc8e70d31c |
| #1600 | M1 | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T005–T013, T087 | full | Schema V1–V19, `config check`, user guide | #1600 | in review |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main | user | prompt 2026-10-27 |
| D2 | tasks | Split Issue 2? | No: one milestone. Review round 1: host example uses a T087 key, so a split leaves M1 unable to parse the examples; no acceptance scenario separates them (rule 3: keep whole) | agent-resolved | review round 1 F1-F4 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks 1523 | 1 | 71774ca9:09ee4cb1 | CHANGES: 1 BLOCKER, 3 MAJOR, all fixed (single milestone; T087 un-[P]'d and made implementation too); round 2 below |
| Tasks 1523 | 2 | 18c87a47:982c1a9d | CHANGES: 1 MAJOR (T087 position, moved before T010 so types/rules precede T012), 1 MINOR (ledger row, fixed); prose-only reorder, no round 3 |
| M1 A (code-review high) | 1 | c31afd77:2b633544 | 10 findings; fixed: V5 line, V7 api_key_file for copilot/pi, V8 element position, V18 normalisation + `.claude.json`, `idle_stop` message; rest declined below |
| M1 A | 2 | b98bb4c1:53ad9c6b | CLEAN; 2 MINOR left as is (V18 `.`; idle_stop u64 overflow message) |
| M1 B (conformance, sonnet) | 1 | - | cycle 20 lacked red; fixed (stub-red 10/10 fail, green 10/10), Verify observed: daemon test 10 pass, `config check host.toml` exit 0 |
| M1 B | 2 | b98bb4c1:53ad9c6b | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | V14/V15/V17 only under `--files` | The M1 Verify step needs `config check host.toml` to exit 0, and that fixture names files that do not exist; the app validates before the files are delivered. Documented in the user guide. |
| M1 | A r1 | Only the first structural error is reported | serde stops at the first unknown key or type error; semantic rules all run once the file deserializes. Documented in the user guide. |
| M1 | A r1 | V17 sees JSON MCP configs only | Codex/OpenCode MCP formats are Issue 11's; JSON covers `.mcp.json`. |
| M1 | A r1 | V14 ignores the directory mode and Windows | data-model V14 is about the file mode; the "ACL not checked" note belongs to the diagnostic (Issue 3). |
| M1 | A r1 | `--kind` mismatch only after a clean parse, at 1:1 | Minor; a mismatch is reported once the file is otherwise valid. |

## Handover

None.

RED CI attempt 1 (run 40756285827, Windows): tests-only fix, TOML literal strings for paths and the Windows default shell in the defaults test. Gate green.

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 2 (#1523)" headings; other sub-issues add their own.
- TDD test list (speckit-tdd-plan) deliberately not generated here; each milestone's implement run derives it for Issue 2.
