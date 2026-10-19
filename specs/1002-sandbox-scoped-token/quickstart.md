# Quickstart: validating #1002

## Part A — automated (no GitHub, no container runtime)

| What | Command | Expect |
|---|---|---|
| Core: JWT, request/response, narrowing, schedule, git env, sanitizer, helper, migration, status texts | `mise run test-core` (cloud: `cargo test -p micold-core --all-targets`) | green |
| FR-014 container argv + FR-014 Kubernetes manifest (fake) | same, `sandbox_credentials`, `sandbox_argv`, `kube::start_tests` | no SSH key, agent socket, credential store, `GH_TOKEN` value or App key path in any arg, env, mount or secret |
| FR-015 git over HTTP with the helper | `cargo test -p micold-daemon --test scoped_git_credential` | push and pull of the matching repo succeed; SSH-form remote rewritten; `git credential fill` empty for another repo or host |
| Whole gate | `mise run gate` (cloud: `scripts/gate.sh`) | green |

## Part B — real container runtime (`mise run image && mise run test-sandbox`)

The real-runtime sandbox suite starts a container daemon on a host prepared with `~/.ssh/id_ed25519`,
a running `ssh-agent`, `~/.git-credentials` and `GH_TOKEN`, with a saved profile naming
`ssh_agent` and `git_credentials`, and asserts that `docker inspect` and a `grep -r` of the
container's `/`, `$HOME` and `env` find none of them, and that `/run/micold/git` is a tmpfs.

## Part C — manual, with a real GitHub App (needs eyes and a GitHub account)

1. Register a GitHub App (Settings › Developer settings › GitHub Apps): no webhook; repository
   permissions *Contents: Read and write*, *Metadata: Read*. Generate a private key. Install it on
   one test repository `you/scratch` only.
2. In the client: Settings › GitHub App: enter the App ID, choose the `.pem`. Saved without error.
3. Start a container daemon serving a clone of `you/scratch` (origin as `git@github.com:you/scratch.git`).
   Status shows *Git credential for you/scratch, renews at HH:MM*.
4. In a session: `git commit --allow-empty -m t && git push && git pull` → both succeed;
   `env | grep -i -E 'token|ssh_auth'` → nothing; `ls ~/.ssh` → absent.
5. In the session: `git ls-remote https://github.com/you/other-private-repo` → authentication
   failure (US2-2).
6. `cat /run/micold/git/you/scratch` → token; `curl -s -H "Authorization: Bearer <token>"
   https://api.github.com/repos/you/scratch/hooks` → `403` (no admin permission, SC-003).
7. Uninstall the App from `you/scratch`, restart the daemon → it starts; status *No git credential: The GitHub
   App is not installed on you/scratch…*; `git push` fails at once (US3-2).
8. Remove the GitHub App settings → status *No GitHub App is set up…* (US3-1).
9. Open the settings of a daemon saved before this change with *SSH agent* ticked → the item is
   gone and the one-time notice is shown, with *Dismiss*; dismiss it, reopen → notice gone (US1-3).
10. Keep a daemon running with the client open for over 2 h, push → succeeds (SC-004). With
    *survive logout* on, quit the client for over an hour, reopen it → within a few seconds of the
    reconnect the status shows a fresh expiry and a push succeeds, with no daemon restart (US3-4,
    research R8).
11. Repeat 3–4 with a Kubernetes daemon (kind cluster).
12. Give the App *Pull requests: Read and write*, and on the daemon turn on *Pull requests*: the
    toggle's help text says what it allows; after the next renewal (or a reconnect) a
    `curl -X POST …/repos/you/scratch/pulls` with the token is accepted. Turn it off: after the
    next renewal the same request gets `403`. With the toggle on but the App lacking the
    permission, the status shows *No git credential: GitHub refused a credential…* (FR-011).
