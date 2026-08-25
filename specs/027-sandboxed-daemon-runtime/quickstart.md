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
| `micold-core/tests/sandbox_argv.rs` | argv is a pure function of the spec — identical across repeated builds (K-1), correct flag and unit per supported limit (K-2) |
| `micold-core/tests/sandbox_argv.rs` | **no flag for an unsupported limit** (K-3), and reconciliation reports it with a reason — the R5 answer, checked as behaviour rather than documented as a caveat |
| `micold-core/tests/sandbox_argv.rs` | argv mounts equal the `MountSet` as sets (K-4) — nothing implicit, no home, no runtime socket |
| `micold-core/tests/sandbox_argv.rs` | on Linux/macOS specs every `ProjectMount` has `container == host` (K-5) — the R2 claim git's worktree metadata depends on |
| `micold-core/tests/sandbox_argv.rs` | `NoOutbound` emits the masquerade-disabled network **and** the published port (K-7). The measured failure mode — an `--internal` network that makes the port inert — is asserted *not* to be generated |
| `micold-core/tests/sandbox_argv.rs` | the escalation denylist: no `--privileged`, `--cap-add`, `--pid=host`, `--network=host`, `seccomp=unconfined`, no host path outside the `MountSet` (K-11) |
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

**The fake runtime** backs every `sandbox_*` test: a binary placed first on `PATH` that records its
argv and replays canned output. That is why §A runs on Linux, macOS and Windows with nothing
installed — and it is the only reason Principle VI's coverage of the adapter layer is honest rather
than aspirational.

**What §A cannot tell you**: whether the container it describes is actually confined. Every test
above asserts on strings.

---

## §B — The manual pass

Needs Docker installed. The view work in §B.6 is runnable with the repo's `visual-pass` skill rather
than by hand.

### B.1 — First enable, cold

Start from no image present (`docker rmi` the tag first). Settings → Session service → enable the sandbox.

- [ ] Progress moves continuously through image acquisition; no silent stretch longer than a few
      seconds (SC-004, C-8)
- [ ] Whole thing completes in under five minutes on a normal connection (SC-004)
- [ ] A session starts, and its terminal behaves exactly as an unsandboxed one (SC-001, FR-025)

### B.2 — The boundary, tested adversarially

This is the feature. In a sandboxed session's terminal:

- [ ] `ls /` shows the container's root, not the host's
- [ ] A registered project is present **at its host absolute path** (R2)
- [ ] `ls ~` does **not** show the host home directory's contents
- [ ] An unregistered directory outside every project is unreachable
- [ ] `ls /var/run/docker.sock` — absent. The sandbox cannot drive its own runtime (C-3)
- [ ] With no credential opt-ins: `cat ~/.gitconfig`, `ssh-add -l`, and the AI CLI's auth path all
      come back empty or absent (FR-004a)
- [ ] `touch <project>/probe && ls -l` on the **host** shows the file owned by your user, not root
      (R3, C-4). Delete it afterwards

### B.3 — Network posture

- [x] With `NoOutbound`: `curl https://example.com` fails; the session stays connected throughout —
      the control channel is unaffected (R4, C-5)
- [x] With `NoOutbound`: `nslookup example.com` **resolves**. Confirm the documented caveat is
      accurate and that `docs/user-guide/sandboxed-daemon.md` states it (R4)
- [x] Switching to `Outbound` and restarting the sandbox: `curl` succeeds

### B.4 — Limits

- [ ] Set a memory limit, then allocate past it in a session: the process is killed, the session
      reports it, and the daemon survives (FR-012, FR-016)
- [ ] On a runtime that cannot enforce storage: the field is shown **disabled with the reason**, not
      hidden and not silently accepted (FR-015, SC-009, C-2)
- [ ] Change a limit, restart the sandbox, confirm it took effect

### B.5 — Lifecycle and failure

- [ ] Register a new project while the sandbox runs: it is marked **stale** with an explicit restart
      action; running sessions keep working; nothing restarts on its own (R9, M-4)
- [x] `docker stop` the container from outside: the client shows a persistent failure with a reason
      and a remedy, and does **not** fall back to an unsandboxed daemon (FR-035, FR-035b, S-2)
- [ ] Accept the offered fallback explicitly: it works, and the unsandboxed state stays visible for
      as long as it lasts (FR-035a/b)
- [ ] Sessions survive a client restart while sandboxed (FR-014)
- [ ] With survive-logout enabled, the sandbox comes back after a reboot (FR-014a/b, R6)
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
- [ ] Idle with the view open: no repainting (the automated counterpart is
      `idle_requests_no_frames.rs`) — **inconclusive under lavapipe**; the view costs no more idle
      than the main surface, but the absolute claim rests on the automated test

### B.7 — The development loop (FR-024c)

- [ ] `mise run image` builds a `:dev` image from the working tree
- [ ] Running against it works
- [ ] Rebuild the client only, leave the `:dev` image stale, reconnect: refused as `StaleDevImage`,
      naming the tag and the rebuild command (FR-024d, R8, P-4)
- [ ] `docker save` / `docker load` round-trips the image, and enabling the sandbox works with the
      network off entirely (FR-024a — Principle IV's offline claim, which is nominal until this is
      checked once)
