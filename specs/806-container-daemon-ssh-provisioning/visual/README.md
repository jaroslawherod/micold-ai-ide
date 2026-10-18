# Feature 806 visual pass (partial)

Real client (`micold-ai-ide`, debug build), `ICED_BACKEND=tiny-skia`, private Xvfb 1024x768 window, private HOME, no container runtime. Quickstart Part B.

| # | Check | Verdict | Evidence |
|---|-------|---------|----------|
| 1 | Image select shows exactly "Basic image (recommended)" and "My own image" | PASS | `b1-image-select.png` |
| 1 | AI CLIs: five boxes, only Claude Code ticked | PASS | `b1-ai-cli-section.png` (fresh build of HEAD f0ca1c0d): "AI tools in this daemon" lists Claude Code (ticked), GitHub Copilot, Pi Coding Agent, Codex, OpenCode (unticked), with the note "Changes apply at the next start...". The heading reads "AI tools", not "AI CLIs" as the quickstart says. |
| 5 | Install failure: row names Codex as not installed with npm output | NOT-CHECKABLE | The warning only comes from a real provisioning run (`ProvisionEvent::Warned`, shown by `ui/settings/daemons.rs` under the row); it needs a container runtime and an SSH server in it, and neither the showcase nor persisted settings can inject one. Covered by `tests/daemons_feature.rs` (state level), not seen on screen. |
| 3 | CPU and memory fields exist and take 2 / 4 GiB | PASS (form only) | `b3-limits.png`: "Processor limit" 2 (cores), "Memory limit" 4096 (mebibytes). `docker inspect` not checked. |
| 4 | "My own image": SSH-server note, reference and archive fields | PASS | `b4-own-image.png`: "Your image must run an SSH server; see the user guide.", Image reference, Image file |
| 6 | Migration: row says moved off the released image | PASS | `b6-migration-row.png`: "Moved off the released daemon image; it moves to the basic image on its next start." |
| 6 | Migration: form shows basic image | PASS | `b1-image-select.png` (Edit form of the migrated daemon shows "Basic image (recommended)") |
| 6 | Migration: all five CLIs ticked | NOT-CHECKABLE | No AI CLI section yet (as above) |
| 6 | Start clears the notice | NOT-CHECKABLE | Needs a runtime |
| 7 | Legacy container row with sessions | NOT-CHECKABLE | Needs a runtime |

SC-002 (add to session in under 5 minutes) is not measurable without a container runtime; CI's sandbox suite covers the sequence.

## Defects / observations

- Seeded migration settings made the client auto-start the container daemon; the red banner "The sandbox did not start ... could not create the container's SSH keys in <data dir>/ssh: No such file or directory (os error 2)" appeared although the data dir existed and `ssh/` was created afterwards. Looks like the key dir is created non-recursively or after use on first run; not reproduced with a clean retry.
- Scrolling and editing the settings form under tiny-skia/Xvfb left stale or black repaint regions (shadow layers) until the next repaint. Likely a software-renderer damage artifact; glyphs and weights in the container section were otherwise clean.

Note: the red "could not create the container's SSH keys" banner seen in the pass came from this box (no `ssh-keygen` on PATH if `which ssh-keygen` is empty); `ensure_dir` is recursive. Not a form defect.
