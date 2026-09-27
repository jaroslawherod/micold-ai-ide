# Quickstart §B — visual pass record

- **Date:** 2026-09-27
- **Build SHA:** 15f8643b (branch `fix/a-project-local-cli-stays-unreachable-in-that-project`)
- **Binaries:** pinned pair at `/home/jaro/vp/bin-033/` (`micold-ai-ide`, `micold-daemon`), built from this
  worktree; pair verified to connect (`client attached to daemon`, no `refusing client`) before every
  run in this record.
- **Environment:** Xvfb `:91` (1600x1400, software Vulkan via lavapipe/`lvp_icd.json`), private
  `XDG_RUNTIME_DIR=/tmp/vp033rt`, `XDG_DATA_HOME=/home/jaro/.cache/vp033/data`,
  `XDG_CONFIG_HOME=/home/jaro/.cache/vp033/config`, `XDG_STATE_HOME=/home/jaro/.cache/vp033/state`,
  host (non-sandboxed) session placement, `MICOLD_LOG=debug` set explicitly. `HOME` left untouched
  (real `/home/jaro`) per instructions; PATH for the daemon/client themselves was reduced to a scratch
  `fakebin/` (containing only a `sleep 3600` stub named `claude`) plus the plain system directories —
  this was necessary because the *default* env-include script is `~/.bashrc` (feature default, on by
  default) and the real `~/.bashrc` sources mise, which would have put the developer's real
  `pi`/`copilot`/`claude` back on PATH. Prerequisites step (Settings → **Source a script before each
  session**: on, script pointed at the scratch `include.sh`) was completed before step 1, as the
  quickstart's Prerequisites section requires.
- **Fixtures:** scratch `include.sh` puts a fake `pi` (a `sleep 3600` stub) on PATH only when
  `$PWD/.pi-here` exists; git repos `P` (untracked `.pi-here` at root) and `Q` (no marker), both under
  the session scratchpad; worktree `P-wt` (branch `feat/wt`) created in-app with `.pi-here` copied in
  by hand; a second worktree `P-wt2` (branch `fix/wt2`) created in-app with no marker, for step 6.
  Navigation to these deep scratch paths used two short-lived symlinks,
  `/home/jaro/vp033-P` → repo P and `/home/jaro/vp033-Q` → repo Q, only so the in-app folder picker
  (which has no path-typing affordance) could reach them in a few clicks; both were removed at
  cleanup.

## Results

