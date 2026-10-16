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
