# Data model: Container daemons provisioned over SSH (806)

Types this feature adds or changes. `NEW` marks a new type; paths are under `crates/`. Field names
are the planned Rust names; the settings JSON shape is in
[contracts/daemon-settings.md](./contracts/daemon-settings.md).

## ImageChoice (`NEW`, `micold-core/src/sandbox/image.rs`) — FR-001, FR-004

```text
enum ImageChoice {
    Basic,                         // the pinned basic image (BASIC_IMAGE)
    Own(OwnImage),
}
enum OwnImage {
    Reference(String),             // parsed by the existing ImageRef::parse
    Archive(PathBuf),              // loaded with the runtime's `load`
}
```

- Exactly two user-facing options (FR-001); `Basic` is `Default` (new daemon).
- `Own` validation reuses `ImageRef::parse` and the archive-exists check of today's
  `ImageSource::validate`; an empty reference with no archive is unrepresentable in `OwnImage`, so
  the form's draft (`OwnImageDraft {reference: String, archive: Option<PathBuf>}`) is what fails
  validation, naming the field (US3 #4).
- `ImageChoice::reference()` → the string the runtime pulls or the container records: `BASIC_IMAGE`
  for `Basic`.
- Replaces `ImageSource` in `SandboxProfile`. `ImageSource`, `ImageSourceKind`, `DEFAULT_IMAGE`,
  `DEV_IMAGE_TAG` and `refuses_fingerprint_mismatch` remain only as the deserialisation input of
  the migration (`ImageSource::into_choice`), then are removed from every other use.

## BasicImage (`NEW`, `micold-core/src/sandbox/basic_image.rs`) — FR-002, FR-003

```text
const BASIC_IMAGE_REPOSITORY: &str = "ghcr.io/jaroslawherod/micold-basic";
const BASIC_IMAGE_CONTENT_HASH: &str = "<16 hex>";         // scripts/basic-image-hash.sh
const BASIC_IMAGE: &str = "ghcr.io/jaroslawherod/micold-basic:<hash>@sha256:<digest>";
const RELEASED_IMAGE_REPOSITORIES: [&str; 2] = [DEFAULT_IMAGE_REPOSITORY, RETIRED_IMAGE_REPOSITORY];
fn is_released_image(reference: &str) -> bool;              // repository match, any tag/digest
```

- `BASIC_IMAGE` always carries a digest (unit test: `ImageRef::parse(BASIC_IMAGE).digest.is_some()`).
- `DEV_BASIC_IMAGE = "micold-basic:dev"`: the local build used through "My own image".

## AiCliSelection (`NEW`, `micold-core/src/sandbox/mod.rs`) — FR-010

```text
struct AiCliSelection(BTreeSet<AiCli>);
impl AiCliSelection { fn new_daemon() -> Self /* {ClaudeCode} */; fn all() -> Self /* AiCli::ALL */ }
```

- Persisted per container daemon in `SandboxProfile.ai_clis`. May be empty (edge case "Selection
  empty").
- `SandboxProfile` deserialised without `ai_clis` (a pre-upgrade file) gets `all()`; a profile
  created by the form gets `new_daemon()` (D3). The serde default is therefore `all()` and the
  "add daemon" path sets `new_daemon()` explicitly.
- Not present for `DaemonRuntime::Ssh` (US6 #3): the field lives on the container profile only.

## SandboxProfile (changed, `micold-core/src/sandbox/mod.rs`)

| Field | Change |
|---|---|
| `image: ImageSource` | becomes `image: ImageChoice` (serde: reads old `image`, writes `image_choice`) |
| `ai_clis` | `NEW: AiCliSelection` |
| `budget`, `network`, `credentials`, `survive_logout`, `runtime` | unchanged (FR-014, FR-015) |

## MigrationNotice (`NEW`, `micold-core/src/daemons.rs`) — FR-016, FR-017

```text
enum MigrationNotice {
    MovedOffReleasedImage,                 // stored on ContainerSettings until a basic-image start succeeds
}
enum LegacyContainer { RunningOnReleasedImage }   // runtime fact, not stored; from Adoption::Legacy
```

- `ContainerSettings.migration: Option<MigrationNotice>` (persisted). Set by the read-repair,
  cleared (and saved) by the first `Started` on `ImageChoice::Basic`.
- The row text: stored notice → "Moved off the released daemon image; it moves to the basic image
  on its next start." Runtime `LegacyContainer` → "Still running on the released daemon image;
  restart it with no sessions running to move it to the basic image." with the restart enabled
  only when the daemon reports zero running sessions.

## ContainerSettings (changed, `micold-core/src/daemons.rs`)

| Field | Change |
|---|---|
| `profile`, `container_name`, `port` | unchanged |
| `migration` | `NEW: Option<MigrationNotice>`, `#[serde(default)]` |

## DaemonKeys (`NEW`, `micold-core/src/provision/keys.rs`) — FR-006

```text
struct DaemonKeys { dir: PathBuf }  // <daemon state dir>/ssh
  client_key()   -> dir/client_ed25519       (0600 / owner-only)
  client_pub()   -> dir/client_ed25519.pub
  host_key()     -> dir/host_ed25519         (0600 / owner-only; mounted read-only)
  host_pub()     -> dir/host_ed25519.pub
  authorized()   -> dir/authorized_keys      (mounted read-only)
  known_hosts()  -> dir/known_hosts          (rewritten per port)
fn keygen_argv(platform, path) -> SshCommand   // ssh-keygen -t ed25519 -N "" -q -f <path>
fn ensure(dir, runner) -> Result<DaemonKeys, ProvisionFailure>  // creates once, never overwrites
```

- One per container daemon (Principle II: never shared between daemons).

## Reach (`NEW`, `micold-core/src/ssh/argv.rs`) — FR-005, FR-006, FR-008

```text
enum Reach {
    UserConfig(SshTarget),                 // 489: the user's ssh config, as today
    Pinned(PinnedReach),                   // container / 686 pod: -F none, own key and known_hosts
}
struct PinnedReach { host: String /* "127.0.0.1" */, port: u16, user: String, key: PathBuf, known_hosts: PathBuf }
```

- `argv::command` takes `&Reach`; `UserConfig` produces today's argv byte for byte (489 unit tests
  stay green unchanged).

## ProvisionTarget (`NEW`, `micold-core/src/provision/mod.rs`) — FR-008

```text
struct ProvisionTarget {
    reach: Reach,
    owned: Ownership,                      // AppOwned { ai_clis: AiCliSelection, state_dir: String, egress: Egress } | UserHost
    bounds: StepBounds,
}
enum Egress { Direct, Relay { allow: Vec<HostPort> } }   // R8
struct StepBounds { reach, probe, copy: Option<Duration>, install, start, version: Duration }
```

## Step, ProvisionEvent, ProvisionFailure, ProvisionWarning (`NEW`, `micold-core/src/provision/`) — FR-018, FR-019, FR-021

```text
enum Step { Acquire, CreateContainer, Reach, Probe, Copy, InstallAiClis, Start, VersionCheck }
enum ProvisionEvent { Entered(Step), CopyPercent(u8), InstallLine(String), InstallDone(AiCli, Result<(), String>) }
struct ProvisionFailure { step: Step, cause: FailureCause, output: String }
enum FailureCause {
    PullOrLoad, ContainerStart, NoSshServer, AuthRefused, CopyFailed,
    NoMatchingBuild { os, arch }, DaemonStart, VersionMismatch { found, expected },
    TimedOut { bound: Duration }, Cancelled, PostureBlocksDownload, Ssh(SshFailure),
}
struct ProvisionWarning { cli: AiCli, output: String }       // FR-012: install failed, start continues
```

- `Acquire` and `CreateContainer` are the container runtime's steps, reported in the same `Step`
  vocabulary so the row shows one list (US1 #3); the shared sequence starts at `Reach`.
- `ProvisionFailure::reason()` → "<step label> failed: <cause text>"; the 489 `SshFailure` texts are
  reused unchanged through `FailureCause::Ssh`.

## Adoption (changed, `micold-core/src/sandbox/lifecycle.rs`) — FR-017

| Variant | Change |
|---|---|
| `Create`, `Attach`, `Start`, `Replace` | unchanged |
| `Legacy(ContainerId)` | `NEW`: running container on a released image; attach over TCP+token, do not replace |
| `StaleReason::StaleBuild` | no longer produced (fingerprint check dropped, R10) |

## SandboxSpec (changed, `micold-core/src/sandbox/mod.rs`)

- Publishes `127.0.0.1:<port>` → container `2222` (was `7727`).
- Adds the read-only `SshMount { host: DaemonKeys.dir subset, container: /run/micold/ssh }`.
- Drops the token `SecretMount` and `MICOLD_LISTEN_ADDR` for basic/own images; sets `MICOLD_HOME`
  for the entrypoint (R3).

## AttemptLedger and CancelToken (`NEW`, `micold-core/src/provision/attempt.rs`) — FR-019, FR-020

```text
struct CancelToken(Arc<(AtomicBool, Notify)>);
struct AttemptLedger { created_container: Option<ContainerId>, started_daemon: bool }
fn rollback(ledger, runtime) // remove what this attempt created; never an adopted container
```

## State transitions (container daemon start)

```text
stopped ─start→ starting[Acquire → CreateContainer → Reach → Probe → (Copy) → (InstallAiClis) → Start → VersionCheck]
starting ─all ok→ connected            (warnings listed; migration notice cleared on Basic)
starting ─step fails / bound passes→ unreachable(ProvisionFailure)   (rollback, recent errors)
starting ─user stops→ stopped          (CancelToken, rollback)
running legacy ─restart, 0 sessions→ starting (Replace with Basic)
```

## Daemon-side file (`NEW`, written by provisioning, read by `micold-daemon`) — FR-012

`/var/lib/micold-ai-ide/ai-clis/offered.json`: `{"version":1,"clis":[{"cli":"ClaudeCode","bin":
"/var/lib/micold-ai-ide/ai-clis/ClaudeCode@2.1.247/bin"}]}`. Missing file → no extra `PATH`
entries (host daemons, remote daemons). Read on each availability query; malformed → ignored and
logged.
