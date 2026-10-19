# Visual pass — M3: spacing, labels, summaries, marker (quickstart Part B steps 4–6)

Date 2026-10-19. Branch `claude/project-thread-rwlog8`, M3 changes uncommitted in the working tree.
`micold-ai-ide` (dev build, `CARGO_INCREMENTAL=0`, binary copied out of `target-shared`), private HOME with a fake
`KUBECONFIG` of 3 contexts (`dev-cluster`, `prod-eu-west-1-with-a-rather-long-context-name`, `kind-local`), on a
private Xvfb (:77, 1280x900) with `ICED_BACKEND=tiny-skia`, driven with xdotool. No session daemon runs, so the
"Not connected" banner and snackbar show; they are not part of this change. Not a real display or GPU.
"Narrowest Settings width" = window 640 px wide (`MIN_WINDOW_SIZE`), sidebar expanded; with no window manager the
minimum is not enforced, so 640 was set by hand.

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | Container, every section open (Connection, Container, Credentials, Network, Resources, Sessions): equal 8 px field/section rhythm; `AI tools in this daemon` and `Shared with the container` labelled above their checkboxes, note under the label; supporting text directly under its field; the no-outbound caution directly under the Network select; sharing-summary caution last in Credentials | PASS | `light-container-container-credentials.png`, `dark-container-open-credentials-network.png` |
| 2 | Host open: checkbox, then Sessions caution as the section's first line | PASS | `light-host-open-and-closed.png` |
| 3 | Remote over SSH open: Host, User, Port, Key file, checkbox, even spacing (no host list configured, so no `HostPicker`) | PASS | `light-ssh-connection.png` |
| 4 | Kubernetes open: select `Context from your kubeconfig` above `Context` (lists the 3 contexts, placeholder `Choose a context`, no button column); choosing fills Context; `AI CLIs installed in the pod`, `Allowed credentials`, `Node selector`, `Tolerations` each labelled above with their note; supporting lines under their fields; no stray line between two fields | PASS | `light-kubernetes-connection-context-select.png`, `light-kubernetes-placement-storage.png` |
| 5 | Closed headers, summaries as in the contract: Container `micold-dev · Port 7050 · Connects at launch`, `Docker · Basic image · Claude Code`, `Shared: AI CLI sign-in` (changed when the box was ticked), `No outbound connections`, `CPU 2 · Memory 4096 MiB · Processes 512`, `Stops when signed out or idle`; SSH `dev@build-box:2222 · Connects at launch`; Host `Connects at launch` / `Stops when signed out or idle`; Kubernetes `<context> · Namespace team-a · Connects at launch` (after choosing and typing), `Default SSH image · Claude Code`, `Nothing allowed`, `CPU limit 2 · Memory limit 4096 MiB`, `Volume 10Gi · Default storage class`. Typing in Container name / Port / Host / User / Port / Namespace changed the Connection summary | PASS | `light-container-closed-summaries.png`, `light-kubernetes-closed-required-marker.png`, `light-host-open-and-closed.png` |
| 6 | `*` after the title with nothing typed: Kubernetes `Connection*`, SSH `Connection*` (checked in the session, not kept as an image), Container `Connection*` with `Any free port · Connects at launch` | PASS | `light-kubernetes-closed-required-marker.png`, `dark-container-closed-required-marker.png` |
| 7 | Dark theme, Container open (Container, Credentials, Network, Resources, Sessions) and closed; Kubernetes open (Connection … Placement and storage): same spacing and labels as light | PASS | `dark-container-open-credentials-network.png`, `dark-container-closed-required-marker.png` |
| 8 | Narrowest width (640 px), dark, Kubernetes open: fields, notes and checkbox labels wrap inside the column; nothing overflows the form | PASS | `dark-narrow-640-kubernetes-open-and-closed.png` (right half) |
| 9 | Narrowest width, Kubernetes closed with a 48-character context: the long summary ends in `…`, the chevron stays, headers keep one line of title + one line of summary | PASS | `dark-narrow-640-kubernetes-open-and-closed.png` (right half) |

## Defects found
- **D1 (low): a select's long value wraps and clips at a narrow width.** Kubernetes, `Context from your kubeconfig`
  with `prod-eu-west-1-with-a-rather-long-context-name` at 640 px: the value wraps to two lines and the second
  line (`name`) is cut by the field's bottom edge; the chevron overlaps the first line's end. Left half of
  `dark-narrow-640-kubernetes-open-and-closed.png`. It is the shared select control, not an M3 section, and no
  other select was given a long value, so it may predate M3. The `Context` text field below it clips its long
  value without wrapping, as a text field should.
- **D2 (outside the form, for information): the daemon list's action row clips at 640 px.** For an
  `Unreachable` daemon `Reconnect · Stop · Edit · Details · Remove` is cut after `Re`. Not touched by this
  change; `Start · Edit · Details · Remove` (a `Stopped` daemon) fits.

## Not judged
- Light theme at 640 px; Container, Host and SSH at 640 px (Kubernetes only, dark).
- Container with `My own image` (Image reference / Image file rows), SSH with configured hosts (`HostPicker`),
  the kubeconfig context-list failure caution, the key-value / toleration editor error cautions, the unshared
  sign-in caution under `AI CLI sign-in` (needs a running sandbox), and a summary change per section other than
  Connection and Credentials.
- Tab order, and the `Connection*` marker for SSH as an image.
- tiny-skia artefacts, not defects: while a text field has focus and its cursor blinks, the open section's body
  is repainted with a dark shadow tint over it (Host/User/Port, Container name/Port); it clears when focus
  leaves. Mid-transition frames and black blocks after a click also occurred; every capture was taken after a
  resize nudge and a wait. A real GPU backend was not run, so the tint is unconfirmed as an artefact.
- Closing Settings with its own Cancel keeps the add-daemon draft and its errors: re-opening Add daemon showed
  `This field is required` on Name and Container name at once. That is M2/pre-existing behaviour and was not
  exercised further.
