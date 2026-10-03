# Phase 2: clarify

One round is one `speckit-clarify` run (at most 5 questions) and its triage. Triage each question:

- **The repo settles it** (constitution, earlier spec, contract, existing code): answer it and record
  `- Q: … → A: … _(agent-resolved: <path>#<section>)_`.
- **Otherwise the user decides.** Collect these for the round and return `ESCALATE` with them (up to
  4 questions, each with a `(Recommended)` option and its evidence). When continued with the
  answers, record them as `_(decided by user)_` and apply them to spec.md.

After a round's answers are in spec.md, run the next round yourself, in this unit: do not return
between rounds. Stop when a run reports no critical ambiguities, commit (do not push a PR; the
rounds ship in the design PR), and return `DONE` with `CLEAN` as the first summary line. A fifth
round is an escalation (category 5).
