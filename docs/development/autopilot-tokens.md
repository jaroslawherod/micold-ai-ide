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
context grew. `rebuild_eq` is the part of `cost_eq` that cache rebuilds cost: requests that wrote
most of a large context to the cache again, after an idle wait or a compaction. The `output` column
is a lower bound: transcripts store usage from the start of the stream.

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

## The 034 runs at their end, 2026-10-01

`github-issue-worktree` finished (ledger `done`, record unit run): 42.7M `cost_eq` over 2,309
calls. `daemon-mcp-server` was at M5 of its milestones: 37.4M over 1,866 calls so far. Both still
mix skill versions, so these stay indicative.

| | Baseline | 034 github-issue | 034 daemon-mcp (so far) |
|---|---:|---:|---:|
| Orchestrator, whole run | ~5.3M | 4.06M | 4.04M |
| Orchestrator peak context | n/a | 174k | 195k |
| Milestone unit with its subagents, average | ~3.3M | 3.93M (7 units) | 5.67M (5 milestones) |
| Milestone unit peak context | 266k | 82–266k | 211–266k |
| First-round reviewer (session model) | ~0.27M | 0.08–0.27M | 0.11–0.24M |
| Re-review on Sonnet | n/a | 0.05M | 0.06–0.07M |
| `code-review` skill | ~0.25M | 0.15M | 0.28M |
| Record unit (Haiku) | n/a | 0.22M | n/a |

- **The orchestrator ended about 25% under the baseline.** The mid-run figures above (2.9M and
  1.9M) were low only because the runs were unfinished.
- **Re-reviews on Sonnet cost 0.05–0.07M**, against about 0.27M for a baseline review.
- **Milestone units did not get cheaper.** Feature difficulty dominates: daemon-mcp M3 cost 8.9M
  and M1 6.5M.
- **The 150k handover was mostly not followed.** github-issue's last two milestones stayed at 164k
  and 82k. But daemon-mcp M5, started after the rule merged, never checked its context in 144 calls
  and reached 266k; M4 checked once, at 202k, and did not hand over; only M3 handed over, after
  265k. The rule relied on the unit remembering to check.
- **Batching was uneven.** Lone reads fell to 5 and 1 in github-issue's last two milestones, while
  daemon-mcp M4 and M5 still made 22 and 38.
- **The close unit cost 6.6M with its subagents**, 3.2M of it two test-remediation subagents on the
  session model.
- `cost_eq` weights tokens the same on every model, so the record unit's 0.22M overstates its
  price on Haiku.

Because of the handover finding, `scripts/autopilot/context-hook.py` now runs as a PostToolUse
hook. On a branch an autopilot ledger names, it reads the caller's own transcript after each tool
call and, once the context passes the cap, tells a unit to hand over and the orchestrator to
suggest `/clear` and `resume`. It repeats only after another 20k of growth.

## Where the cost of the 034 runs went, 2026-10-01

`github-issue-worktree` ended at 42.8M `cost_eq` over 2,316 calls. `daemon-mcp-server` stood at
51.9M over 2,590 calls in its last milestone. Opus carried 88–92% of both.

**Cache rebuilds were a quarter of each run: 10.6M and 12.4M.** A subagent's prompt cache lasts 5
idle minutes and the main session's 60. Every request in the two runs that came after a longer gap
re-wrote its context; none that came sooner did:

| Idle gap before the request | Subagents: cache kept / rebuilt | Orchestrator: kept / rebuilt |
|---|---:|---:|
| 2 to 5 min | 76 / 0 | 27 / 0 |
| 5 to 5.5 min | 5 / 1 | 2 / 0 |
| 5.5 to 30 min | 3 / 90 | 58 / 0 |
| 30 to 60 min | 1 / 7 | 6 / 0 |
| over 65 min | 0 / 13 | 0 / 19 |

Units idled while the gate built (11 and 20 rebuilds), while a subagent or a forked skill ran or
after a turn ended (25 and 34), and while CI ran (16 and 3). Their context at a rebuild averaged 141k, so each cost about
175k. The orchestrator idled over an hour behind a unit or the human 8 and 11 times, at the main session's
doubled write rate: 1.5M and 2.4M.

`scripts/autopilot/hold.sh` now does the waiting. A unit calls it in the foreground; it returns when
the gate log has its result line, or after 4 minutes with `HOLD`, and the unit calls it again. Each
return is one call that reads the cache and so keeps it: about 12k at a 120k context, against 150k
for the rebuild. After 10 holds (40 minutes) it prints `STOP`, because 12 holds cost what one
rebuild does. The orchestrator runs `hold.sh --long` in the background while a unit works: it comes
back every 50 minutes, up to 12 times.

**Units passed 150k early.** Before the context hook, a milestone unit passed 150k at its 14th to
28th call and spent 80–95% of its cost above the cap (daemon-mcp M3: 7.8M of 8.2M). With the hook,
units handed over at 139–168k after about 50 calls, and three milestones ran as two parts. Cost per
call barely moved, 21–23k against 21–26k, because a part starts at 29k, reads `unit.md`, its phase
file, the ledger and two reference files to reach 55–60k before its first step, then adds about 2k
per call.

