# Contract: Session terminal identity

**Feature**: 031-clickable-terminal-links | **Modules**: `micold_core::env_include`,
`crates/micold-daemon/src/supervisor.rs` | **Research**: R12

What a program running in any session sees when it asks "does this terminal show hyperlinks?"
(FR-006).

## 1. The inherited-identity predicate

`micold_core::env_include::is_inherited_terminal_identity(key: &str, value: &str, keys_case_insensitive: bool) -> bool`

| Key | True when |
|---|---|
| `FORCE_HYPERLINK`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`, `VTE_VERSION`, `WT_SESSION`, `WT_PROFILE_ID`, `TERMINAL_EMULATOR`, `KONSOLE_VERSION`, `DOMTERM` | always |
| `COLORTERM` | value is not `truecolor` or `24bit` (ASCII case-insensitive) |
| anything else | never |

Keys compare exactly when `keys_case_insensitive` is false and ASCII case-insensitively when it is
true. Callers pass `cfg!(windows)`; core does not read the host OS.

## 2. Session spawn (daemon)

For both `PtySession::spawn_ai_cli` and `PtySession::spawn_shell`, on the host and in the sandbox:

1. `CommandBuilder::new(..)` (inherits the daemon's environment).
2. **Remove** every inherited variable for which §1 is true.
3. Apply `TERM=xterm-256color` and the session's include-script values, as today.

So:

| Daemon environment | Include script sets | Session sees |
|---|---|---|
| `TERM_PROGRAM=WezTerm` | — | no `TERM_PROGRAM` |
| `FORCE_HYPERLINK=1` | — | no `FORCE_HYPERLINK` |
| — | `FORCE_HYPERLINK=1` | `FORCE_HYPERLINK=1` |
| `FORCE_HYPERLINK=1` | `FORCE_HYPERLINK=1` | `FORCE_HYPERLINK=1` (see §3) |
| `COLORTERM=truecolor` | — | `COLORTERM=truecolor` |

## 3. Include-script shells (core)

The three `Command` construction sites the include shells use (`env_include.rs`: the Unix `bash`
builder shared by `baseline_env` and `attempt_env`, and the two Windows `powershell.exe` builders)
remove the same variables before running. The diff therefore sees a script-set identity variable as
*added*, not as *unchanged*, which is what makes the last-but-one row of §2 hold.

## 4. Not changed

The client's own environment, and the environment it passes to a daemon it spawns. The container's
environment as created by feature 027. `TERM`, which stays pinned. No variable is ever *added* by
this feature.
