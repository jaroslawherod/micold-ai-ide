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

## Where tool output goes

Measured over the same runs' 435 transcripts (main sessions and subagents): 16.8M tokens of tool
output, counted as characters / 4. Once in context, each result is re-read on every later call of
that session, so an early large read costs many times its size.

| Source | Share | Note |
|---|---:|---|
| `Read` | 41% | Rust sources 40% of it; spec artifacts read whole 70–95% of the time |
| `sed -n`, `cat`, `head` | 20% | File reads that bypass `Read` |
| `grep` | 12% | |
| `git diff`, `git show` | 6% | |
| Gate, CI and `gh` output | under 2% | Already detached to logs |

Within `Read`: spec.md 8%, tasks.md 7%, saved tool results and agent `.output` files 8% (94% read
whole), plan.md, research.md, the TDD files, and the constitution most of the rest.

What changed in response:

- `scripts/autopilot/brief.py milestone <feature-dir> M<K>` gives a milestone unit and its review B
  only the milestone's block, tasks, requirements and stories: about 13k characters instead of 99k
  for spec 031's spec.md and tasks.md.
- `unit.md` rules: read artifacts by section (`brief.py section|items`), code by `grep -n` then
  `Read` with `offset`/`limit`, and `grep` saved tool results and logs instead of reading them.

## Prompt cache

Measured over 53 sessions that ran `speckit-autopilot` (27,816 requests): cache writes are a third
of `cost_eq`, in the main session and in subagents alike. The report's `rebuilds` column counts
requests after a transcript's first that re-wrote more than half of a context of 30k tokens or more.

| Cause | Rebuilds | Cache-write `cost_eq` |
|---|---:|---:|
| Subagent idle 5 min to 1 h: its cache lives 5 min | 163 | 27.6M |
| Main session idle over 1 h: its cache lives 1 h | 117 | 23.9M |
| Compaction | 163 | 14.0M |
| Other (shorter gaps, prefix changes, subagent idle over 1 h) | 71 | 12.2M |

What the skill does about it:

- **Wait once.** A milestone unit runs review A while the gate builds, then review B and
  `visual-pass` together on the green build, so fewer of its waits run past 5 minutes. A job that
  needs the build lock never starts while the gate holds it, since it would idle there too.
- **Fixed prompt prefixes.** Every unit prompt opens with the same line, and reviewer prompts put
  role, reading rules, output contract and rubric before anything that varies.
- **One model per context.** Units and reviewers pick their model when dispatched; nothing switches
  model inside a running context.

Compaction is the next step: units that stay small never compact.

## Context size

Over the same 53 sessions, 100 of 490 subagents and 25 of 53 main sessions passed 150k tokens of
context. Requests above 150k cost 134M `cost_eq` in subagents (40% of their total) and 76M in main
sessions (37%). Each of those requests re-reads the whole context.

What the skill does about it:

- **Units hand over at 150k.** At each checkpoint a unit runs `scripts/autopilot/context.py
  "<its description>"`, which reads the size of its own last request from its transcript. Past the
  cap it writes a *Handover* into the ledger, commits locally and returns `HANDOVER`; the
  orchestrator starts a fresh unit of the same phase that continues from it.
- **Checkpoints sit between steps.** `speckit-implement` runs as one step, since its TDD hook
  drives every pending behaviour at once; a milestone that grows far past the cap inside it was cut
  too large (see the skill's `references/milestones.md`).
- **The orchestrator** checks its own size after each merge and, past the cap, tells the user that
  `/clear` then `/speckit-autopilot resume` would restart it small. The ledger is current at every
  merge, so nothing is lost.

## Tool-call batching

Over the same 53 sessions (29,590 requests), 85% of requests made exactly one tool call and 9% made
several. 39% made one read-only call (`Read`, or a shell command whose every step reads: `grep`,
`sed -n`, `cat`, `git status`, `gh … view` and the like). 1,925 runs of two or more such requests in
a row add up to 655M tokens of context re-read by the requests after each run's first: about 66M
`cost_eq` at the cache-read price. Some of those reads needed the one before, so not all of it can
be saved. The report's `unbatched` column counts them per unit.

What the skill does about it:

- **Batch rules** in `unit.md`, the reviewer prompt and the orchestrator: independent reads in one
  message, dependent shell probes chained in one command, `grep -n -C5` instead of `grep` then
  `Read`.
- **`scripts/autopilot/checkpoint.sh`**: at each checkpoint a unit gets branch, changes, unmerged
  commits, ledger state and context size from one call.

## First runs with the changes, 2026-09-30

Three runs were in flight while the optimizations merged: spec 034 `github-issue-worktree`, spec
034 `daemon-mcp-server`, and spec 035 `report-missing-include-script`. They mix skill versions.
Each orchestrator loaded the old `SKILL.md` when it started on 09-29 and kept it, while each unit
read `unit.md` and its phase file from disk when it started. Read these numbers as indicative;
a run started after the last change is the clean measurement.

Per unit kind, in `cost_eq` (baseline per unit is its share of the baseline total over its unit
count, so it is approximate):

| | Baseline | 034 github-issue | 034 daemon-mcp | 035 |
|---|---:|---:|---:|---:|
| Orchestrator per run | ~5.3M | 2.9M (6 milestones) | 1.9M (5 milestones) | not separable (shared session) |
| Reviewer | ~0.27M | 0.18M | 0.14M | 0.04–0.23M |
| `code-review` skill | ~0.25M | 0.15M | 0.26M | 0.10–0.19M |
| Milestone unit | ~3.3M | 2.97M | 3.04M | 0.98–2.04M |
| Milestone peak context | 266k | 266k, then 123k | 266k, then 139–172k | 233k, then 132–192k |

Milestone units by the skill version they started on:

| Started | Units | `cost_eq` per task |
|---|---|---|
| Before steps 3–8 | 034 github-issue M1, M2 | 0.19–0.20M |
| After the per-step reads, model tiering, reviewer and cache changes | 034 github-issue M3–M5, 034 daemon-mcp M1–M3, 035 M2–M4 | 0.13–0.46M, mostly near 0.2M |
| After the 150k handover | 034 github-issue M6a, 034 daemon-mcp M5 (unfinished) | 0.95M and 1.75M in total, peak under 172k |

What they show:

- **Peak context fell** once the handover landed: 123–172k, against 214–266k before. No unit has
  handed over yet; none reached the cap at a checkpoint.
- **Reviewers cost 30–85% less** each. Scoped re-reviews on Sonnet cost 0.05–0.08M.
- **The orchestrator costs about half the baseline** per run. It still followed the old
  `SKILL.md`, so this comes from moving work into units and scripts.
- **Milestone cost per task shows no clear change.** How hard a feature is outweighs the skill
  changes (034 daemon-mcp M2: 0.46M per task over 8 tasks). Batching had not merged: milestone units
  still made 117 and 166 lone reads right after another read.
