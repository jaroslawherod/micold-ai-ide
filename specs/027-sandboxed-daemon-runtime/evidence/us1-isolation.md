# US1 verification: the agent can only touch the project

**Date**: 2026-08-19 · **Runtime**: Docker 29.5.1, `overlayfs`, cgroup v2, x86_64 Linux
**Image**: `micold-daemon:dev`, built from this working tree
**Covers**: quickstart.md §B.2 (the boundary), §B.3 (network posture), §B.4 (limits)

The container was created with the argument vector `sandbox::argv::create` produces for a default
profile — the same flags, in the same order — with a temporary directory standing in for a
registered project and the entrypoint replaced by a shell so the boundary could be probed from
inside. The daemon binary itself is not what is under test here; what the sandbox *can reach* is.

## §B.2 — the boundary, tested adversarially

| Check | Result |
|---|---|
| A registered project is present at its **own host absolute path** | `hello from the host` — read through the identical path (research R2) |
| `ls /home/jaro` — the host home directory | `No such file or directory` |
| An unregistered path outside every project (`/home/jaro/workspaces`) | `No such file or directory` |
| `ls -l /var/run/docker.sock` — the runtime's own control socket | `No such file or directory` |
| `cat ~/.gitconfig` with no credential opt-in | `No such file or directory` |
| The mounted token, readable by the daemon | present at `/run/micold/token` |
| Process table | 6 processes — the container's own, not the host's |

## Ownership (FR-004's companion, research R3)

```
$ docker exec micold-sandbox touch <project>/written-inside.txt
$ ls -l <project>/written-inside.txt      # on the host
host sees owner: jaro:jaro                # host uid:gid = 1000:1000
```

A file a session creates comes back owned by the user who ran the application, editable without
elevation. This is the failure that would have made the sandbox worse than no sandbox.

## §B.3 — network posture

```
egress  : BLOCKED     # wget http://1.1.1.1/ fails
dns     : RESOLVES    # getent hosts example.com succeeds
```

Both as documented. The DNS result is the caveat `docs/user-guide/sandboxed-daemon.md` states
explicitly: names resolve because the runtime's embedded resolver answers from the host side, while
connections to them fail. The posture is "no outbound connections", not "no outbound traffic".

## §B.4 — limits actually applied

```
$ docker inspect micold-sandbox --format '{{.HostConfig.NanoCpus}} / {{.HostConfig.Memory}} / {{.HostConfig.PidsLimit}} / {{.HostConfig.RestartPolicy.Name}}'
2000000000 nanocpus / 4294967296 bytes / 512 pids / restart=no
```

The default budget reached the runtime intact: 2 cores, 4 GiB, 512 processes. `restart=no` is
correct for a profile with session survival off.

## The token does not leak

```
$ docker inspect micold-sandbox | grep -c deadbeef
0
```

The secret is delivered as a read-only bind mount, so it is absent from the container's
configuration, its argument vector, and anything `inspect` shows (obligation P-3).

## Not covered here

Starting an actual session inside the sandbox, and the client's end-to-end connect. Those need the
GUI and belong to §B.1 and §B.5, which remain outstanding — see tasks T043 and T045's follow-ups.
What this run establishes is the claim every other part of the feature rests on: the boundary is
real, and it is the one the code asks for.
