# 035 M4 visual pass (quickstart Part B, B1-B11 except B12), both themes

Date 2026-09-30. Xvfb :91 (1600x1400) + lavapipe, not a real display. Client and daemon copied from
the shared target dir (built by the gate at HEAD 1ed44b44) to a private pin dir `~/vp/bin-035-m4`;
daemon attachment confirmed in its log. Private XDG data and runtime dirs (`~/.cache/vp91/data`,
`/tmp/vp91`), theme seeded as `dark` or `light` in settings.json. `/tmp/does-not-exist.sh` absent at
the start of each step except B6 (touched), and absent at the end. Crops are of the Environment page
below the Timeout field; `-notice-` crops are the bottom-centre notification area just after Save.
Mid-flight animation and real-GPU frame pacing not exercised.

| Step | Theme | Screenshot | Verdict |
|---|---|---|---|
| B1 | dark | M4-B1-dark.png | PASS: red caution "Script not found: /tmp/does-not-exist.sh", then OFF note |
| B1 | light | M4-B1-light.png | PASS: same |
| B2 | dark | M4-B2-dark.png | PASS: nothing below Timeout |
| B2 | light | M4-B2-light.png | PASS: same |
| B3 | dark | M4-B3-dark.png | PASS: nothing below Timeout |
| B3 | light | M4-B3-light.png | PASS: same |
| B4 | dark | M4-B4-dark.png | PASS: one "Script not found: ..." caution, then ON note |
| B4 | light | M4-B4-light.png | PASS: same |
| B5 | dark | M4-B5-notice-dark.png, M4-B5-dark.png | PASS: notice "The environment-include script was not found: /tmp/does-not-exist.sh"; reopen shows B1's page |
| B5 | light | M4-B5-notice-light.png, M4-B5-light.png | PASS: same |
| B6 | dark | M4-B6-dark.png | PASS: "The last attempt could not find the script", then "/tmp/does-not-exist.sh exists now. Save Settings or restart a session to source it." |
| B6 | light | M4-B6-light.png | PASS: same |
| B7 | dark | M4-B7-dark.png | PASS: caution "Script not found: ~/env.sh", TILDE note, OFF note |
| B7 | light | M4-B7-light.png | PASS: same |
| B8 | dark | M4-B8-dark.png | PASS: only the REL note |
| B8 | light | M4-B8-light.png | PASS: same |
| B9 | dark | M4-B9-notice-dark.png, M4-B9-dark.png | PASS: typed an existing readable file, Save: no notification; reopen shows nothing below Timeout |
| B9 | light | M4-B9-notice-light.png, M4-B9-light.png | PASS: same |
| B10 | dark | M4-B10-notice-dark.png, M4-B10-dark.png | PASS: cleared path, Save: no notification; reopen shows nothing below Timeout |
| B10 | light | M4-B10-notice-light.png, M4-B10-light.png | PASS: same |
| B11 | dark | M4-B11-notice-dark.png, M4-B11-dark.png | PASS: Save unchanged gives the not-found notice; reopen shows B1's page |
| B11 | light | M4-B11-notice-light.png, M4-B11-light.png | PASS: same |

Style: cautions and notes use the page's shared caution/note styles in both themes; every line is
whole, left-aligned with the fields, with no overflow or clipping in the page column.