| Step | Verdict | Observation |
|---|---|---|
| 1 | PASS | Opening P with no Settings/list opened first: within ~5s both the Default row and P-wt's row showed the chevron. Opening the Default row's list showed exactly "Claude Code" and "Pi Coding Agent". Picking Pi started a session; the resulting process (`sleep 3600`, our fake `pi`) had cwd `…/repos/P` and PATH including only the scratch `only-in-p/bin`. Screenshot: `step1-list.png`. |
| 2 | PASS | Opening Q: Default row showed only "+" (no chevron). The primary press started a session; the resulting process (`sleep 3600`, our fake `claude`) had cwd `…/repos/Q`, confirmed via `ps`/`/proc`. Screenshots: `step2-q-default.png`, `step2-q-claude-session.png`. |
| 3 | PASS | Returned to P; opened Settings → Environment and closed it (Cancel). Settings' "Default AI CLI" dropdown offered only "Claude Code" (it answers for home, which lacks `pi`). After closing, P's Default row still showed the chevron. Screenshots: `step3-settings-claude-only.png`, `step3-chevron-after-settings.png`. |
| 4 | PASS | Stopped the daemon this run started (`kill <pid>`, not `pkill -f`). The client logged `attach: disconnected` then `attach: connected`; a new daemon process was launched by the supervising client and picked back up the persisted catalog/settings (`catalog adopted load_status=Loaded`). No user action was taken beyond the kill. P's Default row showed the chevron again once the client had reconnected. Log excerpt in `step4-log-excerpt.txt`; screenshot `step4-chevron-after-reconnect.png`. |
| 5 | PASS | Set `default_ai_cli` to `Pi` (Settings' own dropdown only offers CLIs available at home, so this was set by editing the daemon-owned `settings.json` directly before a restart — the daemon reloaded it and confirmed via `catalog adopted load_status=Loaded`). In P, the primary press started a Pi session directly with no list (bottom-right session tag read "pi", confirmed via `ps`/`/proc` cwd = P). In Q, the same press opened the list with a toast: "Pi Coding Agent isn't installed. Install it, or start this session on another AI CLI." Screenshots: `step5-p-pi-direct.png`, `step5-q-list-pi-not-installed.png`. Default AI CLI was reset back to `ClaudeCode` afterwards (same settings.json + restart method) before continuing to step 6, matching the quickstart's parenthetical "(default back to Claude Code)" for step 6. |
| 6 | PASS | Created a second worktree `P-wt2` (branch `fix/wt2`) with no `.pi-here` copied in. Its row showed only "+" and the delete icon — no chevron. Opened P-wt's list (chevron click, showed Claude Code + Pi Coding Agent) and closed it (Escape) without picking anything; P-wt2 still had no chevron afterward. P-wt2's primary press started a `claude` session (default back to Claude Code), confirmed via `ps`/`/proc` cwd = `…/P/.claude/worktrees/fix-wt2`. Daemon log also shows `AI CLI availability reported … available=[ClaudeCode]` for that directory (no Pi, correctly). Screenshots: `step6-wt2-no-chevron.png`, `step6-wt2-claude-session.png`. |
| 7 | PASS | Settings → Environment → turned **Source a script before each session** off, saved: P's Default row lost the chevron (only "+" remained). Turned it back on, saved: the chevron reappeared. Screenshots: `step7-off-no-chevron.png`, `step7-on-chevron-restored.png`. |
| 8 | PASS | Left the window idle for 65s (blocking wait) with `MICOLD_LOG=debug` already active. The daemon's debug log file did not grow by a single line during the wait (151 lines before and after) — no new `AiCliAvailabilityRequest`/"AI CLI availability reported" entries, or any other daemon activity, appeared. Log tail (from just before the wait) in `step8-idle-log-tail.txt`. |

## Review notes (M3 review B)

- **Worktree rows' chevrons (steps 1, 6, 7).** A worktree row's action cluster is revealed on hover,
  and no screenshot hovers a worktree row: the images show the chevron on the Default rows only.
  The worktree-row claims rest on other evidence, not on the images: after each connect the daemon
  log (`step4-log-excerpt.txt`) shows exactly two replies of `available=[ClaudeCode, Pi]`, and `P`
  and `P-wt` are the only directories carrying the marker; the spawned processes' cwd and PATH were
  read from `/proc`; and the M1 tests pin the per-row chevron decision.
- **Step 5's method** deviated as recorded above; `quickstart.md` step 5 now says how to set Pi.
- **The fixture's include script** was `[ -f … ] && export …`, which exits 1 where there is no
  marker, so home, `Q` and `P-wt2` resolved through the env-include failure fallback
  (`outcome=NonZeroExit { code: 1 }`). `pi` is absent either way, so the verdicts hold;
  `quickstart.md` now uses an `if … fi` that exits 0.
- The record names build `15f8643b`; the commits after it on this branch change comments only.

## Coverage note

This pass exercised static appearance, state changes after settings/reconnect/idle, directory-specific
availability answers, and the actual spawned process's cwd/PATH for each case (verified via `ps` and
`/proc/<pid>/{cwd,cmdline,environ}`, not just the UI). It did not attempt to judge animation smoothness
or frame pacing (not applicable to this feature — no animated transition is part of the tested
behavior). All 8 steps show what they name; none were left unrun.