**What a unit starts with.** The first request of every subagent is 27–29k (system prompt, tool
schemas, `CLAUDE.md`, memory index, skill list), carried on about 2,300 calls: roughly 6M per run.
About 10k of it is the schema of the Artifact tool, which a `general-purpose` subagent of an
interactive session gets and a headless one does not (27.8k against 17.4k for the same one-call
task). Units, reviewers and delegated workers now run as the `autopilot-unit`, `autopilot-reviewer`
and `autopilot-worker` agent types in `.claude/agents/`, which disallow it and the other tools they
never call; the reviewer type also has no `Edit`, `Write` or `Agent`. Measured in an interactive
session with a prompt that runs no tool: `general-purpose` 27.8k, `autopilot-unit` 15.9k,
`autopilot-reviewer` 14.2k. A running session lists the types as soon as the files exist.
The daemon-mcp ledger had grown to 28 kB, about 8k tokens, of which the units' working state (PRs,
milestones, handover, escalation) is a tenth; the rest is decisions, review rounds and declined
findings that later units do not need. `scripts/autopilot/brief.py ledger <ledger> [M<K>]` now
prints the state in full and, of the history, only the rows that name the unit's milestone: 6.8 kB
of that ledger for M7, 3.5 kB without a milestone.

**Whole-file reads.** `Read` results were 50–58% of what the big units carried in tool output. The
largest single results were whole files of 10–16k tokens: `tasks.md`, `test-list.md`, `tools.rs`,
`worktree_form.rs`. `scripts/autopilot/read-hook.sh` now runs as a PreToolUse hook on `Read`: on a
branch an autopilot ledger names, it blocks a `Read` without `limit` of a file over 400 lines, and
of a ledger over 80 lines, and says what to do instead. A `Read` with `limit` always passes, so a
caller that needs a whole file asks for it by its length.

**Close-phase test remediation ran on the session model.** In `github-issue-worktree` the close
unit handed its test-strength findings to two subagents on Opus: 161 calls, 3.2M. The close phase
file now sends them to `autopilot-worker` subagents on Sonnet, one per crate, and the unit runs the
tests itself afterwards.

## After the hold, the agent types and the read hook, 2026-10-02

The first run that started with all of it merged is `038-issue-list-reporter-tooltip`, through its
spec and first clarify round: 43 requests, 550k `cost_eq`, no rebuild, no unbatched read. Its units
ran as `autopilot-unit` and `autopilot-reviewer` and started at 15–17k of context, against 27–33k
for the `general-purpose` units of the 034 close phase a day earlier. The spec unit held through
both review rounds with `hold.sh`, and the orchestrator held with `hold.sh --long`.

`autopilot-tokens.py --rebuilds` now lists every rebuild with the context it re-wrote, the idle
minutes before it and the tool calls it waited on, so the cause of a rebuild is read from the
report and not from the transcript. Of the 81 rebuilds of `daemon-mcp-server` (13.2M, nearly all
from before `hold.sh`), 30 followed a turn that ended, 11 a subagent's hand-back, 5 a forked skill,
and the rest a blocking shell wait.

What the two 034 runs still show, and what changed for it:

- **Unbatched reads: 3.8M and 5.9M, a tenth of each run.** 367 and 528 requests made one
  read-only call right after another at about 100k of context, two thirds of them `Bash` after
  `Bash` (a `grep`, then the next `grep`). 14 and 25 chains ran six or more such calls. The batch rules in
  `unit.md` did not stop it; the converge and visual-pass subagents, which never read `unit.md`,
  did it too. `context-hook.py` now also counts: after three requests in a row with one read-only
  call each, at 60k of context or more, it tells the caller once to batch its probes or send the
  search to an `Explore` subagent, and stays quiet for the next eight requests. The rule for
  "read-only" is the report's own (`read_only` in `autopilot-tokens.py`), so the hook warns about
  exactly what the `unbatched` column counts.
- **A forked skill blocks the unit's turn.** `hold.sh` cannot run while `visual-pass` or
  `speckit-tdd-verify` runs in the foreground: the close unit came back from one after 23 minutes
  and re-wrote 109k. `unit.md` now sends a forked skill that runs over 5 minutes through an
  `autopilot-worker`, whose own context is under 20k when it comes back, while the unit holds.
  The sentence adds 42 tokens to every unit call.

## Skill size

What the skill itself costs is fixed per role: every orchestrator call re-reads SKILL.md, and every
unit call re-reads `unit.md` plus its phase file. `mise run autopilot-skill-size` estimates each
role's share in tokens (bytes / 4), and `scripts/autopilot/measure-skill.sh <git-ref>` shows the
delta against a ref, so a change to the skill states what it adds to every call.

Against 65c6906c (all nine steps merged), after moving *Resuming* and *Handoff* to
`references/` and adding the model tiers and the delegation table:

| Role | 65c6906c | Now | Delta |
|---|---:|---:|---:|
| orchestrator | 3,078 | 2,664 | −414 |
| unit (milestone) | 2,368 | 2,773 | +405 |
| unit (other phases) | ~1,800 | ~2,150 | ~+357 |
| on-demand | 5,843 | 6,754 | +911 |

The unit's +357 is the delegation table in `unit.md`. It pays off when a unit delegates one
multi-call job instead of running it in its own context.
