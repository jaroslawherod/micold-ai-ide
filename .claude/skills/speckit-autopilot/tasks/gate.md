# Task: run the local gate

When: a task file sends you here. Pick the gate by what the diff touches.

| Gate | When | Command |
|---|---|---|
| **Full** | Before any push of code, tests, scripts or build config | `mise run gate` |
| **Scoped** | Between review rounds of a milestone | `scripts/autopilot/scoped-gate.sh` |
| **Specs-only** | Every changed file is under `specs/` | `scripts/check-criteria-observables.sh specs/<dir>/spec.md` |
| **Docs** | Only `docs/`, `.claude/` or `*.md` beside `specs/` | `mise run test-scripts` |

Start the full or the scoped gate detached, and wait with `hold.sh`
([../rules/waiting.md](../rules/waiting.md)); its `DONE` line carries the exit code:

```bash
log="$SCRATCHPAD/gate-$(date +%s).log"
setsid nohup bash -c 'mise run gate; echo "GATE_EXIT=$?"' >"$log" 2>&1 &
```

- **Free disk first.** Two milestones lost a gate to `No space left on device`: if `df` shows under
  20 GB free on the target's filesystem, run `mise run sweep` (the `reclaim-disk` skill) before you start it.
- **Always `mise run gate`** (or the scoped gate), never the raw cargo commands: the gate hook counts
  only a green `mise run gate`, and two pushes were refused for it.
- **Detach it.** A plain background task can be killed while it waits on the build lock.
- **The full gate's order is CI's:** fmt → clippy (core, then workspace) → `cargo test --workspace`
  → `mise run test-scripts`. `mise run test` alone is not the gate.
- **The scoped gate** checks fmt, then clippy and tests of the crates the branch changed. It
  records nothing: only a green full gate lets `git push` through.
- **Push only what the gate saw.** A green full gate records the tree it ran on, and a hook blocks
  `git push` of code no green gate saw. Commit the tree the gate ran on; only docs, specs and the
  ledger may change after it.
- **Changed a `cfg(target_os = …)` arm?** Also run
  `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`.
- **Changed how sessions run in a container** (the sandbox code or the image's build files)? Also
  `mise run image`, then `mise run test-sandbox`. No container runtime installed: say so in the
  PR body; CI's sandbox job runs it.
- **Changed how something looks?** (In the chore flow that is not a chore: return `NEXT:`.) Run the `visual-pass` skill through an `autopilot-worker`. Save
  its evidence in the spec directory, when the flow has one.

Next: the task that sent you here.
