# TDD cycle log: 489 ssh-remote-daemon

## M1

### Admitted deviation: test-after for the pure `ssh` modules

The first M1 unit wrote `ssh/target.rs`, `argv.rs`, `classify.rs` and `stdio.rs` together with their
tests, and the second wrote `provision.rs` and `tests/daemons_ssh.rs` the same way, without a
recorded red run first. They are pure functions with the cases enumerated in the contracts. In place
of red-first, each test's strength was checked by hand-applied mutants on the finished code (a mutant
must make a test fail), recorded below. Behaviours I6, I7 and the client reducers follow the order of
`tasks.md`.

### Mutant checks (core/ssh)

| Mutant | Result |
|---|---|
| `StrictHostKeyChecking=yes` to `no` in `argv.rs` | killed |
| `HostKeyUnknown` removed from `is_permanent` | killed |
| port `0` check moved to `1` in `SshTarget::new` | killed |
| copy even when `have=1` in `provision` | killed |
| `select_artefact` ignores the platform match | killed |

### Behaviours outside core/ssh: test-after deviation (Constitution I)

M1 was written across four units, three of which could not compile or run anything (the repo-wide
build lock was held by other worktrees for whole sessions). The remaining behaviours therefore have
no recorded red run: `remote bridge` / `prune` (I6), the `Dial::Ssh` actor (I7), the Daemons-form
reducers (T020), the geometry case (T021) and `ssh_real_endtoend` part 1 (O1, O2). Each was
written together with its test and first executed green; no failing output was captured, and none
is claimed. What backs them instead:

- the tests assert the contract's observable (Starting with copy progress, then Connected with
  version and `user@host:port`; `auto_start` applying to an `Ssh` entry; an Unreachable `Ssh` entry
  not dialled; bridge relaying and exiting on a daemon-side close);
- review A (code-review, high) and review B read each test against the behaviour;
- the real-sshd end-to-end test runs the unmodified production path (`provision::connect`).

Follow-up if the milestone's strength is to be audited: apply the mutants "drop `auto_start` for
`Ssh`", "dial an Unreachable `Ssh` entry" and "bridge skips the daemon start" and record that each
test fails (`speckit-tdd-verify`).

## M2 (T028-T038): test-after deviation

The M2 behaviours (remote identity encode/decode in `ssh::remote_path`, boundary translation of
daemon messages, the repository step, remote worktree location display, add-repository over a
connected daemon, the real-sshd project/worktree/session end-to-end) were written together with
their tests under a starved build lock and first executed green. No red run was captured and none
is claimed. What backs them: tests assert the observables (identity round trip and never shown in
the UI; `RemoteRepositoryRequested` only when connected; geometry of the repository field;
worktree and shell output through a real sshd), and review A and B read each against the
behaviour. Mutants to apply if audited: "wrap_path skips `cwd`", "canonicalize_best_effort touches
the local filesystem for a remote identity", "add_repository ignores a non-connected daemon".

## M3 (T039-T050): test-after deviation

The M3 behaviours (`Backoff` sequence and reset, the `Reconnecting`/`GiveUp`/`Reconnect` state
machine, the client reconnect loop with an injected sleep, the disconnected/lost labels, the
Reconnecting badge and its actions, the fault proxy and the real-sshd cut-and-restore test) were
written together with their tests; no red run was captured and none is claimed. What backs them:
the tests assert observables (the delays 1, 2, 4, 8, 15; the state after each event; the epoch
bump; the label strings; the badge geometry; over a real `ssh` the same daemon pid, an unbroken
tick sequence, recovery within 30 s, stop trying then Reconnect), and review A and B read each
against the behaviour. Mutants to apply if audited: "Backoff never caps", "GiveUp leaves
`should_dial` true", "Reconnect does not bump the epoch", "an SSH drop after `established` is not
retried".

## M4 (T051-T055): test-after deviation

Deviation: the isolation and launch behaviour already existed (one iced subscription actor per
daemon, `DaemonStates::at_launch` a synchronous registry fold, per-id `DaemonLinks`), so T051, T052
and T053 passed on first run; no red phase was possible. T054 needed no code change. T052's core
half lives in the integration file `crates/micold-core/tests/daemons_ssh.rs`.

Mutants each test would fail: a remote event that writes to the local link or outbox (T051); an
`at_launch` that marks a lazy remote `Starting` or an eager one `Stopped` (T052); a daemon that
serves a second bridge from a different process, delivers input to every session, or drops the
other client when one leaves; a cut remote link that stalls a local session (T053).

