# Visual pass M5 (U1, U2) — 2026-10-25

Ran on Xvfb :77 (1600x1400) + Mesa lavapipe (WGPU_BACKEND=vulkan), not a real display, driven with xdotool.
Binaries: micold-ai-ide + micold-daemon built together from claude/project-thread-d7q05f @ 1b309b134
(source identical to snapshot 6cf38804d); pair confirmed by "client attached to daemon".
Host daemon (spawned by the client), private XDG_DATA_HOME/XDG_RUNTIME_DIR, git project "demo-app".

- U1 project open (light+dark): Project rail entry (house/project-root icon) after Jira, heading
  "demo-app", "Setup command" monospace TextArea 4 rows, grows to 10 then scrolls (wheel shows
  lines 1-10 of 12), help line verbatim. Save writes settings.json setup_commands.
- U1 no project (light+dark): only "Open a project to set its setup command." — no field.
- U2: sleep 60 -> "Setting up…" then "Set up"; true -> "Set up"; `echo before; exit 2` -> "Setup failed"
  + one error notification "Setup failed in demo-app / Failing setup: Exited with status 2.";
  SIGTERM to the daemon mid `sleep 300` -> client respawned daemon -> "Setup interrupted".
  Tags last on the second line; Failed error tint, Interrupted warning tint, others neutral; tooltips
  "Setup running since / finished at / failed at 2026-10-25 HH:MM[: Exited with status 2]",
  "Setup interrupted: the daemon stopped". Title and state icon unmoved.
- Not covered: Stopped (needs U3 menu), U3, U4; animation/smoothness.
