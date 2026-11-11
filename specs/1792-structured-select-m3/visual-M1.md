# Visual pass M1 (feature 1792): MenuSelect filled trigger

Date 2026-11-11. Showcase (`micold-showcase`, debug, pinned copy) on headless sway + pixman + lavapipe (not a real display), light scheme only.

Checked: MenuSelect "selector, closed / disabled / open with a filter" against the controls-section `Select`.
Images: `visual-M1-compare.png` (top to bottom: Select closed, MenuSelect closed, disabled, open; crops 2x), `visual-M1-menuselect.png` (the three poses in place).

- Container: same filled tone, rounded top corners, as Select. PASS
- Indicator along the bottom edge: 1 px neutral closed, accent colour when open. PASS
- Text `Model: claude-opus-4 · Default` fully visible in the 280 px trigger, not clipped. PASS
- Trailing chevron same glyph/size/position as Select; it points down in the open pose (no flip). PASS
- Disabled: container, text and chevron dimmed. PASS

Not covered: dark scheme, narrow window, Select open list (the click did not visibly open it in this capture), animation, the dense "closed" poses (not filled by design).
Verdict: PASS.
