Drift M1: the supervised argument text is in rules/supervised.md, not SKILL.md.

- `.claude/skills/speckit-autopilot/rules/supervised.md` — UNPLANNED: SKILL.md is at the 60-line module ceiling (autopilot.test.sh), so the text lives in a rule file linked from SKILL.md; T007/T008 name it.
- `.claude/skills/speckit-autopilot/flows/choose.md` — UNPLANNED: a supervised run must not ask the Flow question before its ledger exists (review A F3).
- TDD: no `tdd/` cycle-log; every task pair is test-first in `scripts/tests/supervisor.test.sh` (tests written and run red before `supervisor.sh` existed); recorded here instead of a separate log.
