# Quickstart: validating #1002

## Part A — automated (no GitHub, no container runtime)

| What | Command | Expect |
|---|---|---|
| Core: PAT checks, stored file, schedule (`AtExpiry`, `refresh`, 24 h warning), git env, sanitizer, helper, migration, status texts | `mise run test-core` (cloud: `cargo test -p micold-core --all-targets`) | green |
| FR-014 container argv + FR-014 Kubernetes manifest (fake) | same, `sandbox_credentials`, `sandbox_argv`, `kube::start_tests` | no SSH key, agent socket, credential store, `GH_TOKEN` value or stored PAT path in any arg, env, mount or secret |
| FR-015 git over HTTP with the helper | `cargo test -p micold-daemon --test scoped_git_credential` | push and pull of the matching repo succeed; SSH-form remote rewritten; `git credential fill` empty for another repo or host |
| Whole gate | `mise run gate` (cloud: `scripts/gate.sh`) | green |

## Part B — real container runtime (`mise run image && mise run test-sandbox`)

The real-runtime sandbox suite starts a container daemon on a host prepared with `~/.ssh/id_ed25519`,
a running `ssh-agent`, `~/.git-credentials` and `GH_TOKEN`, with a saved profile naming
`ssh_agent` and `git_credentials`, and asserts that `docker inspect` and a `grep -r` of the
container's `/`, `$HOME` and `env` find none of them, and that `/run/micold/git` is a tmpfs.

## Part C — manual, with a real fine-grained PAT (needs eyes and a GitHub account)

1. On GitHub: Settings › Developer settings › Personal access tokens › Fine-grained tokens › generate:
   *Only select repositories*: `you/scratch` alone; permissions *Contents: Read and write*,
   *Metadata: Read*; expiry 1 day (the minimum).
2. In the client: Settings › GitHub access tokens: add `you/scratch` and paste the token. It is saved
   and the row shows the expiry date; the field never shows the token again. A classic `ghp_` token,
   a token for another repository, and (if the org allows) a token with no expiry are each refused
   with their reason and nothing is stored (FR-009).
3. Confirm the API facts the checks rely on (research R2): the `github-authentication-token-expiration`
   header and `permissions.push` are on `GET /repos/you/scratch`, `404` is what a PAT not selecting a
   repository gets, and, with a token that also selects a second private
   repository, the *reach* check refuses it. Record the outcome here (close unit, T056); if any fails, apply the
   fallback of contracts/pat-source.md §2 and say so in the settings text.
4. Start a container daemon serving a clone of `you/scratch` (origin as `git@github.com:you/scratch.git`).
   Status shows *Git credential for you/scratch, expires on <date>*.
5. In a session: `git commit --allow-empty -m t && git push && git pull` → both succeed;
   `env | grep -i -E 'token|ssh_auth'` → nothing; `ls ~/.ssh` → absent.
6. `git ls-remote https://github.com/you/other-private-repo` → authentication failure (US2-2).
7. `cat /run/micold/git/you/scratch` → the token; `curl -s -H "Authorization: Bearer <token>"
   https://api.github.com/repos/you/scratch/hooks` → `403` (no admin permission, SC-003).
8. Remove the PAT in Settings → the running daemon's status *No git credential: No GitHub token is
   stored…*, `git push` fails (US3-1, FR-010); the settings say the token is still valid on GitHub.
   Paste it again → pushing works again without restarting the daemon (US2-5).
9. Open the settings of a daemon saved before this change with *SSH agent* ticked → the item is
   gone and the one-time notice is shown, with *Dismiss*; dismiss it, reopen → notice gone (US1-3).
10. With the 1-day token the settings row and the daemon status show the expiry warning from the
    day it was pasted (it is within 24 h of expiry from the start, FR-008, SC-004); a token made with a
    longer expiry shows none until 24 h before. A core test with an injected clock covers the boundary. After the expiry, the container's file is gone, the status says
    *The GitHub token for you/scratch expired…*, `git push` fails, and pasting a new token restores
    pushing in the same daemon (US3-4).
11. Repeat 4–5 with a Kubernetes daemon (kind cluster).
12. Settings text check (FR-011): at the paste form the help names *Contents: Read and write* and
    *Metadata: Read*, says the sandbox can use everything the token can, that the application cannot
    narrow it, that a leaked token stays valid until it expires or is deleted on GitHub, and that
    *Remove* does not delete it there. A token also given *Pull requests: Read and write* lets the
    sandbox open a pull request, as the text says.

### Part C results (#1147, 2026-10-27)

No real fine-grained PAT was made (user decision 2026-10-27), and the app has no base-URL override:
`https://api.github.com` and `github.com` are fixed in `scoped_credential/pat.rs` and the daemon
helper, so a local fake GitHub cannot be pointed at without product code (out of scope for a chore;
follow-up filed). So **no step was run live.** What stands in for each, by an automated test CI
already runs (`mise run gate`):

| Step | Result | Covered by (not live) |
|---|---|---|
| 1 | NOT RUN: no real PAT | |
| 2 | NOT RUN live | `micold-core` `scoped_credential::pat` shape/expiry tests; client `shell::scoped_credential` `a_refused_token_is_not_stored_and_an_accepted_one_is`; `features_settings` `a_token_that_is_not_fine_grained_is_refused_before_any_check` |
| 3 | NOT RUN: real API facts (R2) unverified | only the fake-transport tests: `the_expiry_header_parses_to_unix_seconds`, `the_probe_needs_push_and_a_future_expiry`, `statuses_map_to_their_reasons` (403/404), `the_reach_listing_refuses_another_private_repository_naming_it` |
| 4, 5 | NOT RUN live (container) | client `a_stored_token_is_granted_for_its_own_repository_only`; daemon `--test mcp scoped_git_credential` (git push/pull over HTTP with the helper against a local git HTTP backend) |
| 6, 7 | NOT RUN live | same daemon test (helper empty for another repo/host); `sandbox_real_no_user_credentials_reach_the_container` (`mise run test-sandbox`, CI job `sandbox-runtime`) for the tmpfs credential directory |
| 8 | NOT RUN live | client `a_removed_token_is_withdrawn_from_the_daemon`, `a_replaced_token_is_granted_without_a_daemon_restart`, `a_token_removed_while_disconnected_is_revoked_on_the_reconnect` |
| 9 | NOT RUN live | `features_daemons` `dismissing_the_notice_clears_it_from_the_registry_and_the_row` |
| 10 | NOT RUN live | client `at_the_expiry_the_status_is_expired_and_the_daemon_is_told_to_delete_the_token`, `a_held_token_is_reported_again_when_it_comes_within_a_day_of_its_expiry`; core `schedule` tests (injected clock) |
| 11 | NOT RUN (kind) | `kube::start_tests` (fake) only |
| 12 | NOT RUN live | `micold-client` `features::settings::github_tokens` tests |

The live check against real GitHub and the visual pass of the status rows (T055) remain open on #1147.
