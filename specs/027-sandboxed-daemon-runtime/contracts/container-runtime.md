# Contract: `ContainerRuntime`

**Feature**: [../spec.md](../spec.md) | **Status**: durable — a runtime that does not satisfy this
cannot claim support (FR-020, SC-009)

This is the seam FR-020 requires: everything above it describes *what the sandbox should be*, and
everything below it knows how one particular runtime spells that. Docker is the implementation
shipped at release (FR-021); podman is written alongside it, not deferred, because an abstraction with
one implementation is a guess.

## The trait

```rust
pub trait ContainerRuntime {
    /// Is this runtime installed and usable? Cheap; no container is created.
    fn detect(&self) -> Result<RuntimeVersion, RuntimeUnavailable>;

    /// What can it enforce here? Runs once per version and is cached against it (R10).
    fn probe(&self) -> Result<RuntimeCapabilities, RuntimeError>;

    /// Is the image present locally, and is it stale? (FR-024, FR-024d)
    fn inspect_image(&self, image: &ImageRef) -> Result<Option<ImageFacts>, RuntimeError>;

    /// Make the image available: pull, import from a file, or build (FR-024a–c).
    fn acquire_image(&self, src: &ImageSource, progress: &mut dyn FnMut(Progress))
        -> Result<ImageFacts, RuntimeError>;

    fn create(&self, spec: &SandboxSpec) -> Result<ContainerId, RuntimeError>;
    fn start(&self, id: &ContainerId) -> Result<(), RuntimeError>;
    fn stop(&self, id: &ContainerId) -> Result<(), RuntimeError>;
    fn remove(&self, id: &ContainerId) -> Result<(), RuntimeError>;
    fn inspect(&self, id: &ContainerId) -> Result<ContainerFacts, RuntimeError>;
    fn logs(&self, id: &ContainerId, lines: usize) -> Result<Vec<String>, RuntimeError>;
}
```

`SandboxSpec` is the resolved, reconciled input — a `SandboxProfile` that has already been validated
against `RuntimeCapabilities` and joined with the `MountSet`. Implementations never re-validate and
never silently drop a field; if a spec reaches `create`, every part of it is enforceable.

## Layering: what is pure and what is not

```text
argv.rs   PURE   SandboxSpec + RuntimeCapabilities -> Vec<OsString>
parse.rs  PURE   runtime stdout (--format '{{json .}}') -> typed facts
dialect/  PURE   the per-runtime differences: flag names, defaults, quirks
exec.rs   IMPURE the single process-spawn shim everything above is composed over
```

Only `exec.rs` touches the world. This is the same split `micold-core/src/git.rs` uses for git
porcelain (R7), and it is what lets the entire adapter layer be tested with no runtime installed.

## Obligations

**C-1 — Argv is a pure function of the spec.** Given the same `SandboxSpec` and
`RuntimeCapabilities`, argv is byte-identical. No environment lookups, no clocks, no randomness
inside the builder; anything variable (uid, generated token path, port) arrives *in* the spec.

**C-2 — No flag for an unsupported limit.** If `RuntimeCapabilities` reports a limit
`Unsupported`, argv contains no flag for it. The user's stored value is preserved (RC-3) but never
passed. Violating this is the "silent drift" R5 exists to prevent.

**C-3 — Mounts are exactly the `MountSet`, and nothing else.** No implicit home mount, no runtime
socket, no `/var/run/docker.sock`. A dialect that adds a convenience mount fails the suite. This is
the guarantee the whole feature rests on (FR-006, FR-007).

**C-4 — Identity is mapped, not assumed (R3).** Docker emits `--user <uid>:<gid>`; podman emits
`--userns=keep-id`. Files written into a mounted project are owned by the invoking host user.

**C-5 — `NoOutbound` keeps the control channel (R4).** The network is a user-defined bridge created
with IP masquerade disabled, and the daemon port is published to loopback. A dialect that expresses
"network off" by making the container unreachable fails the suite — that configuration was measured
and rejected.

**C-6 — Errors are classified, never raw text.** `RuntimeError` is a closed enumeration
(`NotInstalled`, `NotRunning`, `VersionTooOld`, `PermissionDenied`, `ImageNotFound`,
`ImagePullFailed`, `PortUnavailable`, `MountRejected`, `LimitRejected`, `SandboxStopped`, `Timeout`,
`Unknown { stderr }`), each carrying the reason and remedy FR-034 requires. `Unknown` retains stderr
for the log and is the only variant allowed to surface unclassified text.

