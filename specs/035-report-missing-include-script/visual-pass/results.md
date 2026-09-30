# 035 visual pass (quickstart Part B: B1, B2, B3, B7, B8, B12)

Date 2026-09-30. Xvfb :78 + lavapipe (not a real display), dark scheme only, freshly built
client from branch fix/github-issues, private XDG dirs. Crops are of Settings > Environment.
Mid-flight animation and light theme not exercised.

| Step | Screenshot | Verdict |
|---|---|---|
| B1 | B1.png | PASS: red caution "Script not found: /tmp/does-not-exist.sh", then muted OFF note |
| B2 | B2.png | PASS: nothing below Timeout |
| B3 | B3.png | PASS: nothing below Timeout |
| B7 | B7.png | PASS: caution "Script not found: ~/env.sh", "~ is not expanded. Use a full path.", OFF note |
| B8 | B8.png | PASS: only "Relative path: whether the script is found depends on each session's directory." |
| B12 | B12.png | PASS: exactly one caution "Script not found", no path (011 interim; a startup resolution attempt had already run) |
| B11 | B11-notice.png, B11-reopen.png | PASS: one Info-style notice "The environment-include script was not found: /tmp/does-not-exist.sh" after Save (Xvfb :79, rebuilt binaries, 2026-09-30), text whole, no truncation, gone after ~6s; reopen shows B1 page (red caution, then OFF note) |

Legibility: lines sit under the Timeout field in the page column, left-aligned with the fields, no overlap or overflow.
