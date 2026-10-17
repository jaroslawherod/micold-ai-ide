# Contract: what #686 needs from the shared SSH image mechanism (#806)

Replaces revision 1's `daemon-image.md`. The mechanism is built by the #806 run (ledger decisions
9-11) and used by both the local container runtime and this runtime (spec FR-004, FR-024). This file
states only what #686 calls and relies on; #806's plan owns the module, its names and its internals
(working name in #686's documents: `micold-core/src/ssh_image/`, client side beside it). Any #686 task
that calls an item below is marked **blocked on #806**. If #806 ships a different shape, the calls in
`kube/start.rs` (phases `Copy`, `Install`, `Connect`), `client/kube/keys.rs` and the daemon actor's
Kubernetes dial adapt; nothing else in #686 depends on it.

## 1. The SSH image

- A published image reference, pinned by digest in the client build (one constant #686 reads as the
  default of `KubeSettings.image`), multi-arch (amd64, arm64).
- Contract a user-given image must also meet (spec FR-004): runs as a non-root user (UID 10001, home
  `/data/home`, so the volume at `/data` is writable under `fsGroup`); a POSIX shell; an SSH server
  started by the image's entrypoint, listening on `127.0.0.1:2222` only, public-key login only for that
  user, no password or root login, no forwarding but the daemon bridge's; it reads its host key and
  `authorized_keys` from a directory given by the environment (`/run/micold/ssh` in the pod, the
  access secret's mount) and copies the host key to `/tmp` with mode 0600 before starting; it works
  with a read-only root filesystem (writes only to `/tmp` and `/data`); it has what installing the AI
  CLIs needs (§4).
- Never inside it: the daemon, a credential, an AI CLI sign-in.

## 2. Keys

- `generate(comment) -> KeyPair { private_openssh, public_line }` (ed25519). #686 calls it for the
  pod's host key (once per daemon, stored in the access secret) and for each client's key (once per
  client per daemon, private part in owner-only files on the client). #686 reaches it through its
  `KeyGen` port, so the access-secret logic is built and tested with a fake before #806 lands.

## 3. Provisioning the daemon over SSH

Input: an SSH target `{ host: 127.0.0.1, port: <relay port>, user, key_file, known_hosts_file,
host_key_alias }` with `StrictHostKeyChecking=yes`, `UpdateHostKeys=no` (spec 489's rules), the
client's build id, and the daemon's environment (`Vec<(name, value)>`, from #686's `daemon_env`).

- Probe, copy the client's own daemon build if absent, start it detached, and open spec 489's bridge
  (`ssh::provision` FR-005, generalised to this target): the same steps as for a remote host, with
  the environment passed to the started daemon (the SSH server clears the login environment).
- Idempotent: a daemon of this build already present is not copied again; one already running is
  reused (spec US1.4).
- **Concurrent-safe (spec FR-023)**: two clients provisioning one volume at once end with one complete
  daemon build per version (copy to a temporary name, then rename), one running daemon (a lock file
  on the volume around start), and both connected to it. The later client waits within the phase
  bound.
- Progress for the row's activity ("Copying daemon"), and a classified failure whose text names the
  cause (spec 489's reasons; a host key mismatch distinguishable so #686 shows `HostKeyMismatch`).

## 4. Installing the AI CLIs on start

Input: the same SSH target, the set of CLIs (`claude`, `copilot`, `pi`), and the install root on the
volume.

- Each CLI at the version the client build pins for it; a CLI already installed at that version is
  skipped (spec US1.4); a CLI not in the set is left installed but not offered (spec FR-004).
- Output streamed line by line (the daemon's log), cancellable by #686 (spec FR-012), bounded by the
  caller (15 min).
- **Concurrent-safe**: install into a temporary directory and rename, under a lock file per CLI on the
  volume; the later client waits and then skips.
- A classified failure naming the CLI and its error (no outbound network, package source
  unreachable, volume full).
- A way for the CI suite (spec FR-021) to install a scripted stand-in CLI from a source reachable in
  the job (a directory on the volume or a file server).

## 5. What #686 builds before #806 lands

Everything up to an open tunnel to the pod's SSH server, proven over `FakeCluster` and the live
adapter's unit tests: kubeconfig, settings, manifests, the access secret (with a fake `KeyGen`), the
pod/PVC/secret lifecycle with rollback, the relay and the `Tunnel` phase (the SSH identification
line), Stop, Remove, the failure reasons, and the Settings form with the Add entry hidden (spec
FR-024). #686 lands `micold-core/src/daemon_resources.rs` (`DaemonResources`) and the resources
fields extracted in `ui/settings/container_fields.rs` in its first milestone; #806 adopts them rather
than creating its own. `PROTOCOL_VERSION`: whichever of #686's `RepoClone` (49) and a #806 bump merges
second takes the next number and re-pins the `schema_hash.rs` hashes. The blocked milestones then wire §2-§4, the clone (protocol 49), reconnect over the bridge,
the Add entry, and the real-cluster suite.
