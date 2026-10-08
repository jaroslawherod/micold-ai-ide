---
name: visual-pass
description: Run a spec's manual visual pass (quickstart Part B) against the real GUI without a human — launch the component showcase or the client on a private Xvfb display (or, with no Xvfb, a private headless sway: see "Wayland"), drive it with xdotool (or a virtual pointer), screenshot with import (or grim), and look at the result. Use whenever a task says "needs eyes at a display", "record the pass", "run quickstart §B", or when a change alters how something *looks* and the geometry gates cannot see it (colour, weight, elevation, glyph collisions, state layers, floated labels, active indicators).
context: fork
model: sonnet
background: false
---

# Running the manual visual pass without a human

**This skill runs as a forked Sonnet subagent** (`context: fork`). It sees none of the caller's
conversation, only the arguments: the worktree path, the quickstart section or the change to check,
and what counts as a pass. It returns one verdict per check with the paths of the cropped
screenshots that show it, so the caller can open a crop when a verdict looks doubtful. Screenshots
stay in this subagent's context, not the caller's.

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
top=$(git rev-parse --show-toplevel); mkdir -p "$top/.visual-pass"
VP=$(mktemp -d "$top/.visual-pass/run.XXXXXX"); mkdir -p "$VP/bin"; echo "VP=$VP"
scripts/build-lock.sh bash -c \
  'cargo build -p micold-client --bin micold-ai-ide -p micold-daemon --bin micold-daemon &&
   cp "$CARGO_TARGET_DIR/debug/micold-ai-ide" "$CARGO_TARGET_DIR/debug/micold-daemon" "$1"/bin/' _ "$VP"
```

**Where working files go.** `$VP`, a run directory `run.XXXXXX` that `mktemp -d` creates inside the
git-ignored `.visual-pass/` at the root of the worktree running the pass. `.visual-pass/` keeps the
files out of `$HOME` and next to the worktree they belong to; the `mktemp` suffix makes each pass's
directory its own, so two sessions cannot overwrite each other's pinned binaries even in the same
worktree. Never `~/vp` or any other folder under `$HOME`. Shell state does not persist between calls:
note the absolute path the block prints and set `VP=<that path>` at the start of each later call,
never a fresh `mktemp`. A "before" build needs the old source: export it with
`mkdir -p "$VP/before-src" && git -C "$(git rev-parse --show-toplevel)" archive <rev> | tar -x -C "$VP/before-src"`,
never a helper `git worktree add`. An export registers nothing with git, so there is no worktree for
housekeeping to miss and none for the autopilot's hook (which forbids removing one) to block. When the
pass ends, step 9 removes it with `rm -rf "${VP:?}"`, which touches only this run's directory. `mise run
sweep` and the `reclaim-disk` skill do not clean `.visual-pass/`.

**Name both bins.** `--bin` filters the whole invocation to the targets it names, so
`-p micold-client --bin micold-ai-ide -p micold-daemon` — what this recipe said until 2026-08-18 —
builds the client and **silently skips the daemon**, leaving whatever `target-shared` already held.
The `cp` then pins a matched-looking pair that is not one. It cost a pass two rounds: the daemon was
a version behind, and the log said so plainly once it was read (`client_version=6 … daemon_version=5`).

**Check that it happened**, rather than trusting the flags: `cargo build … | grep "Compiling micold"`
should name **micold-core, micold-client and micold-daemon**. If the daemon is absent from that
list, nothing you copy is yours — which is what the mismatch below actually means, and the reason it
reads as a schema-hash quirk instead.

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
vp_n=77                                     # the first number to try
until [ ! -e "/tmp/.X11-unix/X$vp_n" ] && mkdir -m 700 "/tmp/vp$vp_n" 2>/dev/null; do
  vp_n=$((vp_n + 1))
done
echo "vp_n=$vp_n"                           # yours now: the number every later step uses
Xvfb ":$vp_n" -screen 0 1600x1400x24 -nolisten tcp &
```

1600×1400 is deliberate: tall enough that a section and the list it floats fit in one frame, so a
comparison is one screenshot rather than two you have to hold in your head.

**Claim a number nobody else is on**, and keep using that one number. `mkdir` is atomic, so the
run's runtime dir `/tmp/vp$vp_n` is the lock: two sessions starting at once, even in the same
worktree on the same diff, cannot both create it, and the loser moves on to the next number. Step 9
removes it, which releases the number. Use the number the block printed: step 4 derives this run's
runtime and data directories from it, and step 9 recognises its own processes by them. Another
session on the same display shows you its window and lets its cleanup stop your processes.

### 4. Launch — **with lavapipe, and with a data directory of your own**

