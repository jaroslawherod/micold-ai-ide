# Visual pass M5 (813): NOT RUN, environment blocked

Date: 2026-10-17. Scope: quickstart B2, B3, B4 plus showcase ProgressBar (0/10, 6/10, 10/10) and
OverviewCard with/without progress.

Result: no check was exercised; nothing is marked passed.

Blockers found on this machine:
- `xdotool` is not installed (no xte, python Xlib or xcffib either), so the showcase cannot be
  scrolled to the ProgressBar / OverviewCard sections (surfaces.rs, ~lines 341 and 387) and no
  keys or clicks can be sent. No sway or grim for the Wayland route either.
- No Vulkan ICD for software rendering: `/usr/share/vulkan/icd.d` is absent, no lavapipe
  (`lvp_icd*`, `libvulkan_lvp*`) anywhere. `WGPU_BACKEND=gl` is known to fail on Xvfb.
- Fixing both needs `apt-get install xdotool mesa-vulkan-drivers`, a system change not
  authorised for this task.
- B2/B3/B4 additionally need the real client + daemon with a fixture project (worktrees
  `feat-42_alpha` with `specs/042-alpha/tasks.md` at 6/10, `misc`, a missing folder), `gh`
  signed in or the client tests' fake, and hooks. None of that is drivable here without a human
  or fixture data; B3 also needs a live `tasks.md` edit watched for 5 s.

Disk: 21 GB free (above the 8 GB floor). A prebuilt `target-shared/debug/micold-showcase` exists
(built 22:21, after HEAD 22:05).
