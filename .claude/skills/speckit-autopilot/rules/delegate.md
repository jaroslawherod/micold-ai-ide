# Delegating mechanical work

A subagent starts with a small, fresh context, and only its answer enters yours. Delegate a task
that takes several calls or reads a lot of output, and that needs no design judgment.

| Task | Subagent and `model` |
|---|---|
| Finding code, tracing a call path, listing uses of a symbol | `Explore` (Haiku) |
| A multi-step git job: rebase onto `origin/main` and resolve conflicts in files this flow owns, find which commit changed a line, compare branches by patch | `autopilot-worker`, `"sonnet"` |
| A straightforward implementation task in a `full` milestone: tasks.md or the BUG record names the file and the change, and it copies a pattern that exists in the repo | `autopilot-worker`, `"sonnet"`, with the task ID, the files, the pattern to copy and the test that must pass. Run that test yourself afterwards. |
| Checking static text: a doc or checklist against the spec, cross-references between spec artifacts, a log or report summarised to its failures | `autopilot-worker`, `"sonnet"` (`"haiku"` for a pure search or count) |

- An `autopilot-*` type your agent types lack: use `general-purpose`.
- Keep one-command operations (`git status`, a single commit or push) in your own context: a
  subagent costs more than one call.
- Keep design choices, debugging and review findings on your own model.
- Tell the subagent exactly what to return and in how many lines, and check any change it made
  with `git diff --stat`.
