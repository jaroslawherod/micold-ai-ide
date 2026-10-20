# Visual pass notes: 1091 quickstart Part B steps 2-5

Date 2026-10-20. Real client (micold-ai-ide + micold-daemon, debug, rebuilt from HEAD cf7c64d7d, both
copied out of target-shared; "client attached to daemon" confirmed). Private headless sway 1600x1200,
pixman + lavapipe, virtual pointer (vptr.py) and wtype; private XDG_RUNTIME_DIR/XDG_DATA_HOME.
Screenshots are 50% downscales. Settings > Daemons > Add daemon. Step 1 (showcase) passed earlier in M2.

- Step 2 PASS (mouse): Container kind opens with Connection open; opening Credentials closes Connection;
  opening Network closes Credentials; pressing the open Network leaves all closed. Not done: Kubernetes,
  SSH and Remote groups were only checked for "first section open" (step 5), not for toggling; no daemon
  was saved, the add form was used for each kind. Space/Enter on a focused header NOT RUN: Tab produced
  no visible focus change in the client (focus ring not visible, and Tab/Space/Enter changed no pixel),
  so keyboard activation could not be confirmed here (covered only by unit tests in accordion_section.rs).
- Step 3 PASS: empty required Container name in a closed section shows "Connection*" in its title.
  After Save daemon the error is shown and Connection is forced open; opening Credentials leaves both
  open; typing a container name closes Connection (summary "abc - Any free port") and Credentials stays.
  Note: before a save attempt the closed section shows the marker but is NOT forced open (error not yet shown).
- Step 4 PARTIAL: dark and light (Settings > Appearance > Light > Save) both render the group with no
  overlap or stray artefacts in settled frames (Host kind in light: Sessions open closes Connection, pressing
  Connection reopens it and closes Sessions). Mid-animation overlap or flicker cannot be seen by screenshots.
- Step 5 PASS: Runs in = Kubernetes, Remote over SSH, Host each show only the new kind's first section
  (Connection) open; the others closed.
