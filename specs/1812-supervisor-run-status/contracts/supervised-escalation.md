# Contract: supervised escalation

## Marker
`scripts/autopilot/supervised.sh [<ledger>]`: exit 0 when `AUTOPILOT_SUPERVISED` is set, non-empty
and not `0`, or the ledger has `- **Supervised**: yes`; exit 1 otherwise. No output.

## Escalation format (ledger *Open escalation*)
```
- **Category**: 1 Product or scope decision
- **Question**: <one sentence>
- **Options**:
  1. <label> (Recommended) — <description>
  2. <label> — <description>
- **Recommended**: <label> — <reason>
- **Evidence**: <paths, commands, results>
- **Banner**: sent <date>
```
A batch: `### Q1`, `### Q2`... each holding Question, Options, Recommended, Evidence; Category and
Banner stay above them. Empty section is `None.`

## Supervised behaviour
1. A unit escalates: writes the section above, returns `STATUS: ESCALATE` as today.
2. The orchestrator (`tasks/ask.md`), when `supervised.sh` exits 0 and a ledger exists: prints the
   banner and the notification, does not call `AskUserQuestion`, ends the turn. With no ledger yet
   (the flow question at entry) it asks interactively as today (FR-011c).
3. Unsupervised: unchanged.

## Answer message
`resume <answer>`: the word `resume`, then the chosen option's label or free text. Batch: one line
per question, `<n>. <answer>` or `<n>: <answer>`. `resume` alone with a non-empty escalation:
banner again, end the turn (FR-011b), supervised only.

## `scripts/autopilot/answer.sh <ledger> <answer-text>`
| Exit | Meaning | Ledger |
|---|---|---|
| 0 | one Decisions row per question (By `user`), section reset to `None.`; prints the rows | changed |
| 2 | batch answer missing or repeating a question number; prints which | unchanged |
| 3 | no open escalation; prints `ignored: no open escalation` | unchanged |
Batch versus single is set by the number of `### Q<n>` subsections in the ledger. In a Decisions row `|` is written `\|` and newlines become spaces.
After exit 0 the orchestrator applies the answer to the source artifact (`tasks/ask.md`, *After the
answer*) and continues from *Next step*.
