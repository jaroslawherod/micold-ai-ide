# When to ask the human

The user does not watch the autopilot. How to ask: [../tasks/ask.md](../tasks/ask.md).

## Ask only in these six cases

| # | Category | Examples |
|---|---|---|
| 1 | **Product or scope decision the repo does not settle** | Duplicate-name collision behaviour. Whether US3 is in scope. Which of two UX flows ships. Which flow fits, when it is not plain. |
| 2 | **Constitution conflict** | The only workable design breaks a principle, so an amendment would be needed. |
| 3 | **Irreversible or outward action** beyond merging this flow's own PRs and keeping its own issue | Cutting a release, editing a ruleset or repo settings, adding secrets or tokens, deleting user data, force-pushing `main`, publishing anywhere. |
| 4 | **Missing access** | A token scope, an external account, hardware or an OS that cannot be emulated or cross-checked. |
| 5 | **Non-convergence** | An artifact review failing after 3 rounds. CI red after 3 fix attempts. Clarify still raising questions after 4 rounds. A bug that will not reproduce. |
| 6 | **The plan proved false**, and the fix changes requirements | A dependency cannot do what plan.md assumed. The spec's success criterion is unmeasurable. |

A problem from work **outside this flow** (red `main`, another feature's failing test) goes under
category 5 or 6, labelled `blocked by work outside my flow`. Do not fix it.

## Never ask about these

- fmt, clippy or test failures in this flow's code
- review findings, including declining a wrong one (record the reason in the ledger)
- merge conflicts with `main`, flaky reruns
- checkless PRs ([../tasks/ci.md](../tasks/ci.md))
- visual verification (use the `visual-pass` skill)
- equivalent implementations, naming, file layout, test count
- a clarify question the repo answers
