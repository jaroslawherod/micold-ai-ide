# Contract: macOS application bundle

**Feature**: [../spec.md](../spec.md) | Normative for `scripts/macos-bundle.sh` and every test that
reads a produced bundle.

## Structure

```text
micold-ai-ide.app/
└── Contents/
    ├── Info.plist
    ├── PkgInfo                       # exactly: APPL????
    ├── MacOS/
    │   ├── micold-ai-ide             # 0755
    │   └── micold-daemon             # 0755
    └── Resources/
        └── icon.icns
```

`Contents/MacOS/` contains **exactly these two files**. This is not a stylistic rule:

- A missing `micold-daemon` makes `micold_core::spawn::daemon_binary()` fall through to a bare
  `micold-daemon` on `PATH`, which a packaged install does not have — every session then fails to
  start while the window opens normally.
- A third file is evidence of a glob, and the file it would be is `micold-showcase` (FR-006).

## `Info.plist` keys

Required, all of them, with `@VERSION@` substituted before the bundle is signed:

| Key | Type | Value |
|---|---|---|
| `CFBundleIdentifier` | string | `io.github.cumulocity-iot.micold-ai-ide` |
| `CFBundleName` | string | `Micold AI IDE` |
| `CFBundleDisplayName` | string | `Micold AI IDE` |
| `CFBundleExecutable` | string | `micold-ai-ide` |
| `CFBundleIconFile` | string | `icon` |
| `CFBundleShortVersionString` | string | the workspace version |
| `CFBundleVersion` | string | the workspace version |
| `CFBundlePackageType` | string | `APPL` |
| `LSMinimumSystemVersion` | string | `15.0` |
| `NSHighResolutionCapable` | true | — |
| `NSDocumentsFolderUsageDescription` | string | one sentence, user's terms |
| `NSDesktopFolderUsageDescription` | string | one sentence |
| `NSDownloadsFolderUsageDescription` | string | one sentence |
| `NSRemovableVolumesUsageDescription` | string | one sentence |
| `NSNetworkVolumesUsageDescription` | string | one sentence |

Forbidden: `LSUIElement` (this is a windowed app), and any key naming an Apple team, certificate, or
notarization state (FR-016 — a contributor's build must be equivalent).

## Signature

- Ad-hoc (`--sign -`), hardened runtime enabled, entitlements from
  `packaging/macos/entitlements.plist`.
- `Contents/MacOS/micold-daemon` is signed **before** the enclosing bundle.
- `codesign --verify --strict "$APP"` exits 0.
- `codesign -dvv "$APP"` reports `Signature=adhoc`.
- `spctl --assess` is expected to **reject**. That rejection is the documented first-launch block;
  it is never asserted in either direction (research R6).

## Producer interface

```sh
scripts/macos-bundle.sh --bin-dir <dir> --out <dir> [--stage-only] [--version <v>]
```

| Flag | Meaning |
|---|---|
| `--bin-dir` | Directory holding the two built binaries (a cargo profile dir, or a `lipo` staging dir) |
| `--out` | Where `micold-ai-ide.app` is written |
| `--stage-only` | Compose and substitute; run no `codesign`. Runs on any OS — this is what makes the layout testable on Linux (research R3) |
| `--version` | Override the version; defaults to the workspace version from `cargo metadata` |

Exit non-zero, with a message naming the missing part, when: a required binary is absent, the icon
is absent, `@VERSION@` remains in the output, or `plutil -lint` fails (on macOS).
