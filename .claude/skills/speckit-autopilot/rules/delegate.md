# Delegating mechanical work

A subagent starts with a small, fresh context, and only its answer enters yours. Delegate a task
that takes several calls or reads a lot of output, and that needs no design judgment.

| Task | Subagent and `model` |
|---|---|
| Finding code, tracing a call path, listing uses of a symbol | `Explore` (Haiku) |
| A multi-step git job: rebase onto `origin/main` and resolve conflicts in files this flow owns, find which commit changed a line, compare branches by patch | `autopilot-worker`, `"sonnet"` |
| A straightforward implementation task in a `full` milestone: tasks.md or the BUG record names the file and the change, and it copies a pattern that exists in the repo | `autopilot-worker`, `"sonnet"`, with the task ID, the files, the pattern to copy and the test that must pass. Run that test yourself afterwards. |
| Checking static text: a doc or checklist against the spec, cross-references between spec artifacts, a log or report summarised to its failures | `autopilot-worker`, `"sonnet"` (`"haiku"` for a pure search or count) |

- Never spawn `general-purpose` from a unit or a helper: it carries about 10k more tokens on every
  call, and a hook blocks it. Only where the `autopilot-*` types do not exist is it the fallback.
- Keep one-command operations (`git status`, a single commit or push) in your own context: a
  subagent costs more than one call.
- Keep design choices, debugging and review findings on your own model.
- Tell the subagent exactly what to return and in how many lines, and check any change it made
  with `git diff --stat`.

## Triage a CI or job log in a worker, never in your own context

A raw job log read by the orchestrator or a unit is re-read on every later call. Dispatch an
`autopilot-worker` with `model: "sonnet"` and hand it the run or job id. Brief it with this recipe
and ask for at most ~40 lines back: the failing step, the failing tests, the decisive lines.

1. Get the step list: `gh run view <run> --json jobs` or, with no authenticated `gh`, the GitHub
   MCP `actions_list` (`method: list_workflow_jobs`, the run id) to find the failed job, then
   `actions_get` (`method: get_workflow_job`) for its steps. Pick the step whose conclusion is
   `failure`.
2. Fetch that step's log: `gh run view <run> --log-failed`, or MCP `get_job_logs` with
   `return_content: true` (blob URLs return 403 in the cloud). An oversized result is saved to a
   tool-results JSON file: `jq -r .logs_content <file> > "$SCRATCHPAD/job.log"`.
3. `grep -n -E 'FAILED|panicked|error'` the log file (`<log>` when handed a non-empty one, else
   `$SCRATCHPAD/job.log`), then `grep -n -C3` around the first hits. Never print the whole log.
4. Return: failing job and step, failing test names, the decisive lines. Nothing else.

The same applies to any long log a debugging unit needs (a gate log is already read with `grep`).