```bash
# The display number you took in step 3 -- the one thing you choose, and the one thing that makes
# these paths yours. Repeat these four lines verbatim in every later call: a Bash call is a new
# shell, so the variables do not survive, and the cleanup step has to derive the same runtime dir.
vp_n=77                                      # the number step 3 printed, not a fresh guess
vp_run=/tmp/vp$vp_n                          # short, private: see sun_path below
vp_data=$HOME/.cache/vp$vp_n/data            # anywhere only you write, spelled out in full
case "$vp_run:$vp_data" in /*:/*) ;; *) echo "both paths must be absolute" >&2; exit 1;; esac
mkdir -p "$vp_run" "$vp_data" && chmod 700 "$vp_run" "$vp_data"

env -u WAYLAND_DISPLAY DISPLAY=":$vp_n" \
    XDG_RUNTIME_DIR="$vp_run" \
    XDG_DATA_HOME="$vp_data" \
    WGPU_BACKEND=vulkan \
    VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
    setsid nohup <binary> > run.log 2>&1 &
```

`env -u WAYLAND_DISPLAY` is required — winit prefers Wayland and will ignore `DISPLAY` entirely.

**`XDG_DATA_HOME` is not optional, and neither is `XDG_RUNTIME_DIR`.** Without them the client and
the daemon you just launched resolve their data directory to the developer's own
`~/.local/share/micold-ai-ide` and write into it: `settings.json`, `projects.json`,
`micold-client.log`, the materialised Pi component. A pass that drives the Settings screen then
leaves the fixture values it typed in the user's real settings file, where they surface later as a
bug report about a setting nobody chose. Seed the project you want opened into *your* data home's
`projects.json`, never the real one — the client binary has no command line to open a project with.

**Both paths must be absolute, and neither may keep a placeholder.** That is what the `case` line
checks, and it is not ceremony. `directories` discards an `XDG_DATA_HOME` that is empty or relative
and falls back to `$HOME/.local/share` *without saying so*, so a mistake here does not fail — it
quietly writes the real directory, which is the one outcome this whole section exists to prevent.
Writing the paths out rather than leaving a `<your scratchpad>` placeholder is the point: a
placeholder in angle brackets is not an assignment to bash at all, it is an empty assignment plus two
redirections, and it leaves `XDG_DATA_HOME` set to the empty string — which is the silent fallback.

**Both paths must also be yours alone, which is why they carry the display number.** Two passes can
be in flight on this machine at once — this has happened — and paths shared between them collide
twice over: the cleanup step below identifies the processes it may stop by their `XDG_RUNTIME_DIR`,
so one session kills the other's client and daemon mid-run, and both resolve the same
`$XDG_RUNTIME_DIR/micold/daemon.sock`, so the second client attaches to the first session's daemon —
a different build than the one it just pinned — instead of spawning its own. Take a display number
nobody else is on in step 3 and everything downstream is private; `chmod 700` keeps other users out.

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

Then **Read a downscaled copy, never the full frame**: `convert shot.png -resize 45% small.png` and
Read `small.png`. A blank or black frame is a launch failure, not a result.

**Every image you Read stays in context for the rest of the run**, and a 1600×1400 frame is the
most expensive thing this skill does. So the rule has no exceptions: downscale to locate, crop to
judge (§7), and Read full resolution only as a crop of the region under test.

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

**Scrolling is trial and error** and costs turns. Locate on the downscaled copy, and batch the moves:
one Bash call can scroll, capture and downscale, instead of one call per step. Full resolution is
only needed once you are on the thing you came to look at, and then only as a crop.

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
vp_n=77                      # the same display number as in step 4
vp_run=/tmp/vp$vp_n
for n in micold-ai-ide micold-daemon; do
  for p in $(pgrep -x "$n"); do
    rt=$(tr '\0' '\n' < /proc/$p/environ 2>/dev/null | grep '^XDG_RUNTIME_DIR=' | cut -d= -f2)
    [ "$rt" = "$vp_run" ] && kill "$p"
  done
