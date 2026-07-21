# Contract: Environment-Include Resolution Engine

**Module**: `src/env_include.rs` (pure/core, not `gui`-gated — research R4)

## Inputs

| Input | Type | Source |
|---|---|---|
| `path` | `&Path` | `Settings::env_include_script_path` (or the live `SettingsDraft`, for a save-triggered refresh) |
| `timeout` | `Duration` | `Settings::env_include_timeout_secs`, clamped (`data-model.md`) |
| `enabled` | `bool` | `Settings::env_include_enabled` — checked by the caller; `resolve()` itself is only invoked when `true` (an explicit `Disabled` short-circuit is the caller's job, not this function's, since "disabled" isn't a resolution outcome — it's a reason resolution never runs) |

## Output

```rust
fn resolve(path: &Path, timeout: Duration) -> (Vec<(String, String)>, EnvIncludeOutcome)
```

`Vec<(String, String)>` is empty for every non-`Success` outcome. See `data-model.md` for
`EnvIncludeOutcome`'s shape.

## Behavior contract

1. **Path existence** (no subprocess spawned yet): if `path` does not exist, return
   `(vec![], EnvIncludeOutcome::MissingScript)` immediately.
2. **Baseline**: spawn the platform's no-rc shell invocation (`bash --noprofile --norc -c 'env -0'`
   on Linux/macOS; an equivalent clean-environment dump on Windows — research R6) and parse its
   stdout into a `HashMap<String, String>`. This step is not itself subject to `timeout` — it
   loads no rc files, so it is expected to return near-instantly; if it fails to spawn at all
   (e.g. `bash` missing from `PATH`), degrade to `EnvIncludeOutcome::NonZeroExit` with a diagnostic
   naming the spawn failure, same as any other subprocess-level failure.
3. **Attempt**: spawn the sourcing wrapper (research R1/R6) against `path`, polling for exit with
   the `timeout` bound (research R2). On Linux/macOS, the wrapper's shell invocation MUST satisfy
   any interactive-only guard the script itself checks (FR-019, research R1's BUG-001 note) — e.g.
   Debian/Ubuntu's stock `~/.bashrc` returns immediately from a non-interactive shell — so the
   default script path (FR-004) resolves its real exports, not just whatever precedes its own
   interactive-guard check.
   - Exits within `timeout`, status `0`: parse its stdout env dump, `diff_env(baseline, attempt)`
     (research R3), return `(diff, EnvIncludeOutcome::Success)`.
   - Exits within `timeout`, non-zero status: return `(vec![], EnvIncludeOutcome::NonZeroExit {
     code, diagnostic })` where `diagnostic` is the captured combined stdout+stderr from the
     wrapper's stderr stream (research R1).
   - Does not exit within `timeout`: kill it, return `(vec![], EnvIncludeOutcome::TimedOut {
     diagnostic })` using whatever partial diagnostic had already been captured before the kill.
4. **No usable diff** (spec Edge Case: an empty/no-op script): `diff_env` returning an empty `Vec`
   is **not** a failure — the outcome is still `Success` with `vars: vec![]`. This is a plain
   empty result, not a special case the caller needs to branch on.

## Non-goals (explicitly out of scope for this function)

- Does **not** decide *whether* to run (the `enabled` flag and empty-path-as-disabled edge case
  are the caller's concern — `refresh_env_include(app)` in `src/main.rs`).
- Does **not** persist anything — the caller decides what (if anything) goes to disk; per FR-008,
  the caller must persist nothing from this function's return value.
- Does **not** merge `TERM` or apply FR-009/FR-010 precedence — that merge happens at the two
  spawn call sites (`launch_spec()`, `ensure_attached_process`), per `data-model.md`'s
  "Relationships" section.

## Test obligations (Principle I — tests precede implementation)

- Pure `parse_env_dump`/`diff_env`: hand-built byte buffers / `HashMap`s, no subprocess, under
  `--no-default-features`.
- `resolve()` integration tests, real subprocess via `tempfile`-written scripts, `#[cfg]`-split
  per platform (mirrors `tests/shell_command.rs`):
  - A script that exports a new variable → `Success`, variable present in the returned `Vec`.
  - A nonexistent path → `MissingScript`, no subprocess spawned (assertable via a path that would
    error loudly if actually invoked, e.g. a directory instead of a file).
  - A script that `exit 1`s → `NonZeroExit { code: 1, .. }`, diagnostic contains the script's own
    printed output.
  - A script that sleeps longer than a short test timeout → `TimedOut`, and the call returns at
    (approximately) the configured timeout, not after the sleep completes.
  - An empty script → `Success`, empty `Vec`.
  - *(BUG-001)* A script shaped like Debian/Ubuntu's stock `~/.bashrc` — opening with
    `case $- in *i*) ;; *) return;; esac` followed by an `export` — → `Success`, the exported
    variable present in the returned `Vec` (FR-019). See
    `tests/env_include_resolve.rs::debian_default_bashrc_guard_blocks_export_from_reaching_session`.
