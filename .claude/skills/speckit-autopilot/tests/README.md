# Testing the skill

`measure-skill.sh` and `autopilot-tokens.py` say what the skill costs, not whether its rules still
hold. These pressure scenarios do. Run them for any change that cuts, moves or rewords rules.

## Arms

Run each scenario in [pressure-scenarios.md](pressure-scenarios.md) as a fresh `Agent` on the model
the scenario names. One prompt = arm preamble + scenario + the closing line. Give `<worktree>` as an
absolute path: a subagent's shell may start in another worktree.

| Arm | Preamble |
|---|---|
| control | "You work in the micold-ai-ide repo at `<worktree>`. Do not read anything under `.claude/skills/`." |
| skill | "You work in the micold-ai-ide repo at `<worktree>`. You are running the `speckit-autopilot` skill as `<role>`: follow `<worktree>/.claude/skills/speckit-autopilot/SKILL.md` (orchestrator) or its `unit.md` and `phases/<phase>.md` (unit), and whatever they link that you need. Do not read anything under `.claude/skills/speckit-autopilot/tests/`." |
| old | as *skill*, pointing at an extracted copy of an older version (`git archive <ref> .claude/skills/speckit-autopilot`) |

- The control arm must fail a scenario at least sometimes, or the scenario tempts nothing. Harden
  it once; if the control still holds, keep it as regression-only and say so in the results.
- The skill arm must pass every scenario. A failure is a regression: quote the reasoning, fix the
  wording, rerun that scenario.
- Run the old arm only for scenarios the skill arm fails, to tell a regression from an old gap.

The control arm still sees CLAUDE.md and the user's memory, so it measures what the skill adds on
top of them.

To run them headless instead of as `Agent`s: `claude -p --model <model> --permission-mode plan
"<prompt>"` from the worktree, one process per arm and scenario, in parallel.

## Record

Append to [results.md](results.md): date, skill commit, arm, model, scenario, choice, pass, and the
deciding sentence of the reasoning for every failure.
