# Contract: the `PATH` a sandboxed spawn gets

Feature 1289, FR-001 to FR-008. The order is computed by
`micold_core::sandbox::toolchain_path` (NEW) and applied in
`DaemonState::with_offered_path` (renamed `with_sandbox_path`). Every spawn and the availability
check go through `spawn_env_for`.

## P1. When it applies

The rule applies only when the daemon has an `offered.json`, that is, an app-owned container or pod
(research R2). Without one, `PATH` is today's: on host and remote daemons it is not touched
(FR-008).

## P2. Order

```text
<offered CLI bin 1> : … : <offered CLI bin n>      # unchanged from today, offered.json order
$HOME/.local/bin
<mise shims>                                       # MISE_DATA_DIR/shims, else $XDG_DATA_HOME/mise/shims, else $HOME/.local/share/mise/shims
<PATH after environment-include, or the daemon's own>
```

- Variables are read from the resolved session environment first, then from the daemon process.
  An empty value counts as unset.
- An entry already present earlier is dropped from its later position, compared as the exact path
  text (FR-004). So `$HOME/.local/bin` already in the image `PATH` moves to second place.
- Folders are added whether or not they exist (FR-003). A missing folder produces no notice.
- Without `HOME`, the two home folders are left out and the rest of the order stands.

## P3. Examples

| Input | `PATH` out |
|---|---|
| bins `[/s/ai-clis/claude/bin]`, `HOME=/home/u`, nothing else set, rest `/usr/local/bin:/usr/bin:/bin` | `/s/ai-clis/claude/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/local/bin:/usr/bin:/bin` |
| same, `MISE_DATA_DIR=/opt/mise` | `…:/home/u/.local/bin:/opt/mise/shims:/usr/local/bin:…` |
| same, `XDG_DATA_HOME=/x`, no `MISE_DATA_DIR` | `…:/home/u/.local/bin:/x/mise/shims:…` |
| rest `/home/u/.local/bin:/usr/bin` | `/s/ai-clis/claude/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/bin` |
| bins `[]` (offered.json lists none) | `/home/u/.local/bin:/home/u/.local/share/mise/shims:<rest>` |

## P4. Consumers

`ai_cli_env_for` (AI CLI launch, and through it every MCP server: FR-005), `env_include_vars_for`
(regular terminals and shells), the setup command's environment (FR-011), and the availability
check (`availability_in`, `ai_clis_available_in`) (FR-007). None of them computes `PATH` itself.
