# Visual pass M1 (feature 1792): quickstart Part B steps 1 and 4

Date 2026-11-11. Debug builds pinned from this worktree (`micold-showcase`, `micold-ai-ide`, `micold-daemon`), headless sway + pixman + lavapipe (not a real display). Showcase in the light scheme, client in its default dark scheme. Steps 2, 3, 5-7 (animation, reduce motion, narrow window, themes) are later milestones and were not run.

## Step 1: showcase, `Select` vs `MenuSelect::filled()`

Images: `visual-pass-M1-states.png` (rows: Select, MenuSelect for rest, hover, press; left 320 px, 1.5x), `visual-pass-M1-menuselect-poses.png` (MenuSelect closed, disabled, open with filter; 2x).

| State | Result | Note |
|---|---|---|
| Rest | PASS | Same filled tone (230,225,230), same height, rounded top corners, 1 px bottom indicator, same chevron glyph/size at the right edge. |
| Hover | PASS | Both go to (214,209,214), identical. |
| Pressed | NOT CHECKED | Both read (214,209,214), same as hover; the press layer was not distinguishable, so the press was probably not captured. |
| Disabled | PASS (MenuSelect only) | Container, text and chevron dimmed. `Select` has no disabled pose in the showcase, so no side-by-side. |
| Open | PARTIAL | MenuSelect open pose: accent bottom indicator, list below, no collisions. `Select` live open list never rendered in this capture (clicks only produced the hover tone, with and without a pointer move), so no side-by-side. |
| Focus | NOT CHECKED | Headless sway has no keyboard device (`wtype`/Tab did nothing). |

Observation, not a defect: the filled MenuSelect text sits in the lower half of the trigger (label-and-value layout), while the unset `Select` ("Theme") is vertically centred. Different content, same container.

## Step 4: client, structured session composer

Images: `visual-pass-M1-composer-closed.png` (options row), `visual-pass-M1-composer-open.png` (model, effort, permission lists open, top to bottom, 0.7x).

- Model, Effort, Permissions each show the Material 3 filled trigger (tonal container, "Label: value" text, trailing chevron, bottom indicator). PASS
- Open: accent bottom indicator, list above the trigger (composer is at the bottom), selected row marked, no clipping; Model list scrolls. PASS
- Model trigger text is ellipsised (`Model: Model not reported yet · De…`) right before the chevron: intended truncation, chevron not overlapped. PASS

Dark scheme only; the sandbox daemon had no model reported, hence the placeholder text.

Verdict: PASS for the checked states; focus, the pressed layer and the Select open list are unverified (tooling).
