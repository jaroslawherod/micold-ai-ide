# Reading a structured session's transcript

A structured session shows the conversation: your messages, Claude's answers, its thinking and its
tool calls. Entries that are not part of the conversation proper are folded into one summary line
each, and by default they are hidden altogether (see *Showing raw events*).

## Your messages

Each message you sent stands out from Claude's output: it sits on a tinted block with a bar in the
accent colour down its left edge, so you can find where each of your turns began while scrolling.
This holds for a message you typed, a queued message once it is delivered, and a slash command (its
card keeps folding and unfolding as before), in the main conversation and inside a sub-agent's
thread. The block and the bar follow the light or dark theme and your accent colour. Selecting and
copying your messages works as it does for the rest of the transcript.

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
with the **N hidden** chip at the end of the header row above the transcript, or in the worktree **⋮** menu in the top bar (see
*The worktree menu* below), to see them as the summary lines described above.

- The switch is off by default and applies to every structured session in the window at once.
- Errors stay visible with the switch off: a failed turn and an error result are always drawn.
- Your choice is remembered per user. It is saved as you press the switch and restored the next
  time the app starts; if the settings file cannot be written, the app says so and the choice
  still applies until you quit.
- Changing the switch clears any text you had selected in a transcript, because the rows changed.

## The worktree menu

The **⋮** button named *Worktree actions*, in the top bar beside the settings button, opens a menu
of actions for the selected worktree or session. The app menu (the other **⋮**) holds none of them.

- **Show raw events** is the switch described above. The row says **On** or **Off**, pressing it
  flips it, and the menu stays open so you see the new state.
- **Structured / Terminal** appears for a Claude Code session and shows which view the session is
  in. Press the other one to switch, as **Open in terminal** and **Open in structured view** do.
  Other CLIs have one view, so the control is not shown.
- Escape, or a press outside the menu, closes it; so does selecting another worktree or session.

The tab row has one **+**, named *Add session or terminal*, in place of the two add icons and the
**Open in terminal** button that used to sit there. It opens a menu with **Another session** and
**Terminal**; Escape or a press outside closes it. Switching a session to the terminal is **Terminal**
in the worktree menu above.

## Status, Changes and context in the top bar

With a worktree or session selected, the top bar shows, before the settings button and the app
menu and in this order:

1. **Context meter**: a small ring with the share of the context window used and the session's cost,
   for example `8% · $1.28`. Hover the ring for the tokens used out of the window, the cost and the
   number of turns and the number of running background tasks. The ring turns to the warning colour near compaction. A session with no usage
   yet shows an empty ring and no cost.
2. **Status**: `Working` while the session's process is running, `Idle` otherwise (starting,
   stopped, failed or exited).
3. **Changes**: opens the changes view of the shown worktree, as the Changes button did before; it
   reads `Changes · no changes`, or the number of files and lines changed.

None of these show on the Overview.

When the window is narrow the bar gives way in order: the ring's text hides first (the ring and the
tooltip, which still carries the cost, stay), then the **Changes** label hides and its icon stays.

## The header row

The session area has one 40 px row above the transcript. It holds the session tabs, which scroll
sideways when they do not fit, and at the end of the row the restart control (shown only for a stopped
session), the split buttons and the **N hidden** chip. The former title row, the `running` status text
and the status cluster are gone: status, the ring and **Changes** are in the top bar.

- A session tab shows an activity dot only while that session is working. A failed or exited session
  shows no dot.
- The **N hidden** chip (named "N hidden events, show") appears only while events are hidden; pressing
  it shows the raw events. With none hidden it is absent.
- The old session bar (meter, totals and the raw-events row) is removed. The list of background tasks
  and the mode label sit in a slim first row of the transcript, only when there is something to show.
- In a narrow window the tabs scroll sideways, and **+**, restart, the split buttons and the chip stay
  reachable.

## The hidden-events notice

When the switch is off and some entries are hidden, a line at the start of the transcript says how
many ("3 events hidden"), with a **Show** button that turns the switch on. A transcript that holds
only hidden entries shows this notice instead of an empty-session message.

A resumed session is read the same way as a live one: the same entries are hidden and the same count
is shown, whether you watched them arrive or opened the session later.
