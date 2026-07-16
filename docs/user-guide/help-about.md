# Help & About

This page covers the Micold AI IDE application window, its **Help** toolbar entry, and the
**About** dialog.

## The application window

When you launch Micold AI IDE, a single main window opens with a **toolbar across the top**.
On the right of the toolbar is an **overflow menu** — a three-dots (⋮) icon button.

## Opening About

1. In the toolbar, select the **overflow menu** (the three-dots icon on the right). An
   **About** item appears beneath it.
2. Select **About**. The About dialog opens as a modal overlay centered in the window.

While the dialog is open, the rest of the window (including the toolbar) is dimmed and does
not respond to input.

## What the About dialog shows

| Field | Meaning |
|-------|---------|
| **Micold AI IDE** | The application name. |
| **Version** | The version of the app you are running, taken from the build. |
| **License** | The open-source license the app is distributed under (Apache-2.0). |
| Description | A one-line summary of the application. |

The version always reflects the build you are running — it is read from the package
metadata, never typed in by hand. If a field's value is unavailable, the dialog shows
`unknown` in its place rather than a blank.

## Closing the dialog

Dismiss the About dialog in either of these ways:

- Click the **Close** button.
- Press the **Esc** key.

Either way, the dialog closes and you return to the main window exactly as it was before you
opened it. Pressing **Esc** when no dialog is open does nothing.

> Clicking the dimmed area outside the dialog does **not** close it — use Close or Esc.
