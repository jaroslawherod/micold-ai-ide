# Quickstart: The Session Daemon in a Sandbox

Two parts. **§A** is what the machine checks, on every platform, with **no container runtime
installed**. **§B** is what only a real runtime and a real pair of eyes can settle — whether the
sandbox actually cannot see what it must not see, and whether the new Settings view is usable.

The split is unusually load-bearing here. §A can prove the *right arguments* are constructed; only §B
can prove the *resulting container* is confined. A feature whose entire value is a boundary cannot
be signed off on argv assertions alone.

---

## §A — The automated suite

```bash
mise run test        # whole workspace, matching CI
mise run test-core   # sandbox + settings + protocol only; much faster while iterating
```

Green is the gate. What each gate is watching:

| Gate | Watching |
|---|---|
| `micold-core/src/sandbox/argv.rs` (unit) | argv is a pure function of the spec — identical across repeated builds (K-1), correct flag and unit per supported limit (K-2) |
| `micold-core/src/sandbox/argv.rs` (unit) | **no flag for an unsupported limit** (K-3), and reconciliation reports it with a reason — the R5 answer, checked as behaviour rather than documented as a caveat |
| `micold-core/src/sandbox/argv.rs` (unit) | argv mounts equal the `MountSet` as sets (K-4) — nothing implicit, no home, no runtime socket |
| `micold-core/src/sandbox/argv.rs` (unit) | on Linux/macOS specs every `ProjectMount` has `container == host` (K-5) — the R2 claim git's worktree metadata depends on |
| `micold-core/src/sandbox/argv.rs` (unit) | `NoOutbound` emits the masquerade-disabled network **and** the published port (K-7). The measured failure mode — an `--internal` network that makes the port inert — is asserted *not* to be generated |
| `micold-core/src/sandbox/argv.rs` (unit) | the escalation denylist: no `--privileged`, `--cap-add`, `--pid=host`, `--network=host`, `seccomp=unconfined`, no host path outside the `MountSet` (K-11) |
| `micold-core/tests/sandbox_argv.rs` | the **Windows** mapping, on whatever platform runs: mapped container paths under `/mnt/host`, host paths unrewritten, rule M-1 under both mappings, and `argv` agreeing with `Placement::git_routing_for` about whether the two halves differ (T114) |
| `micold-core/tests/sandbox_needs_no_runtime.rs` | nothing outside a feature-gated `sandbox_real_*` target reaches `docker`/`podman`, and the process boundary stays in `exec.rs` — the property the three-platform matrix depends on (T115) |
| `micold-core/tests/quickstart_a_runs_everywhere.rs` | the `micold-client` rows below are each named in ci.yml's cross-platform step. That suite runs in full on Linux only, so those rows are enumerated one `--test` flag at a time — and an enumerated list drifts silently. This table's "on every platform" is therefore checked rather than asserted (T115) |
| `micold-core/tests/sandbox_runtime.rs` | each canned runtime failure maps to its `RuntimeError` variant (K-8); malformed JSON classifies rather than panics (K-12) |
| `micold-core/tests/sandbox_runtime.rs` | stop/remove/start are idempotent (K-9); `acquire_image` emits progress more than once (K-10) |
| `micold-core/tests/sandbox_runtime.rs` | podman's dialect passes K-1…K-12 too — an abstraction with one implementation is a guess |
| `micold-core/tests/sandbox_state.rs` | no edge leaves `Failed` for a working unsandboxed daemon without an explicit action (S-2) — FR-035's guarantee, as a graph property |
| `micold-core/tests/sandbox_state.rs` | every terminal failure carries a reason **and** a remedy from the closed enumeration (S-4) |
| `micold-core/tests/settings_roundtrip.rs` | a verbatim v3 document loads with `HostProcess` and a default sandbox (T-1); each missing leaf defaults (T-2) |
| `micold-core/tests/settings_roundtrip.rs` | `credentials` absent → **empty** (T-3). The one default that is a security property, not a convenience |
| `micold-core/tests/settings_roundtrip.rs` | an unknown root key survives a round-trip (T-4); truncated JSON degrades to defaults (T-5); out-of-range budgets clamp and report (T-6) |
| `micold-core/tests/settings_roundtrip.rs` | the token appears nowhere in the written file (T-8) |
| `micold-core/tests/protocol_auth.rs` | right token accepted, wrong and absent refused as `AuthRejected` (P-1); the token is in no log line, argv, or inspect output (P-3) |
| `micold-core/tests/protocol_auth.rs` | fingerprint mismatch refuses a `LocalBuild` image as `StaleDevImage` and accepts a `Registry` one (P-4) — the asymmetry R8 requires |
| `micold-core/tests/schema_hash.rs` | the hash **moves**, once, deliberately. This feature changes the handshake; if the hash did not move, the new fields never reached the wire |
| `micold-client/tests/idle_requests_no_frames.rs` | the new Settings view does not animate at rest — the regression the view rewrite is most likely to cause |
| `micold-client/tests/features_settings.rs` | each section's draft validates independently; leaving a section with an invalid field does not silently discard the edit |
| `micold-client/tests/anatomy_call_sites.rs` | `section_list` is built in `ui/material/` with the chainable-builder-into-`Element` API, not privately in the feature (Principle VIII) |

