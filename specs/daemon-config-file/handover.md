# Handover: unified daemon configuration file — PLANNING ONLY

You are in worktree `docs-daemon-config-file-plan` (branch `docs/daemon-config-file-plan`).
This feature is **planning only**: speckit artifacts that become GitHub issues later. Do not change
any source file (`crates/`, `packaging/`, `scripts/`). Only `specs/daemon-config-file/**` (and
`.specify/feature.json` pointing at it).

## Your steps

1. Run speckit for planning only. Tell each speckit step in its arguments: "PLANNING ONLY — the
   output is a plan split into GitHub issues; no implementation, no source changes, no TDD runs."
   - `/speckit-specify` → `specs/daemon-config-file/spec.md` (a draft is at the end of this file:
     use it as the starting point; keep the Clarifications section as is).
   - Skip `/speckit-clarify`: the user already answered (see Decisions). Record any further open
     question in the PR body instead of asking.
   - `/speckit-plan` → `plan.md`, `research.md`, `data-model.md`,
     `contracts/daemon-config.md` (the TOML schema with every key, type, default, which kinds use
     it, and one full example file per daemon kind: host, ssh, container, kubernetes),
     `quickstart.md`.
   - `/speckit-tasks` → `tasks.md`, grouped so that **each group is one future GitHub issue**
     (proposed split below). Add `issues.md`: one section per issue with title, labels
     (`flow:feature`/`flow:bug`/`flow:chore`, `effort:*`), body, dependencies, and the tasks it
     covers.
2. Verify every code claim below with `LSP`/`grep -n` before you write it into research.md; the
   findings came from Haiku explore agents and two are unconfirmed (marked).
3. Commit (`docs(daemon-config): …`), push, open a **draft** PR against `main` titled
   "docs(daemon-config): plan — one configuration file for every daemon". The PR body: summary, the
   schema (paste the main example), the decisions, the issue split, open questions. Use the
   `docs-not-needed` label only if CI requires one for a specs-only change.
4. Do NOT create the GitHub issues. The user reviews the draft PR first; issues are created after
   approval, then the PR is marked ready and merged.
5. Report: PR URL, the schema example, open questions.

## Decisions (user, 2026-10-27)

- Format: **TOML** (serde + `toml`; comments; `serde_yaml` unmaintained; JSON no comments).
- **The application generates** each daemon's file from Settings and delivers it. A hand-written file
  for an unmanaged daemon must also work.
- **Secrets are referenced by file path, never inline.** The config is safe to print/diff/log.
- **A PAT is used for every repository it reaches.** Drop #1002's one-repo reach refusal. The daemon
  asks GitHub which repos a token reaches (`GET /user/repos`, paginated; cache the answer). Optional
  filter `repos = ["acme/*", "other/x"]`; the most specific match wins; equal specificity → refuse
  at load.
