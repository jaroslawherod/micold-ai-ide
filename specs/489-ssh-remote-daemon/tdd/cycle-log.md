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
