# US1, again — the boundary as a user meets it

**Date**: 2026-08-26 · **Runtime**: Docker 29.5.1, cgroup v2, x86_64 Linux
**Image**: `micold-daemon:dev`, built from this working tree
**Covers**: quickstart.md §B.2 (the boundary, tested adversarially)
**Test**: `crates/micold-daemon/tests/sandbox_real_boundary.rs`, behind `sandbox-real-runtime`

## Why this exists when `us1-isolation.md` already probed the same boundary

That pass used `docker exec` with the entrypoint replaced by a shell. It answered: *what can **a**
shell in **that** container reach?* — a question about the argv `sandbox::argv::create` produces.

What a user meets is different in three ways, and each was capable of being wrong on its own:

- the shell is one the **daemon** spawned, with the daemon's environment, not one `docker exec`
  attached with the caller's;
- it runs in the container the **application** created and the daemon adopted, not one the probe
  built;
- it is reached over the **control channel**, so anything the daemon does to input or output on the
  way (holding it during a start, replaying it, dropping it as stale) is inside the measurement.

So this is the same boundary asked as the product question. It is also now a test rather than a
transcript: it re-runs on every `sandbox-runtime` CI job instead of being true once, in August.

## How it runs

`docker create` with the flags `argv::create` produces, the real `micold-daemon` binary Cargo built
as the entrypoint, a temporary directory as a registered project, and the state directory —
`<data home>/micold-ai-ide`, one level **below** the data home, because the image sets
`XDG_DATA_HOME=/var/lib` — bind-mounted so the host can seed the catalogue and read the daemon's
log. The test then connects as the client does, starts a session, and drives its PTY: each probe is
typed with `\r`, terminated by a split sentinel, and its output read back by diffing the reassembled
screen against the snapshot taken before the command.

## §B.2 — every box, from inside a session

| Check | Probe | Result |
|---|---|---|
| Project present at its **host absolute path** (R2) | `cat <project>/marker.txt` | the marker, through the identical path |
| The session starts *in* the project | `pwd` | `<project>` |
| `ls /` is the container's root | `ls /` | `usr`, `etc` present |
| The project's own parent | `ls <tempdir>` | exactly `project` — the mount, nothing else |
| The host home directory | `ls $HOME` | `No such file or directory` |
| An unregistered sibling directory | `ls <tempdir>/unregistered` | `No such file or directory` |
| A file inside it | `cat <tempdir>/unregistered/secret.txt` | `No such file or directory` |
| The runtime's control socket (C-3) | `ls -l /var/run/docker.sock` | `No such file or directory` |
| A git identity, no opt-in (FR-004a) | `cat $HOME/.gitconfig` | `No such file or directory` |
| The AI CLI's auth directory, no opt-in (FR-004a) | `ls $HOME/.claude` | `No such file or directory` |
| A forwarded ssh agent (FR-004a) | `ssh-add -l` | no `SHA256:` fingerprint |
| Ownership (R3, C-4) | `touch <project>/written-from-inside.txt` | host `stat` shows the user's uid:gid; host rewrites and removes it |

Every absence probe also asserts the project's marker string does not appear in its output, so a
probe that silently read the wrong path cannot pass by returning something plausible.

## Two things that read like leaks and are not

- **`ls <tempdir>` lists `project`.** A bind mount's parent must exist inside the container as a
  path prefix, so the parent directory is created. It lists the mount and nothing else, which is why
  the check is `assert_eq!(parent, "project")` rather than a mere absence — the *first* draft probed
  the parent as an "unregistered directory" and would have failed on correct behaviour.
- **`$HOME` is the host's path.** `argv::create` passes the host home through as `HOME`, which is
  what makes `ls ~` returning nothing worth asserting: the path exists on the host, is not mounted,
  and resolves to nothing inside.

## Where this evidence is weaker than the table suggests

- One runtime, one platform. The Windows path mapping is asserted in `sandbox_argv.rs` against
  strings only; nobody has run a session inside a container on Windows.
- The credential probes prove the *default* is closed. They say nothing about what an opt-in mounts;
  that path has unit coverage and no live pass.
- The project is a temporary directory, not a git worktree. R2's real motivation — git metadata
  pointing at absolute paths — is exercised by the path identity, not by running git.

## Run

```
running 1 test
test sandbox_real_boundary_holds_from_inside_a_session ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
```

`cargo clippy -p micold-daemon --all-targets --features sandbox-real-runtime -- -D warnings` clean.
