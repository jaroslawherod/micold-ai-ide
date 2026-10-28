# Giving a structured session a file

Drag files from your file manager onto a structured session's view to hand them to the assistant.

## Dropping files

Drop one or more files anywhere on the session's view: over the transcript or over the composer. The
files go to that session's composer only. Nothing is typed into a terminal, even when a terminal
pane sits beside the session.

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
just before **Send** (hover it for its name). The system file picker opens and lets you choose
several files at once; the interface stays usable while it is open.

- **Confirm** the choice and the files are handled exactly as if you had dropped them, in the order
  chosen: pictures become chips, other files are inserted as quoted paths, and anything that
  cannot be attached is named in a notice.
- **Cancel** the picker and nothing changes and no notice appears.
- The button is greyed out while a picker is already open for that session, and for a session that
  is not accepting input. If the session stops accepting input while the picker is open, the files
  you chose are not attached and a notice says so.
- Each session has its own picker: what you choose goes to the session whose button you pressed.
