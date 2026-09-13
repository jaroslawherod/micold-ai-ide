# Contract: `micold-core::install_location`

**Feature**: [../spec.md](../spec.md) | Normative for FR-019 and its tests.

## API

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallLocation {
    Installed,
    MountedImage,
    Translocated,
}

/// Classify where an executable is running from. Pure: no filesystem access, no OS calls.
pub fn classify(executable: &Path) -> InstallLocation;

/// Convenience for the boot path: `classify(&std::env::current_exe()?)`, `Installed` on error.
pub fn current() -> InstallLocation;
```

## Rules

| Input | Result |
|---|---|
| any path containing a component `AppTranslocation` | `Translocated` |
| otherwise, a path beginning `/Volumes/` | `MountedImage` |
| anything else, including relative and empty paths | `Installed` |
| any path, when `cfg!(not(target_os = "macos"))` | `Installed` |

Translocation is checked first: a translocated copy of a quarantined app can carry either shape, and
its message is the more specific one.

`current()` never fails — an unreadable `current_exe()` yields `Installed`, because refusing to run
on an inconclusive answer would be a worse failure than the one being prevented.

## Consumer behaviour

The client calls `current()` once at boot. On anything but `Installed` it shows a single screen
instead of the session UI, stating in one sentence that the application is running from the
downloaded disk image rather than from where it was installed, and pointing at the drag-to-
Applications step. It does not continue, and it offers no dismissal — FR-019's "MUST NOT run in a
way that looks installed".

The decision lives in the reducer and is unit-tested there; only the iced view is glue.

## Accepted false positive

An app genuinely installed on an external volume mounted under `/Volumes/` classifies as
`MountedImage` and is asked to be installed properly. Recorded in research R9 as a deliberate trade:
a clear, actionable false positive beats an unclear false negative, and no reliable path-only signal
separates a mounted disk image from a mounted external disk.
