# Phase 1 Data Model: macOS Package

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-31

This feature's "entities" are mostly build-time artifacts rather than runtime state. Two are runtime
types in `micold-core`; the rest are file structures with validation rules, which is where their
correctness actually lives.

---

## 1. `InstallLocation` (runtime, `micold-core::install_location`)

Where the running executable lives, as far as it concerns whether the app should run at all.

| Variant | Meaning | Produced when |
|---|---|---|
| `Installed` | A normal, persistent location | Default; always off macOS |
| `MountedImage` | Running from the delivery container the user has not installed | The executable path starts with `/Volumes/` |
| `Translocated` | macOS relocated a quarantined copy to a randomised read-only path | Any path component equals `AppTranslocation` |

**Rules**
- Total over any `&Path`, including relative and empty paths → `Installed`.
- Classification is by path only: no filesystem access, no macOS API, no `cfg` branch. This is what
  keeps it testable on all three platforms (research R9).
- Order matters: a translocated path can also sit under a mounted volume; `Translocated` wins,
  because its message is the more specific one.

**Transitions**: none. Computed once from `std::env::current_exe()` at boot and held for the
process's life — a running executable does not move.

**Consumer**: the client's boot path. Anything but `Installed` replaces the session UI with the
install-me screen (FR-019); nothing else in the app branches on it.

## 2. `PermissionFailure` (runtime, `micold-core::permission_failure`)

A file-access error the operating system, not the file mode, caused.

| Field | Type | Meaning |
|---|---|---|
| `location` | `ProtectedLocation` | Which protected folder the path fell under |
| `path` | `PathBuf` | The path that was denied, for the message |

`ProtectedLocation` ∈ { `Documents`, `Desktop`, `Downloads`, `RemovableVolume`, `NetworkVolume` } —
the same five the plist declares purpose strings for (research R7). One list, two consumers: a
location with no purpose string would produce a message telling the user to grant something the
system never offers to grant, so a test asserts the enum and the plist agree.

**Rules**
- Classification is `classify(err: &io::Error, path: &Path) -> Option<PermissionFailure>`.
- `Some` only when the error is `ErrorKind::PermissionDenied` **and** the path lies under a known
  protected location. A permission error anywhere else stays a plain error — over-attributing is its
  own kind of wrong message.
- `None` on every non-macOS platform, so no other platform's error text changes.

**Consumer**: the client, when opening a project or listing a directory fails (FR-028).

## 3. Application bundle (build artifact)

See [contracts/bundle-layout.md](./contracts/bundle-layout.md) for the normative structure.

| Part | Source | Validation |
|---|---|---|
| `Contents/MacOS/micold-ai-ide` | `cargo build --release -p micold-client` (universal via `lipo`) | Present, executable, `CFBundleExecutable` names it |
| `Contents/MacOS/micold-daemon` | `cargo build --release -p micold-daemon` | Present, executable, **sibling** of the above |
| `Contents/Resources/icon.icns` | `assets/icon/icon.icns` | Present; `CFBundleIconFile` resolves to it |
| `Contents/Info.plist` | `packaging/macos/Info.plist.in` + version | Parses (`plutil -lint`); every key in R7 present; `@VERSION@` fully substituted |
| `Contents/PkgInfo` | literal | `APPL????` |
| — | — | **No third executable.** `micold-showcase` must appear nowhere |

**Invariant that carries the feature**: exactly two files in `Contents/MacOS/`, named above. More
means something globbed; fewer means sessions cannot start.

## 4. Delivery container (build artifact)

| Part | Value |
|---|---|
| Format | UDZO (compressed, read-only) |
| Volume name | `Micold AI IDE` |
| Contents | the `.app`, plus a symlink `Applications` → `/Applications` |
| File name | see §5 |

**Rules**: produced only by the release path and by `mise run dmg`; never required for the per-PR
gate (FR-036). Mounting it and running the app from there is a state the app refuses (§1).

## 5. Release artifact set

| Artifact | Produced by | Attached to |
|---|---|---|
| `micold-ai-ide_<version>_amd64.deb` | `deb (amd64)` job | draft release |
| `micold-ai-ide_<version>_arm64.deb` | `deb (arm64)` job | draft release |
| `MicoldAIIDE-<version>-universal.dmg` | `macos` job | draft release |

**Rules**
- The version in all three equals `[workspace.package] version`, which release-please bumps.
- The macOS name states the version and that one download serves both Macs — the `universal` token
  is what removes the choice FR-020 forbids putting in front of the user.
- The set is complete or the release is not published: `publish` needs every producing job
  (FR-011). See [contracts/release-artifacts.md](./contracts/release-artifacts.md).

## 6. Version identity

One source, three readers — the thing FR-003 is about:

```text
[workspace.package] version  ──┬──> CFBundleShortVersionString / CFBundleVersion (Info.plist)
  (bumped by release-please)   ├──> env!("CARGO_PKG_VERSION") -> the About dialog
                               └──> the release tag and every artifact file name
```

**Validation**: the bundle script substitutes from `cargo metadata`, never from a literal, so drift
requires deleting the substitution rather than forgetting to update a copy.
