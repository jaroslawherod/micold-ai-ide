# M2 quickstart B6 (M2 half): Settings toggle row

Date 2026-09-30. Xvfb :81 + lavapipe (software Vulkan), not a real display. Client and daemon built from this
worktree (branch feat/daemon-should-expose-mcp-server-for-agent), copied to a private dir; private XDG data/runtime dirs;
daemon log shows "client attached to daemon" (pair connects). Settings -> Environment, crops at 1x.

Each image: top half = toggle on (default), bottom half = after one click on the new row.

- dark-toggle.png: PASS. Row sits directly under "Show activity for Pi sessions"; same checkbox, label size/weight,
  note typography, indent and spacing; label and note fully legible; click flips checked -> unchecked.
- light-toggle.png: PASS. Same as dark (theme set to Light in Appearance and saved into the private data home).

Not run: FR-016 "read and type into other sessions" row (M6, not expected yet); toggle persistence across Save;
no animation or real-GPU check.
