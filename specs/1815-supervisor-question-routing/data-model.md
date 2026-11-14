# Data model: supervisor question routing (#1815)

## Open escalation (input, from #1812)
`run-status.sh --json` `.escalation`: `category` (string beginning with the number), `banner`,
`questions[]` of `{number, question, options[{label,description}], recommendation, evidence}`.
Read-only here.

## Routing row (new, `## Routing` in `.autopilot/supervisor.md`)
One row per question. Key `R<issue>.Q<n>` (free-text escalation: `Q1`).

| Cell | Meaning |
|---|---|
| Key | `R<issue>.Q<n>` |
| State | `pending` (collected, banner not sent) · `asked` (banner and notification sent; answer awaited; the over-cap path stops here, and a restart re-asks these rows with `route next --reask`, sending no banner or notification again) · `answered` (user or parent answered; not yet sent, or sent and unconfirmed) · `resolved` (supervisor answered from repo evidence) |
| Session | the asking session id (the only send target, SC-004) |
| Via | `user` · `supervisor` · `parent` |
| Question, Options, Rec, Evidence | single-line, pipe-free copies (`|` stored as `/`, newlines to spaces; options joined by ` // `) |
| Answer | the answer text, evidence cited when `Via` is `supervisor` |
| Sent | epoch seconds of the `send_session_input`, empty until sent |

Transitions: `pending -> asked -> answered`; `pending -> resolved`; any state is removed by
`route drop <issue>` (session left `asking`, or ended `lost`). `answered` and `resolved` rows of one
issue are held until all its questions are one of the two, then sent together (`route compose`).
After the send and a `CHANGED` wake showing the run left `asking`, the issue's rows are removed
(the worker's *Decisions* holds the record); still `asking` after the Watcher ceiling: `route
overdue` lists it and the skill reports it, never re-sends.

`asked` is set when the banners and the notification go out, before the context check.

## Routing batch (derived)
`route next` output: up to 4 `pending` rows, ascending issue then question number. Not stored.

## Worker Decisions row (existing, #1812)
Gains By `supervisor` when `answer.sh --by supervisor` is used; default `user`.