## M5 (T056-T065): test-after, red proven by mutation

The M5 behaviours (the `~/.ssh/config` alias list and the `ssh -G` parser, `SshDuplicate{other}`
over the resolved (host, user, port), the host picker and its widget, the recent-errors list, the
`ConnectTimeout` watchdog, the real-sshd host-key and auth failures) were written together with
their tests by an interrupted unit; no red run was captured and none is claimed. What backs them:
the tests assert observables (alias list contents, parsed effective host/user/port/timeout, the
duplicate message naming the other daemon, picker geometry, the failure reason strings over a real
sshd with no key recorded), and review A and B read each against the behaviour. Mutants to apply
if audited: "parse_g takes the first of repeated keys instead of the effective one", "duplicate
check compares the alias instead of the resolved host", "watchdog ignores ConnectTimeout".

Red pass after the fact (mutate production, run, restore): removing the user/port/key clearing in
`SshHostChosen` fails `choosing_a_host_from_the_picker_fills_only_the_host_and_clears_the_other_fields`;
making the registry's duplicate comparison never match fails `a_second_daemon_for_the_same_host_user_and_port_is_refused_naming_the_first`,
`editing_a_daemon_to_its_own_target_is_not_a_duplicate_but_to_another_is`,
`two_aliases_of_one_machine_are_refused_when_ssh_resolves_them_alike` (core) and
`a_second_alias_of_the_same_machine_is_refused_naming_the_first_daemon` (client). Both restored; the
tests pass on the real code. The `parse_g` and `ConnectTimeout` mutants stay unproven (the latter is
covered only by the off-by-default real-sshd tests).

## M6 (T066-T072): version mismatch

`ssh/mismatch.rs` was test-first: the 11 tests were run against `todo!()` stubs (all 11 failed on the
panic), then the rules were written (11 pass). The fixture guard test
`a_default_build_carries_the_cargo_package_version_whatever_the_fixture_variable` passes with
`MICOLD_FIXTURE_VERSION_SUFFIX=-old` in the environment and no feature, and the feature-on variant
passes with it set. The client tests (`ssh_version_mismatch.rs`) name `DaemonView.explanation`,
which did not exist before the change. `ssh_real_mismatch.rs` was run once red (the daemon's build
string still read `CARGO_PKG_VERSION`, so the suffix never reached the explanation), then green after
`daemon_build()` used `PACKAGE_VERSION` in both places it was built.

Limit: the stand-in daemon refuses this build's client, so a session cannot be started on it to prove
"running sessions untouched"; the real-ssh test shows the refusal ends no process (same pid, a second
dial is refused alike) and the client tests show the route sends nothing.

## M7 (T073-T081): replace a mismatched daemon with no sessions

Deviation (admitted): the tests and the implementation of `ssh/status.rs`, `runtime_status.rs`,
`remote status | stop`, the client Replace action and the real-ssh replace cases landed together in
WIP commits, so there is no red-before-green run in the history. Red pass after the fact (mutate
production, run, restore): changing `decide_stop`'s `running_sessions == 0` to `>= 0` fails
`remote::tests::only_a_status_proving_no_sessions_ends_the_daemon`; restored, it passes.
`mise run test-ssh` passes both replace cases (offered with no sessions then connects; withheld while
a session runs, on a matching-version daemon since a refused client cannot start sessions on a
mismatched one). Other mutants (unknown `status_version`, newer daemon never offered) are covered by
tests in `ssh/status.rs` and `ssh_replace_offer.rs` but not mutation-proven.

Limit: `status.json` is refreshed every 200 ms, so a session started in that window before a stop
could be missed (reviewer MINOR, accepted and documented in the user guide).

## M9 (T085-T088)

Deviation (admitted): test and implementation landed together, no red run in history. Red pass after
the fact: removing `note_lost_sessions` from `on_connected`'s path (test calls it directly) makes
`a_session_the_restarted_daemon_lacks_shows_lost_to_daemon_on_its_row` fail on `lost to daemon Box`
(the session is dropped); removing `self.epoch += 1` at the `(_, Reconnect)` arm of `daemons.rs`
now fails `an_explicit_reconnect_after_giving_up_starts_again` (the mutant of verification.md
finding 2 is killed). Known limit: a lost session keeps its last lifecycle in other views; only the
worktree row says `lost to daemon`.
