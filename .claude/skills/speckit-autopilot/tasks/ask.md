# Task: ask the human (orchestrator)

When: a unit returned `ESCALATE`, or the flow is not plain ([../flows/choose.md](../flows/choose.md)).
The user does not watch the autopilot. Write each question so they can answer it **cold**: from a
phone notification, hours later, with no scrollback. What may be asked:
[../rules/escalation.md](../rules/escalation.md).

1. **Batch.** Finish the current round, checklist or review first. Send what is left in one
   `AskUserQuestion` call with up to 4 questions. A single question goes out alone, at once. Never
   hold questions for a later round.
2. **Recommend.** Put the recommended option first, labelled `(Recommended)`. Its description says
   why and cites a path, a measurement or a principle.
3. **Notify.** Load `PushNotification` with `ToolSearch("select:PushNotification")`. Send
   `🛑 <issue> needs a decision: <topic>`.
4. **Record.** The escalation is in the ledger's *Open escalation* before you ask (the unit wrote
   it; the flow question at entry has no ledger yet). When the answer arrives, the unit moves it to *Decisions* as `decided by user`.

The message text right before the `AskUserQuestion` call is exactly this banner:

```
🛑 ACTION REQUIRED FROM YOU — <unit> · <issue and feature> · <milestone or "-">
Decision needed: <one sentence, a question>
Why I can't decide this: <category number and name>
What I checked: <evidence — paths, commands, results>
Recommended: <option> — because <reason>
While waiting: <what is paused>. Nothing past this point will be merged.
```

For categories 3 and 4, where the user must *do* something, add one line:

```
What you need to do: <exact command or click path>, then reply "done".
```

## After the answer

- The unit applies the answer to the source artifact (spec.md, plan.md or tasks.md), not only the
  ledger.
- Continue at once. Do not ask "shall I continue?".

## When the user asks for a category 3 action themselves

A direct instruction such as "also cut a release when you're done" is consent for that one action.
Do not ask again. Acknowledge it in one line, have the unit record it in *Decisions* as `decided by
user` with their words, carry on, do it only after the handoff checks pass, and report the result
in the WORK COMPLETE message. If the repo does not say how (which release mechanism), escalate
that as a question.

Next: continue the unit that escalated, or the flow file after the flow question.
