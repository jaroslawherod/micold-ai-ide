# Giving a structured session a file

Drag files from your file manager onto a structured session's view to hand them to the assistant.

## Dropping files

Drop one or more files anywhere on the session's view: over the transcript or over the composer. The
files go to that session's composer only. Nothing is typed into a terminal, even when a terminal
pane sits beside the session.

While you drag files over the window, each structured session that can take input is outlined and
tinted in the accent colour, also when the session fills the window on its own; the outline goes when you drag out again or drop. Some systems do not
report a drag until it ends: there is then no outline, and the drop works just the same.

- **Pictures** (PNG, JPEG, GIF, WebP) become attachment chips in the composer and are sent with your
  next message.
- **Other files and folders** are inserted into the draft as quoted paths at the caret, the same
  way dropping them on a terminal inserts them. No chip is added.
- **Several files at once** are all handled, in the order you dropped them.

A picture that cannot be attached (unreadable, too large, or beyond the per-message limit) is left
out, and a notice names the file and the reason.

## When a drop is refused

A session that is not accepting input, for example one that has ended or one that has not yet
reported that it is running, takes nothing from a drop.
The composer shows a notice such as `2 files not attached: the session is not accepting input`, and
the draft stays as it was.

A drop that carries no files, such as dragged text, changes nothing and shows no notice.

## The Add attachment button

If you would rather not drag, click the paperclip **Add attachment** button in the composer row,
just before **Send** (hover it for its name and shortcut). With the cursor in the message box you
can press **Ctrl+Shift+A** (**Cmd+Shift+A** on macOS) instead. The system file picker opens and lets you choose
several files at once; the interface stays usable while it is open.

- **Confirm** the choice and the files are handled exactly as if you had dropped them, in the order
  chosen: pictures become chips, other files are inserted as quoted paths, and anything that
  cannot be attached is named in a notice.
- **Cancel** the picker and nothing changes and no notice appears.
- The button is greyed out while a picker is already open for that session, and for a session that
  is not accepting input. If the session stops accepting input while the picker is open, the files
  you chose are not attached and a notice says so.
- Each session has its own picker: what you choose goes to the session whose button you pressed.

## The large editor

To write a long or multi-line message, open the large editor: with the cursor in the message box press
**Ctrl+Shift+M** (**Cmd+Shift+M** on macOS), or click the **Open large editor** button at the
top-right of the message box (it appears while the message has several lines or the box is focused
or hovered). A window opens over the app with a large text area holding your draft, the cursor at
its end.

- **Done**, or **Ctrl+Enter** (**Cmd+Enter** on macOS), closes it and puts the edited text in the message box.
- **Cancel**, or **Escape**, closes it and leaves the message box as it was.
- Only text is edited: attached pictures stay on the message box and are not shown in the editor.
- The text goes back to the session whose message box opened the editor, even if you have switched
  sessions or that session has ended in the meantime.

## The activity indicator

An icon at the left of the message box shows what the session is doing: a spinning icon while Claude Code
works, a different icon while it waits for you (a question or a permission request), and a resting
icon otherwise. Hover the icon to read the state ("Working", "Waiting for you", "Ready", "Not
running"); no status text is shown in the row. **Stop session** and any refusal note stay visible
in the row.

## Session options

The **Session options** button (a settings icon) right of the message box, before the attach button, **Stop** and **Send**, shows or hides
a row holding the session's **Model**, **Effort** and **Permission** selectors. The permission
selector is no longer in the message box row itself.

- The row is hidden by default. Whether it is shown is remembered per window and restored when the
  app restarts.
- When Claude Code refuses a model, effort or permission change, the row opens by itself so the
  refused selector is visible.
- While **Bypass permissions** is on, the settings icon carries a warning marker and its tooltip says so.
- The button is reachable with Tab; **Space** or **Enter** toggles the row.

### The selectors

**Model**, **Effort** and **Permission** look and behave the same: a 32 px high box with a thin
border, `Label: Value` inline and a chevron at the end, and the same hover, focus, pressed, open
and disabled looks. A disabled selector says why in its tooltip. Each opens a floating list; the
**Model** list starts with a **filter** field that has the keyboard as it opens. Type to narrow the
models (case does not matter); a typed name that is no listed model is offered as **Use <name>**.
The arrow keys move through the list, **Enter** takes the highlighted row and **Esc** closes it.

## Finding the shortcut

Type **/shortcuts** in the message box and send it to open the shortcut sheet, the same dialog the
Review tab's **Keyboard shortcuts** item opens. Its **Composer** group lists **Add attachment**
with its shortcut (**Ctrl+Shift+A**, **Cmd+Shift+A** on macOS) and **Open large editor** (**Ctrl+Shift+M**,
**Cmd+Shift+M** on macOS); when a Review tab is shown it also
lists the **Review** group. Nothing is sent to Claude Code, and the command is cleared from the
message box. Press **Escape** or **Close** to dismiss the sheet.
