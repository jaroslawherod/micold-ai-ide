# Asking the human

The user started the autopilot so they would not have to watch it. When a question reaches them it
has to be answerable **cold**: from a phone notification, hours later, with no scrollback.

## When to ask

### Ask only in these six cases

| # | Category | Examples |
|---|---|---|
| 1 | **Product or scope decision the repo does not settle** | Collision behaviour for duplicate names. Whether US3 is in scope for this feature. Which of two UX flows ships. |
| 2 | **Constitution conflict** | The only workable design breaks a principle, so an amendment would be needed. |
| 3 | **Irreversible or outward action** beyond merging this flow's own PRs | Cutting a release, editing a ruleset or repo settings, adding secrets or tokens, deleting user data, force-pushing `main`, publishing anywhere. |
| 4 | **Missing access** | A token scope, an external account, hardware or an OS that cannot be emulated or cross-checked. |
| 5 | **Non-convergence** | An artifact review still failing after 3 rounds. CI red after 3 fix attempts. Clarify still raising questions after 4 rounds. A bug that will not reproduce. |
| 6 | **The plan proved false**, and the fix changes requirements | A dependency cannot do what plan.md assumed. The spec's success criterion is unmeasurable. |

A problem caused by work **outside this flow** (red `main`, another feature's failing test) is
escalated under category 5 or 6 and labelled `blocked by work outside my flow`. You do not fix it.

### Handle these yourself, never ask

- fmt, clippy or test failures in this flow's code
- review findings, including declining a wrong one (record the reason in the ledger)
- merge conflicts with `main`
- flaky reruns
- checkless PRs (see `pr-and-merge.md`)
- visual verification (use the `visual-pass` skill)
- choosing between equivalent implementations
- a clarify question the repo answers
- naming
- file layout
- how many tests to write

## How to ask

1. **Batch.** Finish triaging the current round, or resolving the current checklist or review, first.
   Handle everything you can, then send what is left in one `AskUserQuestion` call with up to 4
   questions. A single remaining question goes out alone, straight away. Never hold questions over
   into a later round.
2. **Recommend.** Put the recommended option first and label it `(Recommended)`. Its description
   says why, and cites a path, a measurement or a principle.
3. **Notify.** Load the `PushNotification` tool with `ToolSearch("select:PushNotification")` and send
   a one-line summary: `🛑 <NNN> needs a decision: <topic>`.
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

For categories 3 and 4, where the user has to *do* something rather than choose, add one line:

```
What you need to do: <exact command or click path>, then reply "done".
```

## After the answer

- Apply the answer to the source artifact (spec.md, plan.md or tasks.md), not only to the ledger.
- Continue the flow straight away. Do not ask "shall I continue?".

## When the user asks for a category 3 action themselves

A direct instruction such as "also cut a release when you're done" is consent for that one action,
so do not ask again.
1. Acknowledge it in one line.
2. Record it in *Decisions* as `decided by user`, quoting their words.
3. Carry on with the flow.
4. Do it only after the handoff checks pass.
5. Report the result in the WORK COMPLETE message.

When how to do it is not clear from the repo, for example which release mechanism to use, that is a
question for the user: escalate it then.