`SandboxStopped` is the odd one out: no runtime command produces it. It is what the client's
liveness check reports when the container it was using has been stopped or removed from outside the
application, and it lives here rather than in a state of its own because `Failure` is the single
thing the application has for "the sandbox is not usable, and this is why" (FR-036).

**C-7 — Idempotence.** `stop` on a stopped container, `remove` on an absent one, and `start` on a
running one all succeed. The client's recovery paths call these without first checking, and a
race with the user's own `docker stop` must not produce an error dialog.

**C-8 — Progress is reported during acquisition, not just at the end.** `acquire_image` invokes the
callback often enough for `StageProgress` to move. SC-004 gives first-time enable five minutes; five
silent minutes reads as a hang.

**C-9 — No privilege escalation.** No dialect emits `--privileged`, `--cap-add`, `--security-opt
seccomp=unconfined`, `--pid=host`, `--network=host`, or a host-path mount outside the `MountSet`.
Asserted by a denylist test over generated argv, so a future dialect cannot quietly add one.

## Conformance suite

A runtime claims support by passing this suite. It runs twice: against the **fake runtime binary**
(all three platforms, every CI run, no container involved) and against the real runtime (Linux CI
job, plus `quickstart.md` Part B).

| # | Check | Obligation |
|---|---|---|
| K-1 | argv is identical across repeated builds of one spec | C-1 |
| K-2 | each supported limit produces exactly its flag, with the expected unit conversion | C-1 |
| K-3 | each unsupported limit produces no flag, and reconciliation reports it | C-2 |
| K-4 | argv mounts == `MountSet`, compared as sets | C-3 |
| K-5 | on Linux/macOS specs, every `ProjectMount` has `container == host` | C-3, R2 |
| K-6 | identity flag matches the dialect's `IdentityMapping` | C-4 |
| K-7 | `NoOutbound` yields the masquerade-disabled network **and** the published port | C-5 |
| K-8 | each canned runtime failure maps to its `RuntimeError` variant | C-6 |
| K-9 | stop/remove/start are idempotent against canned "already in that state" output | C-7 |
| K-10 | `acquire_image` emits ≥2 progress callbacks for multi-layer canned output | C-8 |
| K-11 | generated argv contains none of the denylisted escalation flags | C-9 |
| K-12 | malformed/truncated runtime JSON yields a classified error, never a panic | C-6 |

## The fake runtime

**Amended during implementation (T004).** This was specified as a small executable placed first on
`PATH`. That shape does not survive contact with `cargo test`: `PATH` is process-global, cargo runs
tests as parallel **threads** of one process, and a test that rewrites `PATH` rewrites it for every
other test running at that moment. (Edition 2024 makes the same point by marking `set_var`
`unsafe`.) A harness that races is worse than no harness, because it fails intermittently and gets
blamed on the code under test.

So the seam is one level in, at `sandbox/exec.rs`: `CommandRunner` is injected, `SystemRunner`
spawns for real, and `RecordingRunner` records argv and replays canned output in-process. A test
asserts on the recorded invocations to check *what was asked for*, and seeds the response queue to
control *what came back* — including failures that are hard to arrange with a real runtime (daemon
down, disk full, an image vanishing between inspect and create).

Everything the conformance suite asserts — argv construction, output parsing, error classification
— sits **above** this seam and is exercised identically on all three platforms with nothing
installed, which was the property the fake binary existed to provide. What remains below it is the
spawn itself, covered by one real-spawn test against a command every platform ships.

## Adding a runtime

1. Add a `RuntimeKind` variant and a `dialect/<name>.rs`.
2. Declare its baseline capabilities and its probe commands.
3. Declare **its own wording** for every failure the dialect names — service down, not permitted,
   mount refused. These are separate lists per runtime rather than one shared table because the
   runtimes do not merely phrase the same sentence differently: Docker names the mount
   configuration for a refused bind, podman names the syscall (`statfs`). A list borrowed from
   another runtime classifies nothing, and the user gets `Unknown` for a failure the application
   knows how to explain.
4. Pass K-1 … K-12 against a fake binary speaking that runtime's output format.
5. Document its quirks in `docs/user-guide/sandboxed-daemon.md`.

No change to `argv.rs`'s callers, the client, or the daemon. If a new runtime forces one, the
abstraction is wrong and that is the signal to revisit this contract rather than special-case around
it.