**The fake runtime** backs every `sandbox_*` test — as an injected `exec::CommandRunner`, not a
binary on `PATH`. `CliRuntime` is generic over the runner; the tests hand it `RecordingRunner`,
which records each argv and replays canned output in process, so nothing is spawned and `PATH` is
never consulted. That is why §A runs on Linux, macOS and Windows with nothing installed, and it is
the only reason Principle VI's coverage of the adapter layer is honest rather than aspirational.
`sandbox_needs_no_runtime.rs` is what keeps it true: only a `sandbox_real_*` target, gated on the
`sandbox-real-runtime` feature, may construct `SystemRunner` or spawn a runtime by name.

**What §A cannot tell you**: whether the container it describes is actually confined. Every test
above asserts on strings.

---

## §B — The manual pass

Needs Docker installed. The view work in §B.6 is runnable with the repo's `visual-pass` skill rather
than by hand.

Run end to end on 2026-08-26 against Docker 29.5.1, in release, on one freshly built image;
transcript and the two commands in `evidence/quickstart-b-closeout.md`. Every box below is ticked
except the idle-repaint one in §B.6, which a software rasteriser cannot settle.

### B.1 — First enable, cold

Start from no image present (`docker rmi` the tag first). Settings → Session service → enable the sandbox.

Driven by the application's own enable sequence rather than by hand, so the evidence and the code
cannot drift apart. The GUI half — that this is reachable from Settings → Session service — is
covered by §B.6; what is measured here is what happens after the switch is thrown.

- [x] Progress moves continuously through image acquisition; no silent stretch longer than a few
      seconds (SC-004, C-8) — `sandbox_real_enable.rs`, `evidence/performance.md`
- [x] Whole thing completes in under five minutes on a normal connection (SC-004) — same test.
      Its cold state is honestly a cold *reference* and not a cold machine (the layers stay in the
      local store), and the registry route cannot be measured at all until an image is published;
      both caveats are stated in the test and the evidence rather than hidden in the total
- [x] A session starts, and its terminal behaves exactly as an unsandboxed one (SC-001, FR-025) —
      twelve commands run in both placements, identical answers (`sandbox_real_parity.rs`,
      `evidence/us2-parity.md`)

### B.2 — The boundary, tested adversarially

This is the feature. In a sandboxed session's terminal:

Covered twice, and the second pass is the one that counts: `evidence/us1-isolation.md` probed the
container with `docker exec` and a replaced entrypoint, `evidence/us1-isolation-from-a-session.md`
probes it through a session the daemon spawned, over the control channel, as
`crates/micold-daemon/tests/sandbox_real_boundary.rs` — so these boxes now re-check themselves on
every `sandbox-runtime` CI run.

- [x] `ls /` shows the container's root, not the host's
- [x] A registered project is present **at its host absolute path** (R2)
- [x] `ls ~` does **not** show the host home directory's contents
- [x] An unregistered directory outside every project is unreachable
- [x] `ls /var/run/docker.sock` — absent. The sandbox cannot drive its own runtime (C-3)
- [x] With no credential opt-ins: `cat ~/.gitconfig`, `ssh-add -l`, and the AI CLI's auth path all
      come back empty or absent (FR-004a)
- [x] `touch <project>/probe && ls -l` on the **host** shows the file owned by your user, not root
      (R3, C-4). Delete it afterwards

### B.3 — Network posture

- [x] With `NoOutbound`: `curl https://example.com` fails; the session stays connected throughout —
      the control channel is unaffected (R4, C-5)
- [x] With `NoOutbound`: `nslookup example.com` **resolves**. Confirm the documented caveat is
      accurate and that `docs/user-guide/sandboxed-daemon.md` states it (R4)
- [x] Switching to `Outbound` and restarting the sandbox: `curl` succeeds

### B.4 — Limits

`evidence/us4-limits.md`, as `sandbox_real_limits.rs` and `sandbox_real_storage.rs`. The third box
found a real defect: `overlayfs` was classified as enforcing a storage limit it silently ignores.

- [x] Set a memory limit, then allocate past it in a session: the process is killed, the session
      reports it, and the daemon survives (FR-012, FR-016)
