# Visual pass — M2: daemon form sections (quickstart Part B steps 2–3)

Date 2026-10-18. Branch `claude/project-thread-rwlog8`. `micold-ai-ide` (dev build, `CARGO_INCREMENTAL=0`),
private HOME, on a private Xvfb (:77, 1280x900) with `ICED_BACKEND=tiny-skia` (no Vulkan ICD), driven with
xdotool (`windowfocus` first, or typed keys do not arrive). No session daemon runs, so the "Not connected"
banner and snackbar show in every shot; they are not part of this change. Not a real display or GPU.

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | Container: Name and Runs in above; sections Connection (open), Container, Credentials, Network, Resources, Sessions (closed); Save daemon / Cancel below | PASS | `light-runs-in-options.png` (top), `dark-container-sections.png` |
| 2 | Host: Connection (open), Sessions (closed) | PASS | `light-host.png` |
| 3 | Remote over SSH: one section, Connection, open (Host, User, Port, …) | PASS | `light-ssh.png` |
| 4 | Kubernetes: Connection (open), Image, Credentials, Resources, Placement and storage (closed) | PASS | `light-kubernetes.png` |
| 5 | Kubernetes: clear Context, Tab away, click the Connection header: it stays open with "This field is required" | PASS | `light-kubernetes-error-reopens-connection.png` |
| 6 | Type `x` after the CPU limit (`2x`), close Resources, Save daemon: Resources open, "Enter a number of cores, like 2 or 1.5." | PASS | `light-kubernetes-save-reopens-resources.png`, `dark-kubernetes-resources-error.png` |
| 7 | Dark theme: the same section list and an error-opened Connection | PASS | `dark-container.png`, `dark-container-sections.png` |

## Not judged
- Tab traversal through every header and the open fields (step 2's last sentence): one Tab was sent,
  the focus order was not walked.
- Host, SSH and Kubernetes were looked at in light theme only; dark was Container and the Resources error.
- Spacing and alignment between fields and sections is M3's pass (steps 4–6), not judged here.
- tiny-skia leaves stale-damage artefacts; every capture was taken after a window-resize nudge.
