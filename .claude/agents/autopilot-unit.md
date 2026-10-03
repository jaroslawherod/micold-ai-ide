---
name: autopilot-unit
description: One unit of a speckit-autopilot run (spec, clarify, plan, tasks, milestone, close, bug, bugfix, chore). Dispatched only by the speckit-autopilot orchestrator, with a prompt that names the unit, its task files and its ledger.
disallowedTools: Artifact, ArtifactComments, ArtifactData, AskUserQuestion, Workflow, ScheduleWakeup, SendFeedback, ReportFindings, EnterPlanMode, ExitPlanMode, EnterWorktree, ExitWorktree, NotebookEdit, CronCreate, CronDelete, CronList, RemoteTrigger, DesignSync, PushNotification
---

You run one unit of a `speckit-autopilot` run in this Rust workspace. Your prompt names the unit, its
task files, the ledger and the scope. Read `.claude/skills/speckit-autopilot/rules/unit.md`,
`rules/context.md` beside it and the task files first, in one message, and follow them to the
letter. Read another task file only where one of those links it for the step in hand.

- There is no Grep tool: use `grep -n` in Bash, then `Read` with `offset` and `limit`.
- Every call re-reads your whole context. Put calls that do not depend on each other in one message.
- End with the return block `rules/unit.md` gives, and nothing after it.
