# Contract: provisioning over SSH (shared by containers, 489 remote hosts and 686 pods)

The one start mechanism of FR-008. A runtime brings up whatever runs sshd (a container, a pod, a
host the user owns), then hands this sequence a **target**; the sequence does the rest. Code:
`crates/micold-core/src/provision/` (`NEW`); command lines: `crates/micold-core/src/ssh/argv.rs`.
This contract supersedes the "Attempt sequence" of
`specs/489-ssh-remote-daemon/contracts/ssh-transport.md`; 489's command lines, helper and reason
table stay as written there.

## What a runtime supplies (`ProvisionTarget`)

| Input | Container (806) | Remote host (489) | Pod (686, to adopt) |
|---|---|---|---|
| `reach` | `Reach::Pinned { 127.0.0.1, <stored port>, <uid name>, DaemonKeys }` | `Reach::UserConfig(SshTarget)` | `Reach::Pinned { 127.0.0.1, <port-forward local port>, … }` |
| `owned` | `AppOwned { ai_clis, state_dir: /var/lib/micold-ai-ide, egress }` | `UserHost` | `AppOwned { … }` |
| `egress` | `Relay` when posture is `NoOutbound`, else `Direct` | — | `Relay` or `Direct`, the runtime's choice |
| `bounds` | reach 120 s, probe 120 s, copy 120 s, install 600 s total, start 120 s, version 120 s | connect deadline (`ConnectTimeout`, default 15 s) for reach/probe/start, copy unbounded with progress (489 R3) | runtime's choice |

Plus the shared inputs: an `SshHost` (`SshRunner` + `Artefacts`, today `SystemSsh`), `ProvisionOptions`
(platform, client arch, build id, client build), a `CancelToken`, and an event sink
`Fn(ProvisionEvent)`.

A runtime supplies nothing else: no step of its own runs inside the sequence. 686 implements only
`reach` (its port-forward) and `owned`.

## The sequence

Each step emits `ProvisionEvent::Entered(step)` first, runs under its bound, and checks the
`CancelToken` before it starts and while it waits on a child (cancel kills the child).

| # | Step (`Step`, label shown) | What it runs | Skipped when |
|---|---|---|---|
| 1 | `Reach` — "connecting to the SSH server" | `ssh … true` (a `UserHost` skips the command: the probe is its first connection, but the step is still entered), retried every 500 ms while the failure is `ConnectionRefused` / connection reset / `kex_exchange_identification` (sshd not up yet); any other failure is final | never |
| 2 | `Probe` — "checking the daemon build" | 489 probe script (`sh -s` on stdin) → `os`, `arch`, `home_writable`, `have` | never |
| 3 | `Copy` — "copying the daemon" | 489 copy, bytes on stdin, atomic rename; emits `CopyPercent`; re-probes | `have` |
| 4 | `InstallAiClis` — "installing the AI CLIs" | fixed install script (`sh -s`, below) under `flock`; emits `InstallLine`, `InstallDone(cli, result)` | `UserHost`, or nothing selected is missing |
| 5 | `Start` — "starting the daemon" | writes `$HOME/.micold/remote/last` (app-owned only), opens `micold-daemon remote bridge`, handshake | never |
| 6 | `VersionCheck` — "checking the daemon version" | `Connected::Refused` → `Entered(VersionCheck)` and `Ok(Provisioned { connected: Refused })`; the caller applies the 489 FR-011 rule (explain; replace only with no running sessions) | handshake accepted |

Steps the runtime runs before handing over (`Acquire` — "getting the image", `CreateContainer` —
"starting the container") use the same `Step` type and event sink, so the row shows one list.

### Install script (step 4, app-owned targets only)

Inputs substituted from the pinned table (`provision/ai_cli.rs`), never from user text:
`<state>`, and per missing CLI `<cli>`, `<package>`, `<version>`. For each: `npm install -g
--prefix <state>/ai-clis/<cli>@<version>.part <package>@<version>`, on success `mv` to
`<state>/ai-clis/<cli>@<version>`, on failure `rm -rf` the `.part` and print
`MICOLD_INSTALL <cli> failed`. Finally it writes `<state>/ai-clis/offered.json` (selected ∩
installed) atomically. `npm` missing → each CLI fails with "the image has no npm (Node.js), which
this AI CLI's installer needs". With `Egress::Relay`, the command carries
`-R 127.0.0.1:17728:127.0.0.1:<relay port>` and the script exports `HTTPS_PROXY`,
`npm_config_https_proxy` = `http://127.0.0.1:17728`.

### Install relay (`Egress::Relay`)

A client-loopback HTTP `CONNECT` proxy started for one install command and stopped when it ends.
It answers `200` only for a `CONNECT host:port` in `allow` (from the pinned table:
`registry.npmjs.org:443`), `403` for anything else, and carries bytes opaquely (TLS stays end to
end). It never accepts a non-`CONNECT` request.

## Outcomes

- `Ok(Provisioned { connected, warnings: Vec<ProvisionWarning>, copied: bool })`. `warnings` holds
  each CLI whose install failed, with its output (FR-012: the start still succeeds).
- `Err(ProvisionFailure { step, cause, output })`. The runtime turns it into `unreachable` with
  `reason()` and adds it to the daemon's recent errors, then rolls back what it created (FR-020).

## Failure causes and reason texts (tests assert these)

`reason(&ReasonContext)` = `"<step label> failed: <cause text>"`, followed by `output` (tool output, last 40
lines) where the cause has one.

| Cause | Cause text | Typical step |
|---|---|---|
| `PullOrLoad` | `the container runtime could not get image <ref>` | Acquire |
| `ContainerStart` | `the container runtime could not start the container` | CreateContainer |
| `NoSshServer` | `the image has no reachable SSH server; an image of your own must run one (see the user guide)` | Reach, after its bound with only "not up yet" failures |
| `AuthRefused` | `the container's SSH server refused the daemon key` | Reach |
| `CopyFailed` | `the daemon build could not be written in the target` | Copy |
| `NoMatchingBuild { os, arch }` | `no matching daemon build for <os>-<arch>` | Probe |
| `DaemonStart` | `the daemon did not start` | Start |
| `VersionMismatch { found, expected }` | 489 FR-011 text | VersionCheck |
| `TimedOut { bound }` | `did not finish within <bound>` | any |
| `Cancelled` | `stopped by you` | any |
| `PostureBlocksDownload` | `the daemon's network setting blocks downloads and the install relay could not be opened` | InstallAiClis |
| `Ssh(SshFailure)` | 489's reason table, unchanged | Reach, Probe, Copy, Start (489 target) |

An install failure of one CLI is a `ProvisionWarning`, not a cause: `"<CLI name> was not installed:"`
+ installer output.

## Invariants

1. Copy only when absent; a second start with nothing new copies and downloads nothing (SC-003,
   US6 #2).
2. Nothing is installed on a `UserHost` target (US6 #3).
3. Same step labels, progress events and reason texts for every target (US6 #1).
4. No step retries silently: only `Reach` repeats, and only on "not up yet" failures, inside its
   bound.
5. Pinned reach never reads the user's SSH configuration, keys or agent (`-F none`,
   `IdentitiesOnly=yes`, `IdentityAgent=none`) and never writes a `known_hosts` but its own.
6. The sequence holds no lock across targets: provisioning one daemon touches only its target and its
   own key directory (Principle II, FR-022).
