# Visual pass — 613 notification kinds (M3)

2026-10-06, Xvfb 1600×1400 + lavapipe (software Vulkan), not a real display. Client and daemon
pinned from one build of this branch; isolated `XDG_DATA_HOME`/`XDG_RUNTIME_DIR`. Notification
delivery itself (§B1–B4, B7) needs a notification service and a live CLI: not run here, covered by
the daemon tests.

| Item | Verdict | Evidence | Note |
|---|---|---|---|
| §B5 rows at defaults | PASS | visual-pass/b5-default.png | Four rows in order Needs permission, Session error, Long task finished, Turn finished; icon and note each; first three checked; threshold 60 under Long task finished |
| §B5 master off | PASS after fix | visual-pass/b5-master-off.png | First run: rows greyed but **lost their marks** (`style::checkbox` drew a disabled checked box as surface with an on-primary mark, i.e. invisible). Fixed: disabled checked fills on-surface at 38 % with a surface mark; test `a_disabled_checked_checkbox_keeps_a_visible_mark`. Re-shot: rows and field grey out and keep marks and value |
| §B6 marks as set, persist | PASS | visual-pass/b6-restart.png | Turn finished on, Long task finished off, threshold 20, Save; settings.json holds them; after a client restart the rows show as left |
| §B8 showcase | PASS | visual-pass/b8-light.png, visual-pass/b8-dark.png | Kind rows checked/unchecked, enabled/disabled, both schemes; threshold field valid, refused (range message), disabled. Crops taken before the disabled-checked fill fix |
| §B9 threshold refusal | PASS | visual-pass/b9-refused.png | 5 + Save: refused, "Enter a threshold between 10 and 3600 seconds.", field marked, form stays open; master off greys the field keeping 20 (b5-master-off.png); 20 persists across restart |

No glyph/label collision or clipping seen at 3× in the kind rows.
