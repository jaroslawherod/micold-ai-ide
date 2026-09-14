# Spec Kit autopilot

`/speckit-autopilot <feature idea>`, `/speckit-autopilot bug: <report>` or
`/speckit-autopilot resume`.

You give it one prompt. For a feature, the agent writes the spec and merges it, clarifies it with
you, plans and cuts the work into milestones, then ships each milestone to `main` as its own
reviewed PR. For a bug, it reproduces it, records it against the spec that owns the behaviour, and
ships the regression test and fix as one PR. It reviews every artifact and every diff itself; no
human does. It asks you only for decisions that are yours (see
[When you are asked](#when-you-are-asked)). When everything has merged, it tells you the worktree
can be removed.

The agent follows [SKILL.md](SKILL.md). This page is the human-facing overview.

## The flow

```mermaid
flowchart TD
    START(["Your prompt: feature idea or bug report"]) --> TRIAGE{"Feature or bug?"}

    TRIAGE -->|bug| BUGPATH[["Bug path, see A bug report below"]]
    BUGPATH -->|"no owning spec, or new behaviour"| SPEC
    BUGPATH -->|"fixed and merged"| DONE

    TRIAGE -->|feature| SPEC["speckit-specify"]
    SPEC --> SREV["Fresh-context reviewer: spec rubric"]
    SREV -->|"changes, max 3 rounds"| SPEC
    SREV -->|clean| PR1["PR 1: spec, merged on green"]

    PR1 --> CSCAN["speckit-clarify, one round"]
    CSCAN --> CQ{"Questions left?"}
    CQ -->|none| PLAN
    CQ -->|yes| CTRI{"Settled by the repo? constitution, specs, code"}
    CTRI -->|yes| CAUTO["Agent answers and cites the evidence"]
    CTRI -->|"no: a real decision"| H1["ACTION REQUIRED: batched questions, each with a recommendation"]
    CAUTO --> CSCAN
    H1 --> CSCAN

    PLAN["speckit-plan"] --> PREV["Reviewer: plan against constitution and spec"]
    PREV -->|changes| PLAN
    PREV -->|clean| TASKS["speckit-tasks"]
    TASKS --> MCUT["Cut milestones: each one ships a deliverable"]
    MCUT --> ANA["speckit-analyze, tasks and milestone review, close checklists"]
    ANA -->|"changes, max 3 rounds"| PLAN
    ANA -->|clean| PR2["PR 2: clarifications, plan, tasks, milestones, merged on green"]

    PR2 --> MLOOP[["Milestone loop, one PR per milestone"]]
    MLOOP --> MORE{"More milestones?"}
    MORE -->|yes| MLOOP
    MORE -->|no| CLOSE["speckit-converge, tdd-verify, docguard-guard"]
    CLOSE -->|"unbuilt work: new milestone"| MLOOP
    CLOSE -->|complete| FIN["Final PR: spec status Closed"]
    FIN --> DONE(["WORK COMPLETE: PRs listed, decisions summarised, worktree safe to remove in micold IDE"])

    ESC["ACTION REQUIRED: escalation"]
    SREV -.->|"not converging"| ESC
    ANA -.->|"not converging"| ESC
    MLOOP -.->|blocked| ESC

    classDef human fill:#ffe0e0,stroke:#c00,stroke-width:2px,color:#000
    classDef pr fill:#e0f0ff,stroke:#06c,color:#000
    class H1,ESC human
    class PR1,PR2,FIN pr
```

### One milestone

```mermaid
flowchart TD
    M0["Reset the worktree branch to origin/main, once the previous PR is MERGED"] --> M1["speckit-implement, limited to this milestone's tasks"]
    M1 --> M2["tdd-run: red, green, refactor for each behaviour"]
    M2 --> GATE["mise run gate: fmt, clippy, tests, shell suites"]
    GATE --> EXTRA["If cfg changed: macOS cross-check. If visuals changed: visual-pass"]
    EXTRA --> REV["Review A: code-review high. Review B: fresh subagent checks deliverable, scenarios, constitution"]
    REV -->|"real findings, max 3 rounds"| GATE
    REV -->|clean| PR["Ledger updated, push, PR with deliverable and review summary"]
    PR --> CI{"ci complete"}
    CI -->|"red in this flow's code"| FIX["systematic-debugging, fix, push"]
    FIX -->|"attempts 1 to 3"| CI
    FIX -.->|"attempt 4"| ESC["ACTION REQUIRED"]
    CI -.->|"red outside this flow"| ESC
    CI -->|"no checks"| DIAG["Conflicting? Awaiting approval? Outage?"]
    DIAG --> CI
    CI -->|green| MERGE["gh pr merge --rebase, no branch deletion, confirm MERGED"]
    MERGE --> SHIP(["Milestone shipped on main"])

    classDef human fill:#ffe0e0,stroke:#c00,stroke-width:2px,color:#000
    class ESC human
```

### A bug report

```mermaid
flowchart TD
    B0(["bug: your report"]) --> REPRO["systematic-debugging: reproduce on origin/main, with variations"]
    REPRO --> RQ{"Reproduces?"}
    RQ -.->|no| ESCR["ACTION REQUIRED: the missing reproduction detail"]
    ESCR -.->|"your answer"| REPRO
    RQ -->|yes| OWN{"Which spec owns the broken behaviour?"}
    OWN -->|none| FEAT["Feature path from speckit-specify, citing the reproduction"]
    OWN -.->|"a feature still in flight"| ESCO["ACTION REQUIRED: blocked by work outside my flow"]
    OWN -->|"a Closed spec"| REP["bugfix-report: BUG-k.md with root cause and false completions, ledger beside it"]
    REP --> PATCH["bugfix-patch: missing requirement, reopened tasks, fix tasks"]
    PATCH --> VER["bugfix-verify, then fresh reviewer: bug rubric"]
    VER -->|"changes, max 3 rounds"| PATCH
    VER -->|clean| SIZE{"New behaviour, or more than 10 tasks?"}
    SIZE -->|yes| FEAT
    SIZE -->|no| RED["Regression test fails on origin/main for the reported reason"]
    RED --> MS[["One milestone: fix, gate, reviews A and B, PR fix(NNN) BUG-k, CI, merge"]]
    MS --> BDONE(["WORK COMPLETE: BUG-k fixed, worktree safe to remove in micold IDE"])

    classDef human fill:#ffe0e0,stroke:#c00,stroke-width:2px,color:#000
    classDef pr fill:#e0f0ff,stroke:#06c,color:#000
    class ESCR,ESCO human
    class MS pr
```

A bug gets no spec PR, design PR or close phase: the BUG record, the spec patch, the regression test
and the fix all ship in one PR. If the fix turns out to be new behaviour, the bug becomes the input
to a normal feature flow.

## When you are asked

Every question arrives as a `🛑 ACTION REQUIRED FROM YOU` banner and a push notification. The banner
says:
- what is needed
- why the agent cannot decide it
- what it already checked
- what it recommends
- what stays paused while it waits

Questions are batched. You are asked only when one of these applies:

1. A product or scope decision that the repo does not already settle.
2. A conflict with the constitution.
3. An irreversible or outward action beyond merging the flow's own PRs: a release, a settings or
   ruleset change, secrets, deleting data.
4. Access the agent lacks.
5. Something that will not converge: a review after 3 rounds, CI after 3 fixes, clarification after
   4 rounds, a bug that will not reproduce. This includes a flow blocked by work outside it.
6. The plan proved false in a way that changes requirements.

Everything else the agent handles itself: review findings, lint and test failures, merge conflicts,
flaky CI, visual checks, and implementation choices. Details:
[references/escalation.md](references/escalation.md).

## What the agent owns

The agent owns only the work its own flow created:
- this worktree and its branch
- the feature directory it created, or for a bug, the BUG record it filed and its patch to the
  owning Closed spec
- the PRs recorded in its ledger

It never merges, reviews or fixes other sessions' PRs, specs or worktrees. It never deletes the
worktree or the branch. Removing the worktree in micold IDE cleans up both.

## Resuming

Progress is kept in `specs/<NNN>-<slug>/autopilot.md`, or for a bug in `bugs/BUG-<k>.autopilot.md`
beside the BUG record. It is committed with every PR and holds the phase, PRs, milestones, every decision (and who made it), declined review findings, and follow-ups.
After a crash or `/clear`, run `/speckit-autopilot resume` in the same worktree. It finds the run
itself, from the unfinished ledger that records this worktree's branch.

## Files

| File | Holds |
|---|---|
| [SKILL.md](SKILL.md) | The phases, the milestone loop, ownership, the handoff, red flags |
| [references/escalation.md](references/escalation.md) | When and how the human is asked |
| [references/milestones.md](references/milestones.md) | How tasks become deliverable milestones |
| [references/review-rubrics.md](references/review-rubrics.md) | Reviewer dispatch and the rubric for each artifact and for code |
| [references/pr-and-merge.md](references/pr-and-merge.md) | Local gate, PR format, waiting for CI, merging, known failure modes |
| [templates/autopilot-ledger.md](templates/autopilot-ledger.md) | The progress ledger |
