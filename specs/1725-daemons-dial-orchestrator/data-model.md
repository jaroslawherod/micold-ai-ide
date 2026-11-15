# Data model: Daemons dial the orchestrator

## `[orchestrator_link]` (`micold_core::daemon_config::OrchestratorLink`)

```toml
[orchestrator_link]
enabled = true                       # default false
socket  = "/run/user/1000/micold/orchestrator.sock"   # optional
```

`#[serde(deny_unknown_fields)]`; `DaemonConfig.orchestrator_link: Option<OrchestratorLink>`.

An unknown key or a wrong type is the existing schema pass's finding (as for every table). The one new rule:

| Rule | Finding |
|---|---|
| L1 | `orchestrator_link.socket` is empty |

`[orchestrator_link]` and `[orchestrator]` are independent: no rule of O1-O5 reads this table and L1 reads
nothing of that one.

Effective endpoint: `socket` if set, else `endpoint::orchestrator_endpoint()` for kind `host`, else the same
file name beside `resolve_pinned()`'s daemon socket for `ssh`, `container`, `kubernetes`.

## `LinkStatus` (`micold_core::orchestrator::dial`, pure)

| State | Entered when | Logged |
|---|---|---|
| `NotLinked` | start (enabled), a lost link, an absent endpoint | on change only |
| `Linking` | a dial begins and the endpoint accepted | no |
| `Linked` | `Welcome` read | yes |
| `Refused(reason)` | `LinkVerdict::Refused` (`VersionMismatch`, `AlreadyLinked`) | yes, with the orchestrator's message |
| `HelloRefused(detail)` | `DaemonMsg::Refused` after `Accepted` | yes |

`LinkStatus` is also the `link` object of `status.json`: `{ "state": "linked", "since": <unix s>, "detail":
"<refusal message or empty>" }`; absent when `[orchestrator_link]` is not enabled. `STATUS_VERSION` is unchanged
(the field is additive, `#[serde(default, skip_serializing_if = "Option::is_none")]`).

## `RetryPolicy`

Holds one `Backoff`. `after_failure()` -> next delay; `after_refusal()` -> `MAX_DELAY` (+jitter);
`on_linked()` -> reset; `after_loss()` -> `reset()` then `next_delay()` (1 s, and the next failure gives 2 s) (+jitter). Pure; the jitter unit is an
argument.

## `LinkForward` (`micold_core::ssh::argv`)

| Field | Meaning |
|---|---|
| `remote_socket: String` | the path printed by `remote prepare-link`, in the remote's owner-only runtime dir |
| `local_socket: PathBuf` | `endpoint::orchestrator_endpoint().socket_path` on the holder's machine |

Renders `-R <remote_socket>:<local_socket>`; both paths are validated (absolute, no `:` , no control
character, no whitespace) at construction so no argv text is built from unchecked input. Carried by
`ProvisionOptions.link: Option<LinkForward>`.

## Link lifecycle (dialer side)

```text
NotLinked --connect ok--> Linking --Accepted+Welcome--> Linked
    ^  ^                     |                             |
    |  +--- refused/closed --+--- lost / deadline ---------+
    +-- delay (RetryPolicy) --+
```
