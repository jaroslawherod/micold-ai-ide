# Asking the human

The user does not watch the autopilot. Write each question so they can answer it **cold**: from a
phone notification, hours later, with no scrollback.

## When to ask

### Ask only in these six cases

| # | Category | Examples |
|---|---|---|
| 1 | **Product or scope decision the repo does not settle** | Duplicate-name collision behaviour. Whether US3 is in scope. Which of two UX flows ships. |
| 2 | **Constitution conflict** | The only workable design breaks a principle, so an amendment would be needed. |
| 3 | **Irreversible or outward action** beyond merging this flow's own PRs | Cutting a release, editing a ruleset or repo settings, adding secrets or tokens, deleting user data, force-pushing `main`, publishing anywhere. |
| 4 | **Missing access** | A token scope, an external account, hardware or an OS that cannot be emulated or cross-checked. |
| 5 | **Non-convergence** | An artifact review failing after 3 rounds. CI red after 3 fix attempts. Clarify still raising questions after 4 rounds. A bug that will not reproduce. |
| 6 | **The plan proved false**, and the fix changes requirements | A dependency cannot do what plan.md assumed. The spec's success criterion is unmeasurable. |

A problem from work **outside this flow** (red `main`, another feature's failing test) goes under
category 5 or 6, labelled `blocked by work outside my flow`. Do not fix it.

### Never ask about these

- fmt, clippy or test failures in this flow's code
- review findings, including declining a wrong one (record the reason in the ledger)
- merge conflicts with `main`, flaky reruns
- checkless PRs (see `pr-and-merge.md`)
- visual verification (use the `visual-pass` skill)
- equivalent implementations, naming, file layout, test count
- a clarify question the repo answers

## How to ask

1. **Batch.** Finish the current round, checklist or review first. Send what is left in one
   `AskUserQuestion` call with up to 4 questions. A single question goes out alone, at once. Never
   hold questions for a later round.
2. **Recommend.** Put the recommended option first, labelled `(Recommended)`. Its description says
   why and cites a path, a measurement or a principle.
3. **Notify.** Load `PushNotification` with `ToolSearch("select:PushNotification")`. Send
   `🛑 <NNN> needs a decision: <topic>`.
4. **Record.** Write the escalation into the ledger's *Open escalation* section before asking. When
   the answer arrives, move it to *Decisions* as `decided by user`.

The message text right before the `AskUserQuestion` call is exactly this banner:

```
🛑 ACTION REQUIRED FROM YOU — <phase> · <NNN-feature> · <milestone or "-">
Decision needed: <one sentence, a question>
Why I can't decide this: <category number and name from the table above>
What I checked: <evidence — paths, commands, results>
Recommended: <option> — because <reason>
While waiting: <what is paused>. Nothing past this point will be merged.
```

For categories 3 and 4, where the user must *do* something, add one line:

```
What you need to do: <exact command or click path>, then reply "done".
```

## After the answer

- Apply the answer to the source artifact (spec.md, plan.md or tasks.md), not only the ledger.
- Continue at once. Do not ask "shall I continue?".

## When the user asks for a category 3 action themselves

A direct instruction such as "also cut a release when you're done" is consent for that one action.
Do not ask again.
1. Acknowledge it in one line.
2. Record it in *Decisions* as `decided by user`, quoting their words.
3. Carry on with the flow.
4. Do it only after the handoff checks pass.
5. Report the result in the WORK COMPLETE message.

If the repo does not say how (for example, which release mechanism), escalate that as a question.
