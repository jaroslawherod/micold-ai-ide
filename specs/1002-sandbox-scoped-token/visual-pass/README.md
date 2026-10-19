# Visual pass: quickstart Part C step 9 (feature 1002)

Client built from this branch, run on a private Xvfb with `ICED_BACKEND=tiny-skia`, a private
`XDG_DATA_HOME`/`XDG_RUNTIME_DIR`. Seed settings: one `local_sandbox` daemon whose profile
`credentials` was `["ssh_agent","git_config","git_credentials"]`. Settings → Daemons.

| # | Check | Result | Evidence |
|---|-------|--------|----------|
| 1 | Row shows the `RemovedGitCredentials` message text with a *Dismiss* button; legible, no overlap | PASS | `01-daemon-row-notice-crop.png`, `01-daemon-row-notice.png` |
| 2 | Credential items offer only *Git configuration* and *AI CLI sign-in* | PASS | `02-credential-items.png` |
| 3 | Dismiss removes the notice; it does not return after quit and relaunch | PASS | `03-after-dismiss-crop.png`, `04-after-relaunch.png` (settings.json holds no `notice`) |

Notes: Dismiss persisted at once, without pressing Save. The sandbox itself did not start (no
container runtime here); that banner is unrelated. No visual defect seen.
