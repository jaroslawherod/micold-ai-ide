# Contract: release artifacts and the publish gate (Windows extension)

**Feature**: [../spec.md](../spec.md) | Extends `specs/028-macos-package/contracts/release-artifacts.md`
(PR #284). Normative for `.github/workflows/release.yml`, `.github/workflows/ci.yml` and `site/`.

## Job graph

```text
release-please ──> deb (amd64)       ─┐
               ──> deb (arm64)       ─┤
               ──> macos             ─┼──> publish   (gh release edit --draft=false --latest)
               ──> windows (x64)     ─┤
               ──> windows (arm64)   ─┤
               ──> image / manifest  ─┘
```

`publish.needs` MUST include `windows`. `release_publishes_complete_sets.rs` (introduced by 028)
already fails if a job containing `gh release upload` is missing from `needs`, so no new test is
required. If 028 has not merged, this feature introduces that test with the same rule (see research
"Coordination").

| Situation | Outcome |
|---|---|
| Both Windows legs succeed | Release published with both setup executables |
| Either Windows leg fails (build, `iscc`, install smoke, upload) | Release stays a draft, and the failed leg is red (FR-014, SC-005) |
| Failed leg is re-run and succeeds | `publish` runs, and the release goes out complete. The version is not consumed. |

## Artifacts added

| Name | Job | Runner |
|---|---|---|
| `micold-ai-ide-<version>-x64-setup.exe` | `windows (x64)` | `windows-latest` |
| `micold-ai-ide-<version>-arm64-setup.exe` | `windows (arm64)` | `windows-11-arm` |

Both are uploaded with `gh release upload "$TAG_NAME" <path> --clobber`.

## The Windows release job

1. `actions/checkout`, then `dtolnay/rust-toolchain@stable` with the job's target.
2. `scripts/windows-installer.sh --arch <arch> --out-dir dist` (contracts/windows-installer.md).
   It uses `CARGO_TARGET_DIR: target` from the workflow env.
3. `scripts/windows-install-smoke.sh dist/<asset>`, the same script CI runs on every change. A failure
   holds the release.
4. `gh release upload`.

No secret, certificate or signing identity is used (FR-010).

## Release notes

`.github/release-notice-windows.md` is appended to the release body by `publish`, after the macOS
notice. It is at most 5 lines:

- which file to pick (x64 vs ARM64, and how to tell)
- the SmartScreen "More info → Run anyway" step
- a link to `docs/user-guide/install-windows.md`

## Per-change coverage (ci.yml)

| Job / step | Runner | Proves |
|---|---|---|
| `test` matrix, Windows leg: `cargo test -p micold-daemon --all-targets` | `windows-latest` | FR-020 to FR-025 (contracts/windows-endpoint.md) |
| `test` matrix, Windows leg: "Package the Windows installer", then "Install and launch the Windows installer" | `windows-latest` | FR-018 / SC-007 for x64 |
| `windows-arm64-package` (new job, listed in `ci-complete.needs`) | `windows-11-arm` | FR-015 / FR-018 for ARM64 |

`ci_gate_covers_every_job.rs` enforces the `ci-complete.needs` listing.

Both run under the existing code-affecting path filter. A docs-only change skips them (feature 023).

## Docs job

The required pages list gains:

- `docs/user-guide/install-windows.md`
- `docs/development/windows-packaging.md`

## Site

`site/stage.sh` validates each download link against the release's asset list. The Windows download
links name the two setup executables. A link to a non-existent asset fails the Pages build (FR-017).
