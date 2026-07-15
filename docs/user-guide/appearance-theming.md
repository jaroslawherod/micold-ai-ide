# Appearance & Theming

Micold AI IDE uses a single Material Design layout and a shared design system, so every
screen — the top app bar, the project view, the known-projects list, and every dialog —
shares one consistent look. The application ships light and dark themes, and by default
follows your operating system's preference.

## Appearance & layout

The window is organised as a Material Design layout:

- A **top app bar** across the top holds the application title and its primary actions
  (including the Help menu and the theme selector).
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

Open the **theme selector** in the top app bar and pick one of:

- **Follow system** — track the OS preference (the default).
- **Light** — always use the light theme, regardless of the OS.
- **Dark** — always use the dark theme, regardless of the OS.

Your choice takes effect immediately and is remembered across restarts. Selecting **Follow
system** again resumes tracking your OS preference and switching live when it changes.
