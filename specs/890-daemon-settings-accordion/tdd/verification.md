---
feature: 890-daemon-settings-accordion
verdict: PASS_WITH_GAPS
verified_at: f3d8a05a
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
independent: partial (smell pass by a fresh `autopilot-reviewer`; mutants and report by the closing unit)
mutation_tool: none (deliberate mutants, 3 sampled)
---

# TDD verification — 890

**PASS_WITH_GAPS** — every behaviour has a test through its real entry point and the deliberate
mutants were all caught, but M3 has no recorded red (honestly logged) and one existing test lost an
assertion (fixed here).

## Test-first evidence

| Group | Class | Note |
|---|---|---|
| M1 U1–U4, U7, U8 | PROVEN | red recorded in `cycle-log.md`, tests before header code |
| M1 U5, U6 | NOT_APPLICABLE | characterisation of today's header; no red by design |
| M2-1…M2-4 | PROVEN | red recorded (compile failures and assertion failures against the old `Section`) |
| M2-5 | LIKELY | red measured afterwards on `origin/main` (1851 > 800) |
| M3-1…M3-3 | TEST_AFTER | the cycle-log says no separate red was taken; tests were then checked to bite |

Existing tests: `tests/features_daemons*.rs` unchanged (SC-004). Assertions changed earlier, in M2 and M3 (#1075, #1082), and reported here, not edited:
`daemons_section_geometry.rs` (Kubernetes form: `Kubernetes`, `Allowed credentials`, `staging`
removed from one list — each now asserted by a new test, the `staging` button became a select by
design). **F1 below** was a real loss, fixed.

## Findings

1. **F1 HIGH (fixed)** `daemons_feature.rs` — `sections_open_by_toggle_and_by_error_…` dropped its
   discard-then-reopen assertion. Restored at the end of that test.
2. **F2 MEDIUM** `daemons_section_geometry.rs` Kubernetes list — see above; covered by the new tests.
3. **F3 MEDIUM** `daemons_feature.rs` `every_kind_opens_on_connection_only…` — no Host daemon in the edit
   loop; `only_connection` ends in `.all()`. Open, follow-up.
4. **F4 MEDIUM (fixed)** `features_daemon_sections.rs` — `false.then_some(Sessions)` now `None` with a reason.
5. **F5 MEDIUM** `features_daemon_sections.rs` — three tests hold many unlabelled `assert_eq!`. Open.
6. **F6 MEDIUM** `showcase_completeness.rs` — two tests check the static pose table only; the
   repo's completeness rule ties poses to the drawn view elsewhere. Open.
7. **F7 MEDIUM (fixed)** `daemons_section_geometry.rs` — `Manual start || Connects at launch` is now one exact summary.
8. **F8 MEDIUM** `daemons_section_geometry.rs` — SSH not in the header-gap check, bare tolerances. Open.
9. **F9 LOW (fixed in part)** dead `let _ = &mut w;` and unused `World` removed; the `if section == Connection` branch stays.
10. **F10 LOW** assorted (duplicate message, duplicate `TextField`). Open.

Open findings are test-style and add no behaviour; none changes a verdict on coverage.

## Mutation (deliberate, 3 sampled, none left in the tree)

Rows 1–2 were run by the closing unit and are recorded in this report only (each restored, suite re-run green); row 3 is the same mutant as M1's in the cycle-log, re-run here.

| Mutant | Result |
|---|---|
| `holding_error`: Kubernetes `namespace` moved to Placement | killed (`a_click_on_a_section_an_error_holds_open_is_not_recorded`) |
| `marked`: `req.empty` → `!req.empty` | killed (`the_marker_follows_the_required_fields`) |
| accordion header: summary drawn while open (drop `!s.open`) | killed (`an_open_section_paints_no_summary`) |

## Traceability

FR-002/003 → M2-1, M2-4; FR-004/005 → M2-2, M2-3; FR-008/009 → M3-1, M3-3; FR-011–013 → U1–U8;
FR-014 → M3-2; SC-002 → M2-5; SC-003 → M3-3; SC-004 → unchanged `features_daemons*.rs`;
SC-005 → U6. Each test named exists and ran green at `f3d8a05a` (gate).

## What was not audited

No mutation tool: three hand mutants only, none on `ui/settings/daemons.rs` view code. The
smell pass read the diffs; it ran nothing. Coverage and suite speed were not assessed beyond the gate.
