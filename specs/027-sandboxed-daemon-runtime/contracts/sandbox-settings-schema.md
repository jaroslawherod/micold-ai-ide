# Contract: Settings Schema v3 → v4

**Feature**: [../spec.md](../spec.md) | **Extends**:
[`specs/003-material-design-layout/contracts/settings-schema.md`](../../003-material-design-layout/contracts/settings-schema.md)

The durable on-disk shape of `settings.json` after this feature. It follows the existing contract's
rules rather than inventing new ones — v2 added `scrollback_lines` and v3 added the
environment-include fields, both by the same missing-field-defaults route, and v4 adds one nested
object the same way.

## Version

`SETTINGS_VERSION` moves `3` → `4` in `crates/micold-core/src/settings.rs`.

## Added

One field on the root document:

```jsonc
{
  "settings_version": 4,
  "theme": "System",
  "scrollback_lines": 10000,
  "env_include_enabled": true,
  "env_include_script_path": "/home/u/.bashrc",
  "env_include_timeout_secs": 10,

  "daemon": {
    "placement": "host_process",         // "host_process" | "local_sandbox"
    "sandbox": {
      "runtime": "docker",               // "docker" | "podman"
      "image": {
        "kind": "registry",              // "registry" | "imported_file" | "local_build"
        "reference": "ghcr.io/<org>/micold-daemon:<version>",
        "path": null                     // set only when kind = "imported_file"
      },
      "budget": {
        "cpus_milli": 2000,              // null = runtime default
        "memory_bytes": 4294967296,
        "pids": 512,
        "storage_bytes": null
      },
      "network": "no_outbound",          // "no_outbound" | "outbound"
      "credentials": [],                 // subset of git_config|ssh_agent|git_credentials|ai_cli_auth
      "survive_logout": false
    }
  }
}
```

Nested rather than flattened with a `sandbox_` prefix because the existing flat fields grew one
feature at a time and the root is already six keys wide; the sectioned Settings view (FR-026) makes
the grouping user-visible, and matching it on disk keeps the two readable together.

## Rules

**S-1 — Every added field has a serde default.** A v3 document read by v4 code yields
`placement: host_process` and a default `sandbox` block. No migration step, no rewrite on read; the
file is rewritten only when the user next saves. This is the existing contract's rule and the reason
v2 and v3 needed no migration code either.

**S-2 — Missing means default, not absent.** `daemon` missing entirely, `daemon.sandbox` missing, or
any single leaf missing all resolve to the documented default. Partial documents are normal, not
corrupt.

**S-3 — `credentials` defaults to the empty array (FR-004a/b).** Not to a "sensible" set. A user who
never opened the sandbox section shares nothing, and an unrecognised entry in the array is dropped
with a log line rather than failing the load.

**S-4 — `placement` defaults to `host_process`.** Upgrading the app never moves a user into the
sandbox. Enabling it is an explicit act (FR-001).

**S-5 — Unknown fields are preserved on read and rewritten on save.** A settings file written by a
newer build and read by an older one does not lose the newer build's keys. (New in v4; the flat
schema never needed it.)

**S-6 — Corrupt or unreadable degrades to defaults, never crashes.** Unchanged from the base
contract and from `store.rs`'s existing behaviour (Principle IV).

**S-7 — Ranges are clamped on read, not rejected.** Out-of-range budget values clamp into the
supported range and report the clamp, following `clamp_scrollback` and `clamp_env_include_timeout`
(RB-1). A hand-edited file with `"pids": 1` opens the app with a corrected value and a note, not an
error dialog.

**S-8 — Writes stay atomic.** Temp file plus rename, as `JsonFileSettingsStore` already does. A
crash mid-save cannot leave a half-written sandbox profile.

**S-9 — Storing a limit the current runtime cannot enforce is legal.** Reconciliation is a runtime
concern, not a persistence one (RC-3), so switching runtimes back restores the user's intent.

## Not stored

The **authentication token** (R1) is not in `settings.json`. It lives in the per-user state directory
at `0600`, is regenerated per sandbox start, and is mounted read-only into the container. Settings
are a document a user may reasonably copy between machines or paste into a bug report; a secret must
not travel with it.

Probed `RuntimeCapabilities` are also not stored here — they are a cache keyed by runtime version
(R10), and belong with other derived state, not with user intent.

## Test obligations

| # | Check | Rule |
|---|---|---|
| T-1 | a verbatim v3 document loads, yields `host_process` + default sandbox, `settings_version` reads as 4 after save | S-1 |
| T-2 | `daemon` absent / `daemon.sandbox` absent / each leaf absent each resolve to the documented default | S-2 |
| T-3 | `credentials` absent yields the empty set; an unknown entry is dropped, the rest survive | S-3 |
| T-4 | an unknown root key survives a load/save round-trip | S-5 |
| T-5 | truncated JSON yields defaults with a recovery status, not an error | S-6 |
| T-6 | out-of-range budget values clamp and report | S-7 |
| T-7 | the full v4 document round-trips byte-stably | — |
| T-8 | no serialised form of the token appears anywhere in the written file | Not stored |

## Casing, corrected during implementation

The examples above originally showed PascalCase variants (`"HostProcess"`, `"Registry"`). The
durable settings document is snake_case — `theme` established that in feature 003's contract — and
one file with two casings is a trap for whoever hand-edits it next. The new enums carry
`#[serde(rename_all = "snake_case")]` to match, and the examples above are the corrected form.
