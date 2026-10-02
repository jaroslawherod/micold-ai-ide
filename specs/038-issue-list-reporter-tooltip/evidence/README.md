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
