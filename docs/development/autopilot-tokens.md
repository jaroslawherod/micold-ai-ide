# Autopilot token usage

`scripts/autopilot-tokens.py` reads Claude Code session transcripts and reports token use per
unit: the orchestrator, each subagent, and each subagent's own reviewers and forked skills.

```bash
mise run autopilot-tokens                           # every session of the current worktree
scripts/autopilot-tokens.py <session-id|path.jsonl> # one session
scripts/autopilot-tokens.py --json …                # machine-readable
```

Read `cost_eq` first. It weights every token by its API price ratio (input 1, cache write 1.25 or 2,
cache read 0.1, output 5), so one number compares runs. `peak_ctx` shows how large one unit's
context grew. The `output` column is a lower bound: transcripts store usage from the start of the
stream.

## Baseline, 2026-09-29

32 micold sessions that ran `speckit-autopilot`, before any token optimization. 444M `cost_eq` in
total.

| Unit kind | Units | Calls | Cache read | Share of `cost_eq` | Peak context |
|---|---:|---:|---:|---:|---:|
| Orchestrator (main session) | 32 | 7,091 | 893M | 38% | 267k |
| Unlabelled Sonnet subagents (mostly forked skills) | 76 | 4,864 | 528M | 15% | 266k |
| Unlabelled Opus subagents | 21 | 2,649 | 345M | 11% | 267k |
| Milestone units | 15 | 2,100 | 312M | 11% | 266k |
| Reviewers (artifact, bug rubric, review B) | 179 | 3,407 | 261M | 11% | 251k |
| Bug units | 21 | 2,177 | 245M | 9% | 266k |
| `code-review` skill | 53 | 954 | 60M | 3% | 160k |
| Close, design, spec, clarify units | 14 | 644 | 76M | 3% | 266k |

What it says:

- **The orchestrator is the largest cost.** It should only keep the ledger, ask, wait and merge, yet
  it makes a third of all calls and reaches 267k context. Older runs predate the orchestrator split,
  so part of this is work that now runs in units.
- **Cache reads are about 90% of the input side.** Cost follows calls × context size, so fewer calls
  and smaller contexts matter more than shorter output.
- **Many units reach 250k+ context.** A unit that long re-reads a full context on each call.
- **Subagents without a `description`** show up as `general-purpose` and cannot be attributed. Give
  every dispatch a description that names its unit.

Re-run the same command after each optimization and compare against this table.
