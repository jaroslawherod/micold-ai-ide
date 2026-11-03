# Reading a structured session's transcript

A structured session shows the conversation: your messages, Claude's answers, its thinking and its
tool calls. Entries that are not part of the conversation proper are folded into one summary line
each, and by default they are hidden altogether (see *Showing raw events*).

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

## Showing raw events

By default the transcript leaves out the session start, the unrecognised or unreadable entries, a
tool result without its call, and the footer of a turn that succeeded. Turn **Show raw events** on
in the bar above the transcript to see them as the summary lines described above.

- The switch is off by default and applies to every structured session in the window at once.
- Errors stay visible with the switch off: a failed turn and an error result are always drawn.
- Your choice is remembered per user. It is saved as you press the switch and restored the next
  time the app starts; if the settings file cannot be written, the app says so and the choice
  still applies until you quit.
- Changing the switch clears any text you had selected in a transcript, because the rows changed.

## The hidden-events notice

When the switch is off and some entries are hidden, a line at the start of the transcript says how
many ("3 events hidden"), with a **Show** button that turns the switch on. A transcript that holds
only hidden entries shows this notice instead of an empty-session message.

A resumed session is read the same way as a live one: the same entries are hidden and the same count
is shown, whether you watched them arrive or opened the session later.
