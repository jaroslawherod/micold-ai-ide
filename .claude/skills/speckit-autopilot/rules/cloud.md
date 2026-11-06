# Cloud threads

A cloud thread (no `mise` or `gh`, or no `autopilot-*` agent types listed) is the orchestrator itself, with a large fixed context and no unit handover, so its
context is what costs. All other rules hold; these add to them.

- **One issue per thread.** Never brief several issues ("issues 1120-1123") in one thread.
- **A fresh thread per milestone.** After each merged milestone, write *Handover* in the ledger, commit, push,
  ask for a new thread that runs `/speckit-autopilot resume`, and stop.
- **Hand over at 150k** of context: do the same at once, whatever the step (the
  `context.md` cap, applied to the orchestrator).
- **Wait with `hold.sh`** ([waiting.md](waiting.md)), in the thread and in its subagents.
- **Pass `model`** on every `general-purpose` fallback ([../tasks/dispatch.md](../tasks/dispatch.md)).
- **Follow-up questions** for a finished thread go to the project chat: a reply after an hour idle
  re-writes the whole cache.
