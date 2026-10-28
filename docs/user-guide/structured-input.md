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
