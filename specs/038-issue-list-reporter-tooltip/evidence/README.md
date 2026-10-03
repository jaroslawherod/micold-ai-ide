# Feature 038, milestone M1: recorded visual pass (quickstart B1, B2)

Ran on Linux under Xvfb + lavapipe (not a real display), 2026-10-02, commit 86ea073c. Client against `cli/cli` (1,038 open issues, 1,000 loaded), light and dark theme, window 1600 wide and the 640 minimum; showcase `Typeahead` at 1600 and 420 wide. Crops, not full frames.

| Step | Result | Screenshot | Notes |
|---|---|---|---|
| B1 light | ok | b1-light.png | `#number title` first line; login, ` · ` and labels below in smaller, dimmer text |
| B1 dark | ok | b1-dark.png | same |
| B1 row without labels | ok | b1-nolabel-dark.png | `#8586 X Privacy Policy`: login `Authentiksolid` alone, no separator (light theme seen live, not saved) |
| B1 searched row | ok | b1-searched-dark.png, b1-searched-light.png | `#288 Add support for .netrc` (among the 38 least recently updated, beyond the 1,000 loaded) has the same two lines; the "Showing the 1,000 most recently updated of 1,038" hint is shown |
| B2 narrow (640) | ok | b2-narrow-dark.png, b2-narrow-light.png | long titles and a 4-label row (`#14545`) wrap inside the row; nothing cut or outside the list |
| B2 wide (1600) | ok | b2-wide-dark.png | the dialog keeps its width, so wrapping is the same as at 640 |
| B2 pick + marker | ok | b2-picked-light.png | pick `#14537` (two-line title): ticket and name filled; reopened list shows the picked marker beside its first line. Type stayed empty because its only label (`needs-triage`) maps to no type; picking `#14216` (label `bug`) filled Type `fix`, ticket and name (seen live) |
| B2 showcase light | ok | b2-showcase-light.png | first row picked, second highlighted, dimmed `fix/logout-redirect`, `main`; at 1600 wide nothing wraps (the long title and the label list fit on one line each) |
| B2 showcase dark | ok | b2-showcase-dark.png, b2-showcase-dark-420.png | same pose; at 420 wide the long title and the many-label row wrap, rows differ in height, nothing cut or overlapping (list opened upward) |

# Feature 038, milestone M2: recorded visual pass (quickstart B3)

Ran on Linux under Xvfb + lavapipe (not a real display), 2026-10-03, commit 93db16de. Client against `cli/cli` (1,038 open issues, 1,000 loaded), dark then light theme, window 1600x1400. Every press was captured (30 crops per theme) and read; the files below are a selection. Crops, not full frames.

| Step | Result | Screenshot | Notes |
|---|---|---|---|
| Down x15 | ok | b3-{dark,light}-down-03.png, -down-08-scrolled.png, -down-15-bottom.png | each press moved the highlight to the next issue (#14563 ... #14550, 15 rows), none skipped or repeated; the highlighted row was wholly visible after every press, including the scrolled frames and the tall (two-line title) `#14550` at the bottom |
| Up x15 | ok | b3-{dark,light}-up-03.png, -up-12.png, -up-15-top.png | back through the same rows; the highlight passes the picked row `#14526` (marker kept); the first row is reached on press 14 and press 15 stays on it (no wrap), wholly visible |
| Enter on a tall row | ok | b3-{dark,light}-pre-pick.png, -picked.png | highlight on `#14467` (two-line title); Enter fills Type `feat`, Ticket `14467`, Name |
| Reopen | ok | b3-{dark,light}-reopened.png | `#14467` carries the picked marker and is highlighted |

Not covered: scroll smoothness. The list was already open at the start of each run; the dark run began with `#14526` picked from an earlier click.

# Feature 038, milestone M3: recorded visual pass (quickstart B4)

Ran on Linux under Xvfb + lavapipe (not a real display), 2026-10-03, commit 615b5092 (client and daemon built from it and pinned in `~/vp/bin038m3`; pair connected). Client against `cli/cli` (1,038 open issues, 1,000 loaded), light then dark theme, window 1600x1400. Crops, not full frames.

| Step | Result | Screenshot | Notes |
|---|---|---|---|
| B4 empty hint | ok | b4-hint-light.png, b4-hint-dark.png | the empty field reads "Search by number, title, label or reporter" |
| B4 login, lower case | ok | b4-login-lower-light.png, b4-login-lower-dark.png | `bagtoad` narrows the list to BagToad's issues (`#14529`, `#9724`, `#14563` ...); the login is bold and tinted in each row's second line, labels unchanged |
| B4 login, capitals | ok | b4-login-caps-light.png, b4-login-caps-dark.png | `BAGTOAD` gives the same list and the same emphasis |
| B4 searched issue | ok | b4-searched-login-light.png, b4-searched-login-dark.png | `JomeFavourite` lists `#6413 Doc Sidebar Navigation Improvement`, one of the 38 least recently updated (beyond the 1,000 loaded; not in the 1,000 most recent): same two lines, `jomefavourite` emphasised |

Observations, not failures: a searched row emphasises the login only when the typed text matches it, as FR-012/013 say. Typing `billygriffin` did not list his beyond-cap `#3065` because the existing server search for that text does not return it (it did when typed as the number `3065`, with two lines and the login unemphasised); `#6413` for `jomefavourite` was returned. Matching is fuzzy, so a short login such as `travi` also picks up unrelated rows whose titles contain its letters.

Not covered: the 150 ms look of the list narrowing.
