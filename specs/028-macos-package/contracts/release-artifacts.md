# Contract: release artifacts and the publish gate

**Feature**: [../spec.md](../spec.md) | Normative for `.github/workflows/release.yml`.

## Job graph

```text
release-please ──> deb (amd64)      ─┐
               ──> deb (arm64)      ─┤
               ──> macos            ─┼──> publish   (gh release edit --draft=false --latest)
               ──> windows (x64)    ─┤
               ──> windows (arm64)  ─┤
               ──> image / manifest ─┘
```

The `windows` legs were added by feature 030
(`specs/030-windows-installer/contracts/release-artifacts.md`), and `image-manifest` by the sandbox
image work; neither changes the rule below.

`publish` MUST list every artifact-producing job in `needs:`. This single fact is the whole of
FR-011: GitHub Actions runs a `needs:` job only if all of its dependencies succeeded, and
release-please creates the release as a **draft**, so:

| Situation | Outcome | Why it is the required one |
|---|---|---|
| Every artifact job succeeds | Release published with every asset | The normal path |
| The macOS job fails | Release stays an unpublished **draft**; the failed job is red | Nothing incomplete is published; nothing is lost; the version number is not consumed |
| The macOS job is re-run and succeeds | `publish` runs; release goes out complete | Recovery is a re-run, not a new version (FR-011) |

Adding an artifact job without adding it to `needs:` silently reintroduces partial publication. A
published release is immutable and cannot be corrected.

## Artifacts

| Name | Job | Runner |
|---|---|---|
| `micold-ai-ide_<version>_amd64.deb` | `deb (amd64)` | `ubuntu-22.04` |
| `micold-ai-ide_<version>_arm64.deb` | `deb (arm64)` | `ubuntu-22.04-arm` |
| `MicoldAIIDE-<version>-universal.dmg` | `macos` | `macos-latest` |
| `micold-ai-ide-<version>-x64-setup.exe` | `windows (x64)` | `windows-latest` |
| `micold-ai-ide-<version>-arm64-setup.exe` | `windows (arm64)` | `windows-11-arm` |

Every asset is uploaded to the draft with `gh release upload "$TAG_NAME" <path> --clobber`, so a
re-run replaces rather than duplicates.

## The macOS job

1. `dtolnay/rust-toolchain@stable` with `targets: aarch64-apple-darwin, x86_64-apple-darwin`.
2. `cargo build --release` for both targets, both shipped crates.
3. `scripts/macos-dmg.sh` — `lipo`, bundle (via `scripts/macos-bundle.sh`), sign, verify, stage,
   `hdiutil`.
4. Signature verification (contracts/bundle-layout.md) — a failure here fails the job, which holds
   the release (FR-037).
5. Upload.

No secret, certificate, or Apple account is used. A contributor running `mise run dmg` on their own
Mac produces an artifact of equivalent trust status (FR-016).

## Per-pull-request coverage

The release job is **not** what proves the packaging works. `ci.yml`'s macOS matrix leg composes and
launches the bundle on every code-affecting change (FR-036); the release job additionally covers the
second architecture, the disk image, and the signature check — the three steps only a release
performs (FR-037).
