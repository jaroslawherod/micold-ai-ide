# 010 T033: quickstart step 11 re-run against the BUG-001 fix

**Date**: 2026-09-13
**Run by**: an agent, not a person at a display. Xvfb `:95` at 1600×1400 with Mesa lavapipe
(software Vulkan), driven with `xdotool` and captured with `import`, per the repo's `visual-pass`
skill.
**Build**: `micold-ai-ide` and `micold-daemon` from branch `fix/daemon-session-start-open-bugs` at
`9d924be2` (`fix(010): report a start refused over a missing folder as failed`). Both were built in
one invocation, with all three `micold-*` crates recompiled, and copied into
`~/vp/bin-daemon-start/` inside the build lock. `strings` finds the new reason text ("folder no
longer exists") and the new log line ("session spawn refused; not starting") in the pinned daemon.
The daemon log shows `client attached to daemon`, so the pair connected.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp95` and a scratch `XDG_DATA_HOME`/`XDG_CONFIG_HOME` whose
seeded `projects.json` names only the fixture. Every process started here was stopped afterwards,
by PID, after checking that its environment was this run's.
**Fixture**: `/home/jaro/.aaa-vp95/r10`, a fresh `git init -b main` repository with one commit and
no worktrees. It is the same shape as T029's `r10`.

## Result: PASS

Same procedure as the failing T029 run: with the project open, rename its root away
(`mv r10 r10-moved`) outside the app, then press the Default entry's **+**.

| What step 11 asks | Observed |
|---|---|
| A failure state, not a silently stuck session | The session row turns to the error tint. The status bar reads **`New session · failed · restart`**, where T029 saw `starting…`. |
| The reason is surfaced | The pane and a snackbar both say **"This session's folder no longer exists: /home/jaro/.aaa-vp95/r10. Restore it, or close this session."** |
| Not a crash | The client and daemon stayed up. No process was spawned: the daemon had no child until the restart below. |
| It stays that way | Re-captured 30 s later, still `failed` with the same reason. The snackbar had timed out, as it should. |

`step11-failed-with-reason.png` stacks three strips at 60%:

- the sidebar row and the pane reason, taken 30 s after the press;
- the snackbar and the status bar, taken at the press.

The daemon log for the same press:

```
WARN micold_daemon::state: session spawn refused; not starting session=61085576-… error=session working directory does not exist: /home/jaro/.aaa-vp95/r10
WARN micold_daemon::server: session start failed session=61085576-… err=This session's folder no longer exists: /home/jaro/.aaa-vp95/r10. Restore it, or close this session.
```

These two lines appear twice, 35 ms apart. The first pair comes from `SessionCreate`, the second
from the client's follow-up start of the session it had just been told exists. Neither produced a
process, and neither spent crash-loop budget: the reported lifecycle is `Failed { attempts: 0 }`,
which `session_start.rs` asserts.

### Restoring the folder (beyond the step)

T029 left this out of scope. It was checked here because FR-012 says a later successful start
clears the failure. After `mv r10-moved r10` and a press of the status bar's **restart**, the
daemon logged `session started … mode=AiCli launch=Resume`, and the status bar read `running` with
`claude` open in `/home/jaro/.aaa-vp95/r10`. That is its first-run trust prompt, as expected for a
fresh folder. See `step11-restored-running.png`.

## Observed, not fixed here

- **The failure takes a moment to appear.** Before the spawn is refused, the daemon resolves
  `env_include` in the session's directory, and for a directory that does not exist that runs to
  its timeout, 10 s by default (`env-include resolution did not succeed outcome=TimedOut …`). The start is therefore
  reported failed only after that timeout rather than at once. It is still bounded and it still
  ends in `failed`, so step 11 passes. Checking the directory before resolving `env_include` would
  remove the wait, but that is a separate change in code that open PRs touch.
- **A restored, resumable AI-CLI session is checked for its conversation before its folder.**
  Pressing Start on such a session after its folder is renamed away would report the missing
  conversation, because the transcript lookup is keyed by the directory, and not the missing
  folder. The session still ends `failed` with a reason, just a less precise one. Step 11 asks
  about a new session (**+**), and that path is exercised above.

## What this run did not cover

- **macOS and Windows.** Linux only. The change has no platform-specific code.
- **Unmounting** the root rather than renaming it. It takes the same `ensure_cwd_exists` path.
- **A worktree session whose own directory is renamed away.** It uses the same `start_session`
  path and the same refusal, and T029 observed that it failed identically. It was not re-captured
  here.
