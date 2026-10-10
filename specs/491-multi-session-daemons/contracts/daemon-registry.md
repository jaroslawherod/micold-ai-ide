# Contract: persisted registry and bindings

## settings.json (additive; no `settings_version` bump)
```json
{ "daemons": [
    {"id":1,"name":"Host","runtime":{"kind":"host"},"auto_start":true},
    {"id":2,"name":"Untrusted","runtime":{"kind":"container","profile":{...SandboxProfile...},
      "container_name":"micold-sandbox-untrusted","port":7728},"auto_start":false}
  ],
  "legacy_default_daemon": 1,
  "next_daemon_id": 3,
  "daemon": { "placement":"host_process", "sandbox":{...} } }
```
- `daemons` absent: synthesise one entry from `daemon` and set `legacy_default_daemon` (R5).
  Present: authoritative, never re-migrated.
- `daemon` is still written (mirrors the first entry) for one release; unknown fields ignored.
- Unknown `kind` loads as `DaemonRuntime::Unsupported`, shown "unsupported runtime", kept verbatim on save (forward
  compatibility with #687/#688).
- Writes use the existing atomic `write_then_rename`; a corrupt file follows the existing `.bak`
  path unchanged.

## projects/<id>.json
New optional `bindings: { "<worktree dir_name>": <daemon id>, "": <daemon id for Default> }`.
Absent/omitted when empty. A binding naming an unknown id reads as `NoDaemon` (ids are never reused, so a removed daemon's bindings cannot attach to a later daemon); an absent one reads as
`legacy_default_daemon` when it is still registered.

## Invariants
1. Names unique (case-insensitive); ids unique and never reused; container names and ports unique.
2. At most one `host` entry (the host daemon is a per-user singleton endpoint).
3. Removing an entry never writes to, nor deletes, worktree directories.
