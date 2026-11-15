# Plan drift: M4

Paths changed beyond those the tasks of M4 name, with the reason.

- UNPLANNED `crates/micold-core/src/endpoint.rs`: `guard_runtime_dir(dir)` (Unix), the guard
  `resolve_pinned` already applies to `/tmp/micold-<uid>` (create `0700`, or verify owner and mode
  without repairing), for a caller that names the directory. `remote::prepare_link_at` uses it, so
  T029's tests drive a temporary directory instead of the real pinned one, where removing a live
  `orchestrator.sock` would cut the link of a daemon on the machine running the suite.
- UNPLANNED `crates/micold-core/src/provision/failure.rs`: `FailureCause::LinkForward(LinkRefusal)`,
  the cause the start step fails with when the link's forward cannot be opened (FR-005: "fails the
  connection with a message"), with its text and a test of it.
- UNPLANNED `crates/micold-core/src/ssh/status.rs`, `crates/micold-core/src/kube/start_tests.rs`,
  `crates/micold-daemon/tests/sandbox_real_support/mod.rs`,
  `crates/micold-daemon/tests/ssh_real_support/remote.rs`: one line each, `link: None`, at the other
  `ProvisionOptions` construction sites (tests and harnesses) that T034's new field reaches. No
  behaviour changes there.
- UNPLANNED `crates/micold-daemon/tests/ssh_real_support/sshd.rs`: `Sshd::start_with(extra)` appends
  lines to the stand-in server's configuration, so T031 starts it with the basic image's two lines
  and with `AllowStreamLocalForwarding no` (FR-005). `Sshd::start` is `start_with("")`.
- UNPLANNED `crates/micold-core/src/sandbox/basic_image.rs`: `BASIC_IMAGE_CONTENT_HASH` moves with
  T036's edit of `packaging/basic/` (the pin must match the tree: `scripts/tests/basic-image-hash.test.sh`).
  `BASIC_IMAGE`'s digest follows in the image's own pin PR, as for every change of that directory.
- `crates/micold-core/src/ssh/provision.rs` (planned, T034) also maps `FailureCause::LinkForward` to
  `SshFailure::Other` with the refusal's sentence in `ssh_failure_of`, so an SSH host's failure names
  the cause instead of a bare helper line.
- `packaging/basic/sshd_config` is T036's own file (the script reads that task's line without it).
- MISSING `contracts/dial.md`, `research.md`: both are this feature's spec files
  (`specs/1725-daemons-dial-orchestrator/`), named by T029 (the contract the test follows, unchanged)
  and T036 (R1, updated with the `ListenAddress` check).
