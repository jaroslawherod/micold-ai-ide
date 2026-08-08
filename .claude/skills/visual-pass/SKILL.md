---
name: visual-pass
description: Run a spec's manual visual pass (quickstart Part B) against the real GUI without a human — launch the component showcase or the client on a private Xvfb display, drive it with xdotool, screenshot with import, and look at the result. Use whenever a task says "needs eyes at a display", "record the pass", "run quickstart §B", or when a change alters how something *looks* and the geometry gates cannot see it (colour, weight, elevation, glyph collisions, state layers, floated labels, active indicators).
---

# Running the manual visual pass without a human

Spec-kit features here end with tasks nobody can automate: *"§B1 and §B2 are this feature's two
headline claims and neither can be automated; a green suite is not this feature working."* Those
tasks sat unrun for four features because the only recorded route was a person at a display.

There is another route. This skill is the verified one.

## What this catches that the test suite cannot

The gates in `tests/` resolve **layout**. They compare rectangles, and they are exact about
positions. They are structurally blind to:

- colour, tone, and elevation (`style_snapshot` records values, not what they look like composited)
- type weight — two labels at the same box with different weights pass every geometry check
- **glyphs drawn on top of each other** — each node is where its own layout says it is
- anything in `draw` only: `scale` and `fade` transform drawing, so an animating widget occupies
  exactly the boxes it occupies at rest

The first real finding from this skill was a leading search icon drawn on top of the first letter of
its own field's label. Every gate was green.

## The recipe

### 1. Never use `mise run screenshot` for this

`scripts/screenshot-session.py` captures **the logged-in desktop** — the user's browser, their
messages, whatever is on screen — and those pixels land in your context. It exists for a person
photographing their own session. It is the wrong tool here.

It also does not work for driving: XWayland windows cannot raise themselves above native Wayland
ones, so `xdotool windowactivate` silently leaves your app behind the user's browser and you
screenshot the browser.

### 2. Build the binary first, out of band

```bash
./scripts/build-lock.sh --no-lock cargo build -p micold-client --bin micold-showcase
```

Detach it (`setsid nohup … &`) and poll the log — it queues behind any other worktree's build.

### 3. A private X server

```bash
Xvfb :77 -screen 0 1600x1400x24 -nolisten tcp &
```

1600×1400 is deliberate: tall enough that a section and the list it floats fit in one frame, so a
comparison is one screenshot rather than two you have to hold in your head.

### 4. Launch — **with lavapipe, or it will not start**

```bash
env -u WAYLAND_DISPLAY DISPLAY=:77 \
    WGPU_BACKEND=vulkan \
    VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
    setsid nohup <binary> > run.log 2>&1 &
```

`env -u WAYLAND_DISPLAY` is required — winit prefers Wayland and will ignore `DISPLAY` entirely.

**`WGPU_BACKEND=gl` fails**, with `wgpu error: Validation Error / In Surface::configure / Invalid
surface`. Xvfb has no usable GLX. `lvp_icd.json` is Mesa's lavapipe, a software Vulkan rasteriser,
and it renders this application correctly.

Give it ~12s and confirm a window exists before screenshotting:

```bash
DISPLAY=:77 xdotool search --name "." | while read w; do
  echo "$w: $(DISPLAY=:77 xdotool getwindowname $w)"; done
```

### 5. Size it, then capture

```bash
DISPLAY=:77 xdotool windowsize $W 1600 1400
DISPLAY=:77 xdotool windowmove $W 0 0
DISPLAY=:77 import -window root shot.png
```

Then **Read the PNG**. A blank or black frame is a launch failure, not a result.

### 6. Driving it

There is **no window manager on `:77`**, so nothing sets input focus and *keyboard events go
nowhere*. Mouse works immediately; keys do not. Before any `xdotool key`:

```bash
DISPLAY=:77 xdotool windowfocus $W
```

If a key appears to do nothing, this is why — not the application.

| Action | Command |
|---|---|
| Scroll down / up | `xdotool click 5` / `click 4`, one notch each, after `mousemove` over the page |
| Click | `xdotool mousemove X Y; xdotool click 1` |
| Key | `xdotool windowfocus $W` first, then `xdotool key Escape` |

**Scrolling is trial and error** and costs turns. Locate cheaply: capture, then
`convert shot.png -resize 45% small.png` and read the small one. Full resolution is only needed once
you are on the thing you came to look at.

**An open overlay changes what a coordinate means.** A click at the field's own y-position lands on
whichever list row is now covering it. Close the list first, or aim deliberately — an accidental row
press once silently registered a pick and the next screenshot showed a different state than expected.

### 7. Comparing two things

Crop both at **identical geometry** and stack them, so the comparison is one image rather than two
in memory:

```bash
convert shot-a.png -crop 620x165+24+528 +repage a.png
convert shot-b.png -crop 620x165+24+703 +repage b.png
convert a.png -bordercolor '#cc0000' -border 1 \
        b.png -bordercolor '#0066cc' -border 1 -append -scale 200% compare.png
```

Same width, same x, same height — only y differs. Anything that differs is then a real difference,
not a framing artefact. Magnify suspicious details hard (`-scale 700%`); the icon-over-label
collision was invisible at 1× and unmistakable at 7×.

### 8. Both schemes

The showcase's scheme toggle is a button near the top. Scroll to top, click it, scroll back. Scroll
position is **not** preserved across the toggle, so re-locate.

### 9. Clean up

Kill **only what you started**, by PID. Never `pkill -f` — the user's own app and daemon may be
running, and stopping those is not yours to do.

## What this still cannot answer

Be honest about this in the report, and do not mark such a task passed:

- **Mid-flight animation.** A screenshot pipeline cannot reliably catch a chosen frame of a 150 ms
  transition, so "does a reversal resume from where it is, or snap?" stays unanswered.
- **Perceived smoothness.** lavapipe is a software rasteriser; frame pacing here says nothing about
  frame pacing on the user's GPU.

Static appearance, state changes, placement, colour, weight and glyph collisions are all in reach.
The transition's *look* is not. Report which half you covered.

## Recording the result

The pass is evidence, so write down what a reader would need to disbelieve it: the date, that it ran
on Xvfb + lavapipe rather than on a real display, which checks were exercised, and which were left
unrun and why. Attach or reference the comparison images.
