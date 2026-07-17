# Appearance & Theming

Micold AI IDE uses a single Material Design layout and a shared design system, so every
screen — the top app bar, the project view, the known-projects list, and every dialog —
shares one consistent look. The application ships light and dark themes, and by default
follows your operating system's preference.

## Appearance & layout

The window is organised as a Material Design layout:

- A **top app bar** across the top holds the application title and its primary actions
  (including the overflow menu).
- The **main area** shows either the active project or, when nothing is open, a welcoming
  empty state inviting you to open a folder.
- Content sits on distinct **surfaces** (cards) with consistent spacing and rounded corners.
- Buttons follow Material emphasis levels: a **filled** button for the single primary
  action, **outlined** and **text** buttons for secondary and low-emphasis actions. Buttons
  visibly respond to hover, focus, and press, and appear dimmed when disabled.

The known-projects list keeps everything it had before — the active-project marker, the
"git" badge on repositories, and the "unavailable" state for folders that have moved or
been deleted — now presented as Material list items.

## Automatic light/dark

By default the application follows your operating system's light or dark setting:

- On launch it matches your current OS theme.
- If you change your OS between light and dark while the app is running, it switches to
  match within about a second — no restart needed.

Both themes are first-class: the dark theme is fully designed for legibility, not a dimmed
version of the light one.

> **Linux note:** OS theme detection uses the XDG desktop portal (with GTK/KDE settings as a
> fallback). On a session without any of these available, the app cannot read a preference
> and shows the light theme. Choosing a theme explicitly (below) always works.

## Choosing your theme

Open the **overflow menu** (the three-dots icon on the right of the toolbar). Its first item
is a **theme-mode toggle** showing the current mode with an icon; **each click cycles to the
next mode**:

- **Auto** (brightness-auto icon) — follow the OS preference (the default).
- **Light** (sun icon) — always use the light theme, regardless of the OS.
- **Dark** (moon icon) — always use the dark theme, regardless of the OS.

The cycle order is Auto → Light → Dark → Auto. The menu stays open while you cycle so you can
click again. Your choice takes effect immediately and is remembered across restarts. Cycling
back to **Auto** resumes tracking your OS preference and switching live when it changes.

## Motion & animations

The interface uses brief, consistent motion so changes feel considered rather than abrupt:

- **Dialogs fade in and out.** Every dialog — About, the project browser, rename project, add
  worktree, and Settings — fades and gently lifts into view when it opens (about 0.25 s), and
  fades back out when you close it (about 0.2 s), whether you close it with **Cancel**, the
  **Esc** key, or a successful **Save/Create**. As a dialog leaves, the app behind it becomes
  visible again through the fade, rather than the dialog blinking away in a single frame. (A
  dialog that reports an error on submit — for example an invalid Settings value — stays open
  instead, so nothing animates away.)
- **The rest of the interface keeps its familiar motion.** The overflow menu still fades, the
  worktree sidebar still slides as it collapses and expands, the main area cross-fades when its
  content changes, and the sidebar's resize handle highlights on hover — all unchanged.

Motion only runs while something is actually moving, so it never adds ongoing background work
when the interface is at rest. There is currently no reduced-motion setting.
