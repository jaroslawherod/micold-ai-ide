# Phase 2: clarify round

One round is one `speckit-clarify` run (at most 5 questions) and its triage. Triage each question:

- **The repo settles it** (constitution, earlier spec, contract, existing code): answer it and record
  `- Q: … → A: … _(agent-resolved: <path>#<section>)_`.
- **Otherwise the user decides.** Collect these for the round and return `ESCALATE` with them (up to
  4 questions, each with a `(Recommended)` option and its evidence). When continued with the
  answers, record them as `_(decided by user)_` and apply them to spec.md.

Commit the round (do not push; it ships in PR 2). Return `DONE`, with `CLEAN` as the first summary
line when the run reported no critical ambiguities.