- **Every daemon kind alike**: host process and the user's SSH host also take PATs/AI sign-ins from
  the file. With nothing configured they fall back to that host's own git and `gh` credentials.
  Sandboxes (container, pod) keep clearing inherited credential helpers (#1002 FR-004).
- **Live reload**: daemon watches the config file and the secret files; a changed token is used by
  the next git/`gh` command without restart (today's behaviour with `GitCredentialGrant`).
- `settings.json` (UI-edited, daemon-owned live settings) stays; the new file is deployment config.
- `GH_TOKEN` for sessions: resolved per repository at the moment `gh` runs (a wrapper or
  `gh`-reading-a-file approach; decide in research.md; `gh` has no credential-helper mode, so
  likely a `gh` shim on the session PATH that sets `GH_TOKEN` from the daemon's resolver, or
  `micold-daemon gh-token <cwd>`).

## Findings: how configuration reaches a daemon today

| | Host process | SSH host | Container | Kubernetes pod |
|---|---|---|---|---|
| Start | `spawn.rs:73-85`, no env/args | `ssh … micold-daemon remote bridge` (`ssh/argv.rs:320-346`), env `[]` for user hosts (`provision/mod.rs:142-153`) | `podman create -e …` (`sandbox/argv.rs:168-205`) — reaches only the daemon `packaging/basic/entrypoint.sh:64-69` starts; SSH bridge passes another list (`micold-client/src/daemon.rs:1283 container_env`) | bridge env from `kube/credentials.rs:152-171 daemon_env` (`MICOLD_IDLE_STOP=off`, `MICOLD_REPOS_ROOT=/data/repos`, `git_env_with_helper`); pod env `HOME, TMPDIR, MICOLD_SSH_DIR` (`kube/manifest.rs:183-187`) |
| Client auth | socket permissions (`endpoint.rs:199-222`) | user's ssh | token file `MICOLD_TOKEN_PATH` (`protocol/auth.rs:31-35`) + daemon SSH keys (`provision/keys.rs`) | access Secret + SSH keys |
| GitHub PAT | refused (`git_credential.rs:40-47 credential_root` is None) | refused | `GitCredentialGrant` (`messages.rs:484`) → in-memory `/run/micold/git/<owner>/<repo>` | `GitCredentialGrant`, **probably refused**: pod sets neither `MICOLD_IMAGE_REFERENCE` nor `MICOLD_TOKEN_PATH` (UNCONFIRMED at runtime) |
| AI sign-in | user's home | whatever the host has | writable bind mount (`sandbox/mod.rs:700-735`) | read-only Secret subPath (`kube/credentials.rs:73-79`) — subPath never updates |
| Repos | `ProjectAdd` | `ProjectAdd`, no clone root | bind mounts at same path + `ProjectAdd` | `RepoClone` on demand under `MICOLD_REPOS_ROOT` (`repo_clone.rs:15`) |

Env vars the daemon reads today: `MICOLD_DATA_DIR` (`paths.rs:20`), `MICOLD_HOME`,
`MICOLD_IMAGE_REFERENCE` (`daemon/state.rs:772,794`), `MICOLD_TOKEN_PATH`, `MICOLD_LISTEN_ADDR`
(`server.rs:55`), `MICOLD_IDLE_STOP` (`spawn.rs:37`), `MICOLD_REPOS_ROOT`, `MICOLD_SSH_DIR`,
`MICOLD_LOG`, `MICOLD_GIT_CREDENTIAL_ROOT`/`_BASE` (test overrides), `MICOLD_STATUS_RELAY`,
`MICOLD_PI_ACTIVITY_LOG`, plus `HOME`, `TZ`.

PAT store today: `<secrets_dir>/github-pat/<owner>/<repo>` (`scoped_credential/pat_store.rs`),
check in `scoped_credential/pat.rs:223-247` (reach check `parse_reach` :191-219), grant scheduling in
`micold-client/src/shell/scoped_credential.rs` and `scoped_credential/schedule.rs`.

Two bugs found (each its own issue, independent of the epic, fix first):
- **Pod daemon refuses GitHub token grants**: `credential_root()` needs `MICOLD_IMAGE_REFERENCE` or
  `MICOLD_TOKEN_PATH`; the pod sets neither (verify).
- **Container daemon started over SSH lacks the git credential environment and `MICOLD_HOME`**:
  `container_env` (`micold-client/src/daemon.rs:1283`) omits `git_env`; and `git_env` names a bare
  `micold-daemon` helper, which is not on PATH since the daemon lives at
  `~/.micold/remote/<build>/micold-daemon` (verify; pods use `git_env_with_helper` with a full path).

## Draft schema (refine in contracts/daemon-config.md)

```toml
# Written by Micold AI IDE for daemon "podman-sandbox". Read by micold-daemon at start and on change.
# Secrets are never in this file: every *_file names a file that holds one.
schema = 1

[daemon]
name = "podman-sandbox"
kind = "container"                 # host | ssh | container | kubernetes — reported, not branched on
data_dir = "/var/lib/micold-ai-ide"
home = "/home/micold"              # HOME for the daemon and its sessions
time_zone = "Europe/Warsaw"        # optional
image = "ghcr.io/jaroslawherod/micold-basic:5d9ed63715e43146"   # optional
idle_stop = "default"              # "default" | "off" | seconds
log = "info"

[listen]
socket = "auto"                    # host: the per-user endpoint; or tcp = "0.0.0.0:7000"

[auth]
client_token_file = "/run/micold/token"   # optional: absent = endpoint/SSH auth only

[ssh]
dir = "/run/micold/ssh"            # host_ed25519_key(.pub), authorized_keys

[storage]
clone_root = "/data/repos"         # optional: absent = this daemon clones nothing

[[projects]]
path = "/home/jaro/workspaces/micold-ai-ide"
remote = "https://github.com/jaroslawherod/micold-ai-ide"   # optional: clone when absent

[git]
isolate_host_helpers = true        # sandboxes: clear inherited credential helpers
ssh_to_https = true                # rewrite git@github.com: to https for the token

[[github.tokens]]
name = "acme-org"
token_file = "/run/micold/secrets/github/acme-org"
repos = ["acme/*"]                 # optional filter; default = every repo the token reaches

[[github.tokens]]
name = "personal"
token_file = "/run/micold/secrets/github/personal"

[[ai.sign_in]]
cli = "claude_code"
file = "/run/micold/secrets/ai/claude_code/.credentials.json"
writable = true                    # the CLI refreshes it

[session.env]
RUST_LOG = "info"

[session.secret_env]
OPENAI_API_KEY = "/run/micold/secrets/env/OPENAI_API_KEY"   # value = file path
```

Default config locations (no env needed; `--config` overrides):
host `<data_dir>/daemon.toml`; SSH host `~/.micold/remote/daemon.toml`; container and pod
`/run/micold/config/daemon.toml`. Secrets: host — the files where they already are (PAT store,
`~/.claude/.credentials.json`); SSH — copied to `~/.micold/remote/secrets/` (0700/0600); container —
host secrets dir bind-mounted at `/run/micold/secrets`; pod — Secrets mounted as directories (not
subPath, so updates propagate), config as a ConfigMap.

## Proposed issue split (refine in tasks.md / issues.md)

0. Epic: one configuration file for every daemon (links all below).
1. bug: pod daemon refuses GitHub token grants. (independent)
2. bug: container daemon started over SSH lacks git credential env / helper path. (independent)
3. feat: config schema in micold-core — types, TOML parse, validation, schema version,
   `micold-daemon config check`; contract doc. (foundation)
4. feat: daemon loads the config (`--config` + default locations), replaces env reads; watches and
   reloads. (needs 3)
5. feat: client renders a daemon's config from Settings — pure function, golden files per kind. (3)
6. feat: delivery for host process and SSH host. (4, 5)
7. feat: delivery for containers — config + secrets mounts; drop `-e` list and bridge env. (4, 5)
8. feat: delivery for Kubernetes — ConfigMap + directory Secret mounts; drop bridge env. (4, 5)
9. feat: multi-repository GitHub tokens — PAT store without repo, optional filter, Settings UI shows
   reach; drop the one-repo refusal. (5)
10. feat: daemon credential helper reads tokens from config, reach discovery + matching, offline
    cache; retire `GitCredentialGrant/Revoke`. (4, 9)
11. feat: `GH_TOKEN` for `gh` in sessions of every kind. (10)
12. feat: AI CLI sign-ins and session env from config for every kind. (6, 7, 8)
13. feat: projects and clone root from config. (4)
14. chore: retire the old `MICOLD_*` start-up variables, update user guide
    (`docs/user-guide/sandboxed-daemon.md`, settings.md). (all)

## Spec draft (starting point for spec.md)

(see /tmp/micold-daemon-config-spec-draft.md)
