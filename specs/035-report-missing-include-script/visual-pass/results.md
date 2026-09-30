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

## M3: B4, B5, B6 (T027)

Date 2026-09-30. Xvfb :78 + lavapipe, dark scheme only, real client and daemon built from
36015322 (pinned pair, attachment confirmed), private data and runtime dirs.
`/tmp/does-not-exist.sh` absent at the start and again at the end. B6 ran before B5, from the B4
seed.

| Step | Screenshot | Verdict |
|---|---|---|
| B4 | B4-environment.png | PASS: one caution "Script not found: /tmp/does-not-exist.sh" (not two), then the ON note |
| B5 | B5-notification.png, B5-reopen.png | PASS: after unticking and Save, notice "The environment-include script was not found: /tmp/does-not-exist.sh"; reopen shows the same caution, then the OFF note |
| B6 | B6-exists-now.png | PASS: after `touch`, caution "The last attempt could not find the script", then "/tmp/does-not-exist.sh exists now. Save Settings or restart a session to source it." |

Style: cautions in the page's caution style, notes in the note style, in the page column, no
overflow. Light theme not exercised. The first Save click in B5 did not register under xdotool; a
second click after a mouse move did.
