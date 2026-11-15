# Cloud threads

A cloud thread (no `mise` or `gh`, or no `autopilot-*` agent types listed) is the orchestrator itself, with a large fixed context and no unit handover, so its
context is what costs. All other rules hold; these add to them.

- **Sonnet at low effort for mechanical threads.** A milestone, close-out or take-over (`resume`)
  thread is mostly dispatch, hold and merge: it runs on Sonnet at low effort, set at its first
  turn by the brief that starts it, or by the thread's first call (a later switch re-writes the
  whole cache). Close-out is the close unit's thread. Spec, plan, debugging and architecture threads
  stay on the session model. Units and reviewers keep the models their task files give, so pass
  `model: "opus"` where those say Opus.
- **One issue per thread.** Never brief several issues ("issues 1120-1123") in one thread.
- **A fresh thread per milestone.** After each merged milestone, write *Handover* in the ledger, commit, push,
  ask for a new thread that runs `/speckit-autopilot resume`, and stop.
- **Hand over at 150k** of context: do the same at once, whatever the step (the
  `context.md` cap, applied to the orchestrator).
- **Wait with `hold.sh`** ([waiting.md](waiting.md)), in the thread and in its subagents.
- **Pass `model`** on every `general-purpose` fallback ([../tasks/dispatch.md](../tasks/dispatch.md)).
- **Restart small before a long wait.** Before a question that may wait over an hour, a thread above
  ~150k writes its state (ledger *Handover*, or a doc for a planning thread), commits and pushes,
  asks the question, and says the answer should start a fresh thread with
  `/speckit-autopilot resume`: each reply after an idle hour re-writes the whole cache
  (see `tasks/ask.md` *Restart small*).
- **Follow-up questions** for a finished thread go to the project chat: a reply after an hour idle
  re-writes the whole cache.
- **Log usage before resolving.** Call `get_session` on yourself and pipe the JSON to
  `scripts/autopilot/usage-log.py` (appends a row to `/mnt/project-files/token-usage/threads.csv`).
