# Reading a structured session's transcript

A structured session shows the conversation: your messages, Claude's answers, its thinking and its
tool calls. Entries that are not part of the conversation proper are folded into one summary line
each, so they stay out of the way.

## Summary lines

Four kinds of entry are shown as a single folded line:

| Line | What it is | Facts on the line |
|---|---|---|
| Session started | The session began. | The model. |
| Turn done / Turn failed | A turn ended. | How long it took and how many turns it ran. |
| Result without a call | A tool result whose call is not in the transcript. | "error" in the title when the result is an error. |
| Unrecognised event, Unreadable line, Unknown content block, Unknown stream event | Something the app could not read as part of the conversation. | The event's type, when it names one. |

A failed turn or an error result is drawn with the error outline, and its title says so in words.

## Expanding a line

Select a summary line to expand it, and select it again to fold it. The detail depends on the
entry:

- a session start shows its working directory;
- a turn shows its figures, such as the session's running cost, and, for a failed turn, the error
  text;
- a result without a call shows its output;
- an unreadable or unrecognised entry shows the original text, unchanged. A very long text scrolls
  inside a box of fixed height.

Copying a selection copies what is drawn: a folded line copies its summary, an expanded one also
its detail.
