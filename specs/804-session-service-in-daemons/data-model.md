# Data model

No persisted type changes.

- **Daemon** (`micold_core::daemons::DaemonEntry`): `runtime: DaemonRuntime`; `Container` carries
  `ContainerSettings { profile: SandboxProfile, container_name, port }`. `SandboxProfile` fields
  (runtime, image, budget, network, credentials, survive_logout) are exactly the page's fields.
- **Form draft** (`features::daemons::Draft`, UI-only): adds `image_path`, `cpus`, `memory_mib`,
  `pids`, `storage_mib` (typed text; empty means unset, rule RB-2) beside `profile`.
- **Legacy block** `Settings::daemon: DaemonConfig`: no longer user-edited; synced from the legacy
  default entry by `Settings::sync_legacy_daemon` on a form save.
- **Removed** (UI-only): `SettingsSection::Daemon`, the page's `DaemonDraft` text mirrors and
  `PendingPlacementChange` once unreferenced.
