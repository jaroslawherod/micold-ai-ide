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

### 2. Build the binary first, out of band — **then copy it somewhere only you write**

```bash
./scripts/build-lock.sh --no-lock cargo build -p micold-client --bin micold-showcase
```

Detach it (`setsid nohup … &`) and poll the log — it queues behind any other worktree's build.

**Never launch straight out of `target-shared/`.** Every checkout on this machine builds into that
one directory (CLAUDE.md), so `target-shared/debug/<bin>` is whatever branch built *last* — and the
worktrees that made you wait for the lock are exactly the ones that overwrite it the moment you stop
waiting. A pass that launches from there can screenshot another branch's code and report the wrong
result with a perfectly clear conscience. This has happened: a bar screenshot showed a control the
branch under test had deleted, while the source contained no reference to it and its gate passed.

```bash
scripts/build-lock.sh bash -c \
  'cargo build -p micold-client --bin micold-ai-ide -p micold-daemon --bin micold-daemon &&
   cp "$CARGO_TARGET_DIR/debug/micold-ai-ide" "$CARGO_TARGET_DIR/debug/micold-daemon" ~/vp/bin/'
```

**Name both bins.** `--bin` filters the whole invocation to the targets it names, so
`-p micold-client --bin micold-ai-ide -p micold-daemon` — what this recipe said until 2026-08-18 —
builds the client and **silently skips the daemon**, leaving whatever `target-shared` already held.
The `cp` then pins a matched-looking pair that is not one. It cost a pass two rounds: the daemon was
a version behind, and the log said so plainly once it was read (`client_version=6 … daemon_version=5`).

One invocation, both binaries, copy inside the lock — then run the copies. Three details:

- **The client and the daemon must come from the same build.** The client refuses a daemon whose
  protocol *schema hash* differs (`handshake::evaluate`), and the daemon logs that as `refusing
  client: contract or build mismatch` **while printing matching versions on both sides**
  (`client_version=5 … daemon_version=5`, same package version). The message reads like a
  contradiction; it is the hash, which it does not print.
- **`cp` fails with "Text file busy" if your previous run is still using the destination.** Stop it
  first, or the `&&` chain aborts after the first copy and you launch a mismatched pair.
- **Verify what you pinned**, cheaply: `strings <binary> | grep -c "<a string your change adds or
  removes>"`. One grep is much shorter than the detour it saves. Run it against **both** binaries —
  a serde field name your change adds appears in whichever of them serializes the type, so a zero on
  one side and a non-zero on the other is a mismatched pair, before you have launched anything.
- **Then confirm the pair actually connects.** Launch once and grep the sandbox daemon log for
  `client attached to daemon`; `refusing client: contract or build mismatch` means the pin failed.
  This is the check that catches every cause at once, including the ones not yet listed here.

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

`pgrep -f` is the wrong instrument here, twice over. It matches **your own shell**, whose command
line contains the pattern you are searching for, so the cleanup loop kills the script running it —
which surfaces as a bare `exit 144` and a half-executed command. And it matches on the whole command
line, so `pgrep -f micold-ai-ide` also finds `micold-daemon` when the daemon's *path* contains
`.../workspaces/micold-ai-ide/...`; killing "the app" then takes the daemon with it.

Match the executable name and confirm the instance is yours by its environment:

```bash
for n in micold-ai-ide micold-daemon; do
  for p in $(pgrep -x "$n"); do
    rt=$(tr '\0' '\n' < /proc/$p/environ 2>/dev/null | grep '^XDG_RUNTIME_DIR=' | cut -d= -f2)
    [ "$rt" = "/tmp/vp77" ] && kill "$p"
  done
done
```

Give the run its own `XDG_RUNTIME_DIR` (and `XDG_DATA_HOME`) precisely so this test exists — it is
both the isolation and the "is this mine?" predicate. If `/proc/<pid>/environ` is unreadable, the
process is not yours: leave it.

**Keep that runtime dir short.** `$XDG_RUNTIME_DIR/micold/daemon.sock` must fit in `sun_path`
(~108 bytes), and the session scratchpad path alone is longer than that — the daemon fails with
"local socket name length exceeds capacity of sun_path". `/tmp/vp77` works; the scratchpad does not.
Everything else (data home, screenshots) can live in the scratchpad as usual.

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

Commit the cropped image rather than the full frame, and only once you are satisfied with it — PNGs
do not delta-compress, so every superseded copy stays in history forever.
