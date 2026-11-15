<!-- claude-projects-attribution: {"github_login":"jaroslawherod"} -->
_Requested by **jaro** · [project thread](https://claude.ai/code/project/chan_01RyHdfvmnDGyHasLedRFLeQ?thread=cmsg_01RyHdfvmnDGyHasLedRFLeQVywo8xFvzy2nyDFzeziRa1)_

## Milestone M4 of 5 — daemons dial the orchestrator (specs/1725-daemons-dial-orchestrator)

**Deliverable**: when the app opens its SSH connection with the link request, it opens one reverse forward. The container, SSH-host and pod daemons behind that forward are listed `online` with their own kind, and the forward reaches nothing else.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --features ssh-real-runtime --test ssh_real ssh_real_link -- --test-threads=1` (rootless `sshd`; quickstart §B steps 1, 3, 5); unit tests `scripts/build-lock.sh cargo test -p micold-core argv`
**Tasks**: T028–T037 · **Satisfies**: US2 scenarios 1–4; FR-004, FR-005; SC-001 (four kinds), SC-006

- `argv::LinkForward` checks both socket paths. `Forward` is now an enum with two shapes, the install relay and the link. `bridge_with_link` adds `-o ExitOnForwardFailure=yes -o ClearAllForwardings=no -R <remote>:<local>`. `prepare_link` builds the remote step's command. `admit_link` / `link_config_allows` hold the `ssh -G` rule for user-config hosts.
- `micold-daemon remote prepare-link` creates or verifies the owner-only pinned directory, removes `orchestrator.sock` and prints its path.
- `ProvisionOptions.link: Option<LinkRequest>`. `start_step` runs `ssh -G` (user-config reach only), then `prepare-link`, then the bridge with the forward. Every refusal is `FailureCause::LinkForward`, including ssh's `remote port forwarding failed`.
- The client sets `link` at its three construction sites (Unix clients only).
- `packaging/basic/sshd_config` gains `AllowStreamLocalForwarding remote` and `StreamLocalBindUnlink yes`. The content-hash pin moved with it; the published image digest follows in the image pin PR.
- **Behaviour change to check**: an SSH host whose own configuration sets a `LocalForward`, `RemoteForward` or `DynamicForward` now fails to connect through the app. A host whose `ssh -G` fails also fails. In both cases the message names the cause (tasks.md T028). research.md R2 originally said "connect without the link", and review A F2 flagged the difference. The ledger records the decision under *Decisions*. Reverting it is one `match` in `provision::admit_link`.

## Agent review
- Review A (code-review, high, opus, headless): CHANGES with 2 MAJOR and 2 MINOR. Three were fixed and one declined:
  - F1: the forward is now kept when the user's configuration sets `ClearAllForwardings yes`. The link's argv passes `ClearAllForwardings=no`.
  - F2: research R2 was updated to match T028 (a prose-only fix).
  - F3: the scenario-3 test was renamed and now asserts the `Prepare` refusal.
  - F4: declined. The second `ssh -G` only reads files.
- Round 2 on the F1 fix and review B (conformance, sonnet): run by the orchestrator.

## Local gate
`scripts/gate.sh` (CI's order) at 4c34d54: fmt, clippy (core, workspace) and `cargo nextest` (13214 tests) green. `scripts/tests/autopilot.test.sh` failed in this sandbox with a different set of `facts:` cases on each run (1 case, then 7). Their output shows git's own `push negotiation failed` against the test's local remote. No script changed in this diff, so CI validates that part. `ssh_real_link`: 4 passed against a real rootless `sshd` (installed in this session). A mutant that ignores `opts.link` makes all 4 fail. In the whole `ssh_real` suite, `ssh_real_an_unreachable_host_fails_within_the_connection_timeout` fails in this cloud sandbox (`ConnectionRefused` for the unroutable address). That test opens no link. No container runtime was available: `mise run image` / `test-sandbox` were not run (sshd_config changed), so CI's sandbox job covers it. Visual pass: not applicable (nothing visible).

Refs #1725

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvKyGYTBua2wMPbNp6R
