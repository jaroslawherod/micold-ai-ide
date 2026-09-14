# Quickstart §D.1–2 — Pi in the sandbox, in the application

Recorded 2026-09-14, against `main` at `1ee07a42`.

The scripted half of §D, `mise run test-sandbox`, is recorded in [`quickstart-C.md`](quickstart-C.md).

## Setup

- **Display.** No person at a screen. The app ran on a private Xvfb display with Mesa's lavapipe
  (software Vulkan), driven by `xdotool`, following `.claude/skills/visual-pass`. This covers static
  appearance and state changes. It says nothing about rendering on a real GPU.
- **Build.** Client and daemon came from one locked debug build of `1ee07a42`, pinned outside
  `target-shared`. The image `micold-daemon:vp029d` was built from the same tree with
  `MICOLD_IMAGE_TAG=micold-daemon:vp029d mise run image`, so it ships `pi` 0.85.1. A second image,
  `micold-daemon:vp029d-nopi`, was `FROM` it with `/usr/local/bin/pi` removed. Claude Code and
  Copilot stayed in it.
- **Placement.** Settings: *In a container*, Docker, image source *Build from this checkout*,
  network *No outbound connections*, no credentials shared, default AI CLI *Pi Coding Agent*. The
  daemon log shows `listening (sandboxed)` and `client attached to daemon`. `pi` was not on the
  host's `PATH`.
- **Model.** No real provider. A local OpenAI-compatible mock listened on the sandbox network's
  gateway address. It waited 8 s and then answered `MOCK reply #<n> to: <last user message>`. It was
  registered through `models.json` and `settings.json` in the sandbox home's `~/.pi/agent`. That is
  Pi's own provider configuration, not an install. The wait keeps the working state on screen long
  enough to sample.
- **Project.** A fresh one-commit git repository, opened by seeding `projects.json`, with private
  `XDG_*` directories.

## Results

| Step | Result |
|---|---|
| D.1 a Pi session starts in the sandbox, and its badge behaves as on the host | **Pass.** `+` on the project row started Pi in the container. `docker exec micold-sandbox ps` showed `pi` beside `micold-daemon`, and the terminal shows `pi v0.85.1` with `[Extensions] pi-activity.ts`. The component was written at session start to `pi/pi-activity.ts` in the service's state directory, not taken from the image. The sandbox home has no `extensions/` directory, and the project gained no files. One message got `MOCK reply #1`. The activity log for the session's id held only `{"type","at"}` lines: `agent_start`, `turn_start`, `turn_end`, `agent_settled` (FR-012b). The row was sampled every ~0.5 s from Enter ([`quickstart-D-sandbox-turn.png`](quickstart-D-sandbox-turn.png)). At +0.03 s there was no badge. By +0.49 s it read working, after `agent_start` at +0.006 s. At +7.81 s it still read working. By +8.30 s it read awaiting input, after `agent_settled` at +8.09 s. That is the host's sequence and the host's rendering from §B.3, within one sample of each event (FR-012a, FR-017a, SC-007a). Host timings were not sampled the same way, so "exactly as on the host" is judged by sequence and latency bound, not by a side-by-side timing. |
| D.2 an image without `pi` | **Pass.** The image reference was changed to `micold-daemon:vp029d-nopi` in *Settings → Session service* and saved. The change applies when the service next starts, so the app was restarted, and the container came up from the new image. `command -v pi` in it found nothing. Both places the user chooses report it, naming the CLI and the image ([`quickstart-D-missing-cli.png`](quickstart-D-missing-cli.png)). Under *Image reference*: `Pi Coding Agent isn't in micold-daemon:vp029d-nopi. Sessions run in that image, so it has to provide any AI CLI you want to use.` Under *Default AI CLI* in *Environment*, the same sentence. At session start, both `+` and the override chevron offer only *Claude Code* and *GitHub Copilot*. Pi is not offered, and no session was started (FR-018). |

## Findings

1. **Resuming a Pi session into an image without `pi` gives advice that cannot work.** The D.1 session
   was still resumable when the image changed. On restart, the daemon tried to resume it and logged
   `AI CLI not on PATH; not starting … cli="pi"`. The row turned failed. The pane and a toast read
   `Pi Coding Agent isn't installed. Install it, or start this session on another AI CLI. Choose
   restart below to resume it.` Three things are wrong with that advice in a container:
   - *Install it* points at this computer, but the missing thing is the image.
   - A Pi conversation cannot continue on another CLI.
   - *restart* fails the same way until the image changes.

   The settings notices already give the right wording (`isn't in <image>`). The start error did not
   use the availability source that feeds them.

   **Fixed** after this pass, not re-run in the application. The daemon now words the refusal by
   where sessions run (its `MICOLD_IMAGE_REFERENCE`) and by whether it is a resume. This case now
   reads `Pi Coding Agent isn't in micold-daemon:vp029d-nopi, where sessions run, and this
   conversation can only continue in it. Choose an image that provides it, then restart this
   session.` A fresh start on the host keeps the old sentence. The pane no longer appends *Choose
   restart below* to a refused start (`attempts: 0`), since each refusal says what to change. It
   still does for a crash-loop give-up. Pinned by
   `session_start.rs::a_missing_cli_is_advised_on_where_sessions_run_and_on_what_is_being_started`
   and `ui::terminal`'s `a_refused_start_is_not_pointed_at_restart`.
2. **Carried over from §B, finding 2.** Pi's yellow `Warning: No project session found with id …`
   opens every new session, in the sandbox as on the host.

`fd not found` and `ripgrep not found` come from the image not shipping them, in offline mode. They
are not from the application.

## Not covered

- Rendering on a real GPU or display; see Setup.
- The *Registry* and *Image file* image sources. Only *Build from this checkout* was used.
- An image missing more than one CLI, where the notice names several.
- Podman.
