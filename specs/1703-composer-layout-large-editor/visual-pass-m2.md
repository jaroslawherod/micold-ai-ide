# Visual pass M2 (2026-11-05)

Headless sway (pixman, lavapipe); binaries built from this worktree. Quickstart section B.

| Step | Result | Evidence |
|---|---|---|
| 1 | Pass: one control row, collapsed settings icon, no options row | evidence/m2-c2.png |
| 2 | Partial: icon opens Model/Effort/Permissions row, icon turns tonal, settings.json has composer_options_open true. Toggle-off and restart not tested | evidence/m2-c2.png |
| 3 | Pass (Ready only): icon-only indicator, tooltip "Ready", no status word. Busy/Waiting not reached | evidence/m2-tm.png |
| 5 | Not confirmed: Bypass dialog shown, but session stayed at Default (CLI reported unset mode); marker never visible | evidence/m2-pms.png |

Other findings (outside the M2 steps): selectors look inconsistent when closed (step 4 fail); Permissions clipped at 520 px (step 7 fail, evidence/m2-ns.png).
