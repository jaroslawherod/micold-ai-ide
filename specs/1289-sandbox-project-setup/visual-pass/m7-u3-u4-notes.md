# Visual pass M7 (U3 Run/Stop setup, U4 Stop setup) — 2026-10-26

Ran on Xvfb :77 (1600x1400) + Mesa lavapipe (WGPU_BACKEND=vulkan), driven with xdotool; not a real
display. micold-ai-ide + micold-daemon built together (debug, CARGO_PROFILE_DEV_DEBUG=0) from
claude/project-thread-d7q05f @ 9e7407c73, pinned copies; "client attached to daemon" in the daemon
log. Host daemon spawned by the client, private XDG_DATA_HOME/XDG_RUNTIME_DIR, git project
"demo-app". The setup command was seeded once (`sleep 900`) and then changed in Settings › Project;
worktrees were created from the New worktree form. Runs were started and stopped from the app only
(Run setup now exists; no record was seeded on disk). Window sizes 1600x1000 ("large") and 640x480.

## U3 menus — pass

| State | Worktree row | Default row | Overview card |
|---|---|---|---|
| command, no record | … Run setup, Delete (`*-worktree-menu-norecord`) | Review changes, Run setup (`m7-u3-light-default-menu-norecord`) | — |
| running | … Setup output, Stop setup, Delete (`*-worktree-menu-running`) | …, Setup output, Stop setup (`*-default-menu-running`) | … Setup output, Stop setup, Delete (`*-overview-card-menu-running`) |
| stopped | … Setup output, Run setup, Delete (`m7-u3-light-worktree-menu-stopped`, `m7-u3-dark-640-worktree-menu-stopped`) | …, Setup output, Run setup (`*-default-menu-stopped`, `m7-u3-light-overview-default-card-menu`) | … Setup output, Run setup, Delete (`m7-u3-light-overview-card-menu-stopped`) |
| failed | … Setup output, Run setup, Delete (`*-worktree-menu-failed`) | | |
| succeeded | … Setup output, Run setup, Delete (`m7-u3-light-worktree-menu-succeeded`) | | same (`m7-u3-dark-overview-card-menu-succeeded`) |
| interrupted | … Setup output, Run setup, Delete (`m7-u3-light-worktree-menu-interrupted`) | | |
| no command | … Setup output, Delete; no Run setup (`*-worktree-menu-nocommand`) | Review changes, Setup output (`*-default-menu-nocommand`) | |

No Run setup appears while running; no Stop setup when not running. Choosing Run setup or Stop setup
closed the menu every time (row menus and card menus, light and dark). At 640x480 the 9-entry worktree
menu opens upward and fits the window. Interrupted was produced by SIGKILLing the host daemon during
a `sleep 900` run; the client respawned it and the row showed *Setup interrupted*.

Row tags: Run setup → *Setting up…* (`m7-u3-dark-rows-running-setup-interrupted`), then *Setup failed*
(`echo before; exit 2`; `*-row-failed-tag`, `m7-u3-dark-rows-failed`) or *Set up* (`echo ok`;
`m7-u3-light-row-setup-succeeded`). Stop setup → *Setup stopped*, not failed, on Default, worktree
rows and from a card (`m7-u3-light-rows-stopped-tags`, `m7-u3-dark-640-rows-stopped`,
`m7-u3-dark-overview-after-card-stop`); `ps` showed no `sleep 900` after each stop. Failure
notification: "Setup failed in demo-app / Norec: Exited with status 2." (`*-failed-notification`); none
on stop. Tags fit the sidebar at its default width in both themes; "Setup interrupted" is the widest.

## U4 Stop setup — pass

Running: action row reads Copy output, Stop setup, Close (last, filled), all inside the dialog at
1600x1000 and 640x480, light and dark (`m7-u4-*-running-large`, `m7-u4-*-640-running`). Clicking
Stop setup ended the run: state line "Setup stopped at …", body "Stopped.", the button disappeared
(`m7-u4-*-640-after-stop`), and the row showed *Setup stopped*. Failed (`m7-u4-light-failed-large`,
`m7-u4-dark-640-failed`), Succeeded (`m7-u4-dark-640-succeeded`, "ok" / "Exited with status 0.") and
Stopped show only Copy output and Close.

## Busy — not run

Not reachable from the UI: a second client on the same project opens read-only ("This project is
already open in another window", no row menus, no setup tags), and the one client hides Run setup
while a run is running. The daemon's reply text is "Setup is already running in <dir|Default>." and the
client wraps it as an error for "run setup" (read from source, not seen on screen).

## Findings

- F1 MAJOR — Settings › Project, `micold-core` `JsonFileSettingsStore::save` — clearing the setup
  command does not persist. The client and the daemon drop it (no Run setup, a new worktree runs no
  setup), but `settings.json` keeps the old entry. After a client restart the old command is back:
  Run setup is offered again, and the command is sent to the daemon again. Seen on 3 of 3 clears.
  Cause: `StoredSettings.setup_commands` is `skip_serializing_if = "BTreeMap::is_empty"`, and
  `merged_with_existing` keeps top-level keys that the fresh document lacks. So clearing the last
  command leaves the stale key in place. The unit test `no_setup_command_writes_no_key` saves into an
  empty dir, so it does not catch this. `jira_base_urls` has the same shape and may have the same bug.
  `in_force()` re-reads the store, so a reconnect may also bring the command back without a restart.
  This was not checked.

Observations (not findings): **Stop setup** reuses the X (`Icon::Close`) glyph, the same one as
**Close session** two entries above it. The material icon set has no stop glyph. SIGKILLing the daemon
left the setup's `sleep 900` running, orphaned. The location names differ by place: the notification
and dialog title use the friendly name ("Norec"), while the row shows "feat/norec". M6 recorded the
same. The worktrees started real `claude` sessions, as in M6.

Not covered: Busy notification (above); Copy output clipboard contents; animation.
