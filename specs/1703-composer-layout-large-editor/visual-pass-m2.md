# Visual pass M2 (2026-11-05)

Headless sway (pixman, lavapipe); binaries built from this worktree. Quickstart section B.

| Step | Result | Evidence |
|---|---|---|
| 1 | Pass: one control row, collapsed settings icon, no options row | evidence/m2-c2.png |
| 2 | Partial: icon opens Model/Effort/Permissions row, icon turns tonal, settings.json has composer_options_open true. Toggle-off and restart not tested | evidence/m2-c2.png |
| 3 | Pass (Ready only): icon-only indicator, tooltip "Ready", no status word. Busy/Waiting not reached | evidence/m2-tm.png |
| 5 | Not confirmed: Bypass dialog shown, but session stayed at Default (CLI reported unset mode); marker never visible | evidence/m2-pms.png |

Other findings (outside the M2 steps): selectors look inconsistent when closed (step 4 fail); Permissions clipped at 520 px (step 7 fail, evidence/m2-ns.png).

## M3 #1764 row order (2026-11-07)

Headless sway (pixman, lavapipe); client and daemon built from this worktree, structured Claude session in a scratch project. Order seen: activity circle, attach clip, text area (expand icon inside), settings gear, Send.

| Capture | Result | Evidence |
|---|---|---|
| Default window, collapsed row | Pass: order as stated, no overlap, text area fills the gap. Note: the Ready circle sits about 16 px lower than the clip and gear (bottom-aligned), not clipped | evidence/m3-1764-default.png |
| Options row expanded (gear click) | Pass: gear toggles Model/Effort/Permissions row above, gear turns tonal; a second click collapses it | evidence/m3-1764-expanded.png |
| Narrow window (about 620 px surface) | Pass: text area shrinks, clip, gear and Send stay apart and unclipped. The app surface is cut off at the left by the window (sidebar), not by the row | evidence/m3-1764-narrow.png |
| Busy/Waiting activity icon | Not reached (needs a live model turn); Stop button not seen | none |
