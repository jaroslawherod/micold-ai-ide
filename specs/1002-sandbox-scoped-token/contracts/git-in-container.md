# Contract: git inside a sandbox (#1002, FR-004, FR-006, FR-007)

## 1. Environment block

`scoped_credential::git_env(base: &str) -> BTreeMap<String, String>`, `base` = `https://github.com`
in production. Merged into the container's `-e` list (`sandbox/argv.rs`) and into
`kube::credentials::daemon_env`. Sessions inherit the daemon's environment.

```text
GIT_CONFIG_COUNT=5
GIT_CONFIG_KEY_0=credential.helper                    GIT_CONFIG_VALUE_0=
GIT_CONFIG_KEY_1=credential.<base>.helper             GIT_CONFIG_VALUE_1=!micold-daemon git-credential
GIT_CONFIG_KEY_2=credential.<base>.useHttpPath        GIT_CONFIG_VALUE_2=true
GIT_CONFIG_KEY_3=url.<base>/.insteadOf                GIT_CONFIG_VALUE_3=git@github.com:
GIT_CONFIG_KEY_4=url.<base>/.insteadOf                GIT_CONFIG_VALUE_4=ssh://git@github.com/
GIT_TERMINAL_PROMPT=0
```

`GIT_TERMINAL_PROMPT=0` so a missing credential fails at once instead of waiting on a prompt no one
sees (US3). No `GIT_SSH_COMMAND`, no `SSH_AUTH_SOCK`.

## 2. `micold-daemon git-credential <get|store|erase>`

Git's credential-helper protocol on stdin/stdout.

| Input (`get`) | Output |
|---|---|
| `protocol=https`, `host=github.com`, `path=o/r` or `o/r.git`, file `/run/micold/git/o/r` exists, `expires_at > now` | `username=x-access-token\npassword=<token>\n` |
| anything else (other host, other repo, no path, expired, no file, unreadable file) | nothing; exit 0 |

`store`, `erase`: read and discard stdin, exit 0. Owner and repository compared case-insensitively;
`path` is normalised (trailing `.git`, trailing `/`). The root path of the file store is a constant
in core; the test overrides it through the daemon's existing test-root mechanism.

## 3. Sanitized git configuration (*Git configuration* opt-in)

`scoped_credential::sanitize_gitconfig(entries: &[(key, value)]) -> String` over the output of
`git config --file ~/.gitconfig --list --null`. Dropped keys (case-insensitive section and variable,
subsection as written):

- `credential.*` and `credential.<url>.*`
- `url.<base>.insteadOf` / `url.<base>.pushInsteadOf` whose `<base>` contains `@` before the host
  (userinfo)
- `http.extraHeader`, `http.<url>.extraHeader`
- `include.*`, `includeIf.*`
- `core.sshCommand`, `core.askPass`, `sendemail.smtpPass`, `imap.pass`, `http.proxyAuthMethod`
- any setting whose value is a URL with userinfo (`remote.<n>.url`, `http.proxy`, ...)

Everything else is written back in order, as `git config` would read it. The result is written by
the client to `<daemon state dir>/gitconfig` (container; mounted where `~/.gitconfig` was) or into
the git-config secret (Kubernetes), replacing the verbatim copy.

## 4. What is never present (FR-001, FR-014)

In the container argv / pod manifest, environment, mounts, secrets and files: the host's
`SSH_AUTH_SOCK` path, any `~/.ssh/*` path or content, `~/.git-credentials` path or content,
`GH_TOKEN` / `GITHUB_TOKEN` values, `gh`'s `hosts.yml`, and the App private key or its path.
