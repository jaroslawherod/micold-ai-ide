# Quickstart: validating one configuration file for every daemon

End-to-end checks for when the issues land. Each issue's own spec-kit run turns its part into tests;
this is the cross-issue acceptance run. Keys: [contracts/daemon-config.md](contracts/daemon-config.md).

## Prerequisites

- A build with every issue in issues.md merged (`mise run build`).
- Podman (container), a kind or other test cluster with a context (pod), an SSH host you can log in
  to, two GitHub fine-grained PATs: `A` reaching two repositories of one owner, `B` reaching a
  personal repository.
- `mise run image` for the `:dev` basic image.

## 1. Schema and `config check` (Story 1, FR-017)

1. `micold-daemon config check specs/daemon-config-file/contracts/examples/container.toml` (the
   contract's §3.3 example saved to a file) → exit 0, resolved values printed, every secret shown as
   `path (missing)` — never a value.
2. Add `listne = 1` under `[daemon]` → exit 2, `…:line:col: unknown key "listne" in [daemon]`.
3. Set `schema = 99` → refused naming 99 and the supported version.

## 2. Every kind starts from its file (Story 1, SC-001)

For each daemon kind, create it in Settings → Daemons and start it:

| Kind | Look for the file at | Then |
|---|---|---|
| Host | `<data_dir>/daemon.toml` | diagnostic → Config shows `kind = host` |
| SSH | `~/.micold/remote/daemon.toml` on the host | same, `kind = ssh` |
| Container | `/run/micold/config/daemon.toml` in the container | `podman inspect` shows no `MICOLD_*` env |
| Pod | ConfigMap `<name>-config` | `kubectl get pod -o yaml` shows no `MICOLD_*` env; Secrets mounted as directories |

Restart the container both ways (app restart over SSH, `podman restart`) → the same diagnostic both
times.

## 3. One token, many repositories (Story 2, SC-002, SC-004)

1. Paste PAT `A` with no filter. Settings lists the repositories it reaches.
2. In a session of each kind, `git clone` both repositories of that owner → both succeed with no
   prompt. `gh pr list` in each → works (SC-003).
3. Add PAT `B` with `repos = ["me/*"]`. In a clone of a `me/` repository, `gh api user` answers as
   `B`'s user.
4. Replace `A`'s file with a new token; within 5 s a `git fetch` uses it (watch the forge log, or
   revoke the old token first).
5. Two tokens with `repos = ["acme/*"]` → the first in Settings is used for `acme/` repositories;
   Settings shows it per repository and marks the second as shadowed. Move the second up → it is used
   by the next `git fetch`.
6. Host daemon, no tokens: `git push` uses the user's own helper exactly as before.

## 4. AI CLIs from the file (Story 4, SC-006)

1. Container daemon: `[[ai.cli]] id = "claude_code"`, `model = "…"`, `settings_file = …` → a new
   session runs Claude Code with that model and `~/.claude/settings.json` equals the file.
2. `enabled = false` for `opencode` → the new-session menu does not offer it.
3. Codex with `api_key_file` → `OPENAI_API_KEY` is set in the session; `ps`/`/proc/*/cmdline` show no
   key. Claude Code with `api_key_file`: replace the key file → the running session's next request
   uses the new key (`apiKeyHelper`).
3a. Host daemon, user signed in to Claude Code on the machine, `sign_in_file` naming another account
   → sessions use the configured account; `sha256sum ~/.claude/.credentials.json` is unchanged
   before and after.
4. `settings_file` on a host daemon → `config check` refuses (V10).
5. Change `default_ai_cli` in the UI → stays in `settings.json`, other windows follow live.

## 5. Start script (Story 5, SC-007)

1. Container daemon, script installs a tool to `~/.local/bin` and `export FOO=bar`, `run = "once_per_container"`.
   Start → a new session finds the tool and `echo $FOO` prints `bar`. `id -u` in the script's log ≠ 0.
2. Restart the daemon → the log shows the script skipped (marker matches). Edit the script → Settings
   shows "Restart to apply"; the script did not run. Restart → it runs.
3. `exit 1`, `on_failure = "refuse"` → the daemon does not start; the app shows the exit status and
   the script's last lines. Same with `on_failure = "warn"` → it starts with a warning.
4. `sleep 999`, `timeout_secs = 2` → treated as failed after 2 s; no orphan `sleep` remains.
5. `env_file` with `SECRET=x`, script without `export SECRET` → a session's `env` has no `SECRET`;
   add `export SECRET` → it does.
6. Set `env_include` in Settings too, both defining `FOO` → the session sees the env-include value.

Pod with a Claude sign-in: a refreshed sign-in is lost when the pod restarts (known limit, documented).

## 6. Projects (FR-014)

Pod with `[[projects]] path = "/data/repos/acme/api", remote = …` and no clone → the daemon clones it
at start and serves it.

## 7. Read-only view (Story 6)

Settings → Daemons → a daemon → "Configuration" shows the delivered file and the resolved values
with reach per token; no secret value appears anywhere (SC-005).

## 8. Gate

`mise run gate` green; `scripts/check-user-guide-updated.sh` satisfied by the user-guide changes.