done
rm -rf "${vp_run:?}" "${VP:?}"  # release the display number; VP=<the path step 2 printed>
```

Step 4 gives the run its own `XDG_RUNTIME_DIR` (and `XDG_DATA_HOME`) precisely so this test exists —
it is both the isolation and the "is this mine?" predicate, which is why the runtime dir has to be
derived the same way in both places and has to carry your own display number rather than a constant
every session shares. If `/proc/<pid>/environ` is unreadable, the process is not yours: leave it. A
process whose runtime dir is the developer's real one is never yours either — the user's own app runs
that way.

**Keep that runtime dir short.** `$XDG_RUNTIME_DIR/micold/daemon.sock` must fit in `sun_path`
(~108 bytes), and the session scratchpad path alone is longer than that — the daemon fails with
"local socket name length exceeds capacity of sun_path". `/tmp/vp77` works; the scratchpad does not.
Everything else (data home, screenshots) can live in the scratchpad as usual.

## Wayland: when there is no Xvfb

Verified once, 2026-10-08, sway 1.11, headless + pixman + lavapipe (the #430 pass, checks 1-6).
Use it when Xvfb or xdotool is absent, or the user prefers it. Steps 1, 2, 7 and 8 are unchanged;
this replaces 3, 4, 5, 6 and 9.

**Hard rules.** Never ydotool/uinput: it moves the user's real pointer. Never run grim, swaymsg,
wtype or `vptr.py` against the user's own `WAYLAND_DISPLAY` or `SWAYSOCK`: set both explicitly to
the private ones. `vptr.py` exits non-zero unless its socket is under `/tmp/vp*`.

**3. Private sway.** Claim `/tmp/vpwN` as in step 3 (`mkdir` is the lock). Config:
`output HEADLESS-1 resolution 1600x1200`. Start with `WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1
WLR_RENDERER=pixman XDG_RUNTIME_DIR=/tmp/vpwN setsid nohup sway -c <config>`. "Could not find config
for output HEADLESS-1" is harmless. Sway's pid is the number in the socket name
`sway-ipc.UID.PID.sock`; `$!` is setsid's, not sway's. Record that pid.

**Pointer.** Headless sway has no input devices, so `swaymsg seat - cursor ...` does nothing, and
`wlrctl` cannot hover: its virtual pointer lives only while the command runs, so the app sees
`wl_seat` pointer capability appear and vanish within microseconds. Use a persistent client:

```bash
sock=$vp_run/wayland-1   # sway's socket: ls $vp_run
mkfifo "$vp_run/ptr"; WAYLAND_DISPLAY=$sock XDG_RUNTIME_DIR=$vp_run \
  setsid nohup python3 .claude/skills/visual-pass/vptr.py "$vp_run/ptr" > ptr.log 2>&1 &
exec 3<>"$vp_run/ptr"   # hold it open; later: echo "abs 800 600" >&3
```

Commands: `abs X Y`, `rel`, `click`, `down`, `up`, `scroll N`, `quit` (absolute motion uses
1600x1200 extents, so no corner trick). Start it **before** the app, then `swaymsg -t get_seats`
shows capabilities 1; an app already running must be restarted. That check only proves the pointer
exists: confirm a hover or scroll took effect by a pixel diff (`magick compare -metric AE a.png b.png`).

**4. Launch.** As step 4, but its own short runtime dir (`XDG_RUNTIME_DIR=/tmp/vpwN/app`, so
`daemon.sock` fits `sun_path`), `WAYLAND_DISPLAY` set to the **absolute** path of the private socket,
a private `XDG_DATA_HOME`, no `DISPLAY`, and `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json`
(Fedora's name; `lvp_icd.json` does not exist there).

**5. Capture.** `grim -o HEADLESS-1 shot.png` (grim does not draw the cursor). Window content starts
near x=2, y=27 (27 px title bar); grim and sway coordinates are identical. Crop directly with
`grim -g "X,Y WxH" crop.png` (a space, not a comma). Downscale with `magick`, not the deprecated `convert`.

**6. Driving.** Scroll units are 1 px: use big values (`scroll -30000` for the top). Park the pointer
at the right margin (x about 1590) so a scroll over the terminal pane is not captured. The page is
long: find sections by catalogue order (Tooltip is just before Ripple and Motion). Edge-flip checks
need a narrow window: `swaymsg 'floating enable, resize set width 520 px height 900 px, move position 0 0'`.
At 520 px the delay triggers are clipped, so `floating disable` for delay checks. For the bottom
edge, scroll so the trigger runs past the window bottom.

**9. Clean up.** Sway's `/proc/<pid>/environ` is unreadable, so step 9's environ test cannot
identify it: kill it by the pid recorded in step 3. Kill the app and daemon by the environ test
(runtime dir `/tmp/vpwN/app`), send `quit` to the pointer client, then `rm -rf` the runtime dir.

**Traps.** A Bash call containing `rm -rf $S/...png` is blocked by the safety check, and a denied
call runs nothing, kills in the same call included: use literal paths or `cd "$S" && rm -f ...`.
Builds queue behind other worktrees' lock: wait on the binary's existence with a background
`until` loop or `scripts/autopilot/hold.sh`, not a `pgrep` watcher (it mis-fired).

**Cannot judge here, beyond the limits above.** Timing is good to about +-100 ms (grim latency).
Not smoothness or frame pacing, real GPU behaviour, idle power (an idle CPU of ~170% was seen with
no tooltip open and could not be attributed without a baseline), the cursor image, or mid-animation frames.

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