- [x] On a runtime that cannot enforce storage: the field is shown **disabled with the reason**, not
      hidden and not silently accepted (FR-015, SC-009, C-2)
- [x] Change a limit, restart the sandbox, confirm it took effect

### B.5 — Lifecycle and failure

- [x] Register a new project while the sandbox runs: it is marked **stale** with an explicit restart
      action; running sessions keep working; nothing restarts on its own (R9, M-4) —
      `sandbox_real_staleness.rs`, which also measures the premise staleness rests on: the project
      is genuinely unreachable inside the running container
- [x] `docker stop` the container from outside: the client shows a persistent failure with a reason
      and a remedy, and does **not** fall back to an unsandboxed daemon (FR-035, FR-035b, S-2)
- [x] Accept the offered fallback explicitly: it works, and the unsandboxed state stays visible for
      as long as it lasts (FR-035a/b) — and it did **not**, until `8ee7741`: the consent was
      recorded and nothing re-dialled. `evidence/us6-failures.md` §B.5 box 3
- [x] Sessions survive a client restart while sandboxed (FR-014) — same shell process, same shell
      state, over a new connection (`sandbox_real_staleness.rs`)
- [x] With survive-logout enabled, the sandbox comes back after a reboot (FR-014a/b, R6) —
      `sandbox_real_staleness.rs`, with a `--restart no` control beside it. **No reboot was
      performed**: the probe kills the container's own process on the host, so the runtime restarts
      it unasked, the daemon comes back up, and the session is still in the catalogue and still
      answers. What is left to the runtime is restoring the container at boot.
      `evidence/quickstart-b-closeout.md`
- [x] Daemon state survives container recreation: stop, remove, restart — projects are still there
      (FR-011, M-3)

### B.6 — The Settings view

- [x] Settings opens as a full-surface view with a navigation rail, not a 420-point modal (FR-026)
- [x] Every daemon setting is in the Daemon section, and no daemon setting is left elsewhere (FR-027)
- [x] Every pre-existing setting still exists and still works (FR-028) — the migration's real risk
- [x] Active credential opt-ins are each individually visible while active (FR-004c, N-2)
- [x] Keyboard navigation reaches every section and every control; focus order is sane
- [x] Both themes; clean to 640pt wide, degrading below it — no minimum is declared anywhere
      (evidence/us3-settings-view.md)
- [x] Idle with the view open: no repainting (the automated counterpart is
      `idle_requests_no_frames.rs`) — measured by sampling the **X server's** CPU alongside the
      client's: 6 ticks/20s with the view open, 6 with it closed, and 741 while the pointer is
      moved. lavapipe makes a presented frame expensive, which is what makes its absence readable
      (evidence/us3-settings-view.md)
- [x] Every section in the rail carries an icon, and the icons are distinguishable from one another
      at the rail's own size (FR-026b)
- [x] Collapsed, the rail shows icons alone, still marks the current section, still navigates by
      pointer and by keyboard, and the width it gives up goes to the section's content (FR-026c) —
      this found the current row's icon 12dp off the column the other three form; fixed, and
      `gates/rail_icons_align.rs` now asserts it
- [x] The collapsed state survives leaving and reopening Settings, and Save and Cancel both leave it
      alone (FR-026d)
- [x] The overflow menu offers no control a section owns — no theme, no session survival — and
      session survival still works from its section under both placements (FR-026e, FR-014d)

### B.7 — The development loop (FR-024c)

Run 2026-08-26 against Docker 29.5.1 on Linux; transcripts in `evidence/us7-dev-loop.md`. This is
the part of the pass everyone assumes works because everyone builds a `:dev` image — and it did not:
the `StaleDevImage` refusal reached the developer as a `{:?}` dump with no rebuild command, and the
tag inside it was empty, because nothing ever set `MICOLD_IMAGE_REFERENCE`. Both fixed here.

Box 4 is ticked for the round-trip only. "With the network off entirely" could not be checked on
this machine and is recorded as unverified; what was measured is that no registry can serve the tag
and that create succeeds under `--pull never`.

- [x] `mise run image` builds a `:dev` image from the working tree
- [x] Running against it works — with the staleness check *armed*, which is the claim a lenient
      handshake does not make (`sandbox_real_fingerprint.rs`)
- [x] Rebuild the client only, leave the `:dev` image stale, reconnect: refused as `StaleDevImage`,
      naming the tag and the rebuild command (FR-024d, R8, P-4)
- [x] `docker save` / `docker load` round-trips the image (`sandbox_real_enable.rs`, SC-004a), and
      enabling the sandbox consults no registry (FR-024a). Principle IV's offline claim holds as
      "no registry is contacted"; "works with the machine offline" remains unverified — see the
      evidence for why the usual substitutes test the wrong thing
