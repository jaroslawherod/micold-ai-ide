# BUG-005 visual pass — known-projects list and switcher scroll on overflow

Date: 2026-09-27
Branch: `fix/product-list-don-t-have-scroll`, HEAD `e00071f0` ("fix(002): the known-projects list
and the switcher panel scroll when they overflow (BUG-005)")
Task: T069 (spec `specs/002-project-workspace-management`)

## Setup

- Ran on Xvfb `:77` (1600x1400x24) + lavapipe (`WGPU_BACKEND=vulkan`,
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`), no real display or window manager.
- Built `micold-ai-ide` (client) and `micold-daemon` from this worktree's HEAD in one
  `scripts/build-lock.sh cargo build -p micold-client --bin micold-ai-ide -p micold-daemon --bin
  micold-daemon` (both crates present in the compile list; the existing `target-shared` build was
  already current for this commit). Verified the pinned pair with `strings … | grep -c "Known
  projects"` / `"No project open"` (1 each) before copying to a private `~/vp/bin`, then confirmed
  the daemon log line `client attached to daemon` for this run's own `client_window` PID.
- Private `XDG_RUNTIME_DIR=/tmp/vp77`, `XDG_DATA_HOME=$HOME/.cache/vp77/data` — never the
  developer's real data directory. The client has no CLI, so the known-projects catalog was seeded
  directly at `$XDG_DATA_HOME/micold-ai-ide/projects.json` (the daemon is the file's only writer
  but freely loads whatever is there at startup) with 20 real temp folders created under the
  session scratchpad (`.../scratchpad/projects20/proj-00` … `proj-19`), then trimmed to 3 for the
  fourth check. The catalog was rewritten and the client/daemon pair restarted (same runtime dir)
  between the 20-project and 3-project checks, since the daemon only loads the catalog once at
  startup.
- Window sized 1280x800 for checks 1-4, and 1280x600 for a short-window confirmation.
- Driven with `xdotool` (mousemove/click/click 5 for wheel) since there is no window manager on
  the private display; every key input reused `windowfocus` first (not needed here — no keyboard
  interaction was required for these checks).
- Cleaned up by PID, filtered to processes whose `/proc/<pid>/environ` held this run's own
  `XDG_RUNTIME_DIR=/tmp/vp77` (never `pkill -f`), then killed the private Xvfb.

## Checks

### 1 — body list shows a themed scrollbar at its right edge (20 known projects, 1280x800)

PASS. `check1-body-scrollbar.png` (full list top, proj-00..proj-05 visible under the fixed "No
project open" header) and `check1-body-scrollbar-zoom.png` (4x crop of the right edge) both show a
thin themed scrollbar track running down the right edge of the "Known projects" list.

### 2 — mouse wheel over the body list scrolls to proj-19 with actions visible, header stays put

PASS. After 25 wheel notches over the list (`xdotool click 5` at (200, 450), inside the list, not
the header), `check2-body-scrolled-to-proj19.png` shows proj-14..proj-19 with proj-19's Open /
Rename / Forget buttons fully visible, while "No project open" and "Open a project" remain fixed at
the top of the frame. Re-confirmed at a short 1280x600 window
(`check2-short-window-1280x600.png`): the scrollbar is present immediately (less room to show more
rows) and the header still does not move.

### 3 — switcher panel ends inside the window with a scrollbar; wheel reaches proj-19 and "Add project…"

PASS. Opening "Select project" in the app bar (`check3-switcher-open.png`) shows the panel opening
below the app bar and stopping at the window's bottom edge (visible black margin under the panel,
not clipped past it) with a scrollbar on its right edge — confirmed by the 4x crop
`check3-switcher-scrollbar-zoom.png`, which shows the scrollbar's thumb ending above the panel's
own bottom edge. After 25 wheel notches over the panel (mouse at (1150, 400), inside the panel's
x-range), `check3-switcher-scrolled-to-end.png` shows proj-14..proj-19 and "Add project…" all on
screen.

### 4 — with only 3 known projects: no scrollbar in either list, switcher panel is short (4 rows)

PASS. `check4-three-projects-body-no-scroll.png` shows proj-00/01/02 with a clean right edge (no
scrollbar) below the fixed header. `check4-three-projects-switcher-short.png` shows the switcher
panel sized to exactly 4 rows (proj-00, proj-01, proj-02, "Add project…") with no scrollbar.

## Overall: PASS (4/4)

## What this did not cover

- Mid-flight animation / transition smoothness of the panel opening or the scrollbar fading in —
  not answerable from a screenshot pipeline (see the visual-pass skill's own caveat). Only the at-
  rest states above were checked.
- Real-GPU frame pacing: this ran on lavapipe (software Vulkan), not the user's own GPU.

## Screenshots

All under `specs/002-project-workspace-management/evidence/BUG-005/` (downscaled, each well under
150KB):

- `check1-body-scrollbar.png`, `check1-body-scrollbar-zoom.png`
- `check2-body-scrolled-to-proj19.png`, `check2-short-window-1280x600.png`
- `check3-switcher-open.png`, `check3-switcher-scrollbar-zoom.png`,
  `check3-switcher-scrolled-to-end.png`
- `check4-three-projects-body-no-scroll.png`, `check4-three-projects-switcher-short.png`
