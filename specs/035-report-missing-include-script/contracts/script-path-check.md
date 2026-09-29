# Contract: Script Path Check (core)

**Module**: `crates/micold-core/src/script_path_check.rs` (new), `pub mod script_path_check` in
`crates/micold-core/src/lib.rs`. Pure `std`, not `gui`-gated, tested by `mise run test-core`.

## API

```rust
pub const SCRIPT_PATH_CHECK_BOUND: Duration = Duration::from_secs(2);

pub enum ScriptPathState { Present, NotFound { tilde: bool }, NotReadable, Relative, Unchecked }
pub struct CheckedScriptPath { pub path: String, pub enabled: bool, pub state: ScriptPathState }

pub enum ProbeAnswer { File, NotAFile, Missing, Unreadable }
pub trait ScriptPathProbe { fn probe(&self, path: &Path) -> ProbeAnswer; }
pub struct StdScriptPathProbe;             // the real one: metadata + File::open
pub struct FakeScriptPathProbe { .. }      // answering(a) | blocking() ; calls() -> Vec<PathBuf>

pub fn classify(path: &str, probe: &dyn ScriptPathProbe) -> Option<ScriptPathState>;
pub fn check_bounded(probe: Arc<dyn ScriptPathProbe + Send + Sync>, path: String, bound: Duration)
    -> Option<ScriptPathState>;
```

Field and derive details (`Debug, Clone, PartialEq, Eq`) are left to the tasks.

## Behaviour

| # | Input | Probe called? | Result |
|---|---|---|---|
| C1 | `""`, `"   "` | no | `None` |
| C2 | `"~"`, `"~/env.sh"`, `"~\\env.ps1"` (a string test, asserted on every OS) | no | `Some(NotFound { tilde: true })` |
| C3 | `"env.sh"`, `"./env.sh"`, `"scripts/env.sh"` | no | `Some(Relative)` |
| C4 | absolute, probe `File` | yes, once, with the path unchanged | `Some(Present)` |
| C5 | absolute, probe `Missing` | yes | `Some(NotFound { tilde: false })` |
| C6 | absolute, probe `NotAFile` or `Unreadable` | yes | `Some(NotReadable)` |
| C7 | `check_bounded`, probe does not answer within `bound` | yes | `Some(Unchecked)`, returned within `bound` + scheduling slack |
| C8 | `check_bounded`, probe answers at once | yes | same as `classify` |

`StdScriptPathProbe::probe`:

| # | Filesystem | Answer |
|---|---|---|
| P1 | a regular file the user can open | `File` |
| P2 | nothing at the path | `Missing` |
| P3 | a directory | `NotAFile` |
| P4 | `cfg(unix)`: a file with mode `000` (skipped when running as root) | `Unreadable` |
| P5 | `metadata` error other than `NotFound` (for example, a parent directory without search permission, `cfg(unix)`) | `Unreadable` |
| P6 | a script that would create a marker file if run | `File`, and the marker does not exist afterwards (FR-003) |
| P7 | a symlink to a regular file | `File` (follows links, as `source` does) |
| P8 | `cfg(unix)`: a dangling symlink | `Missing` (`metadata` follows the link) |

## Non-goals

- It does not decide whether the feature is on. The check runs in both states (FR-001).
- It does not read the script's contents (Out of Scope).
- It does not replace `env_include::resolve`'s own `path.exists()` gate. 011's sourcing is
  unchanged (Out of Scope).
