# Packaging for macOS

How `MicoldAIIDE-<version>-universal.dmg` is produced, how to reproduce it on your own Mac, and
what would change if the project ever obtained a Developer ID.

The normative descriptions live with the feature that introduced this:
[`specs/028-macos-package/contracts/bundle-layout.md`](../../specs/028-macos-package/contracts/bundle-layout.md)
for what is inside the application, and
[`release-artifacts.md`](../../specs/028-macos-package/contracts/release-artifacts.md) for the job
graph. This page is the working explanation of both.

## The two producers

There is one script that lays out the application and one that wraps it for delivery. Nothing else
composes a bundle — including the release workflow, which calls the same two scripts a contributor
does, so what CI verifies and what you can reproduce locally are the same bytes.

| Script | Produces | Runs on |
|---|---|---|
| `scripts/macos-bundle.sh` | `micold-ai-ide.app`, ad-hoc signed and verified | macOS; `--stage-only` anywhere |
| `scripts/macos-dmg.sh` | `MicoldAIIDE-<version>-universal.dmg` | macOS only (`hdiutil`) |

`macos-dmg.sh` calls `macos-bundle.sh`. It does not repeat any of it.

### Reproducing a release locally

```sh
mise run app    # the .app for this Mac, ad-hoc signed
mise run dmg    # the .dmg, with the /Applications symlink and the signature re-verified inside it
```

Both land under the shared target directory, in `macos/`. `mise run app` also works on Linux, where
it stages the bundle without signing — that is what `scripts/tests/macos-bundle.test.sh` drives, and
it is why the layout rules can be checked without a Mac.

The artifact you get from `mise run dmg` is of **equivalent trust status** to the released one: the
signature is ad-hoc, so it involves no Apple account, no certificate, and no repository secret.
There is nothing a maintainer holds that you do not.

## What the release job adds

`ci.yml` composes and launches the bundle on every code-affecting pull request — that is what proves
the packaging works. The `macos` job in `release.yml` covers the three things only a release does:

1. Both architectures, merged into one universal binary.
2. The disk image itself.
3. The signature verified from *inside the mounted image*, not from the directory it was built in.

A failure in any of them fails the job. `publish` lists every artifact-producing job in `needs:`, so
the release then stays an unpublished draft rather than going out without its macOS download — a
published release is immutable and could never be corrected.
`crates/micold-core/tests/release_publishes_complete_sets.rs` is what keeps that `needs:` list
complete when the next artifact is added.

## Before a release: re-check the version floor

The floor is `LSMinimumSystemVersion` in `packaging/macos/Info.plist.in`, and it is recorded there
and nowhere else. LaunchServices enforces it: a Mac below it is refused by the operating system,
with the required version named, before anything of ours runs.

The project supports **the two most recent major releases of macOS**. Apple ships one every autumn,
so this value has a scheduled expiry rather than an accidental one. Check it as a named step when
preparing a release:

1. What are the two most recent major macOS releases? (Apple's macOS page, or `sw_vers` on an
   up-to-date Mac.)
2. Is the older of the two the version in the plist? If yes, nothing to do.
3. If not, change it in **two places** and nowhere else:
   - `packaging/macos/Info.plist.in` — the `LSMinimumSystemVersion` string.
   - `docs/user-guide/install-macos.md` — the `**Minimum macOS version: …**` sentence.

`crates/micold-core/tests/macos_minimum_version.rs` asserts those two agree, so getting one and not
the other is a red suite rather than a page that tells users their Mac is supported while the
application refuses to open. Nothing else reads the value: no build script, no `cargo` metadata, no
CI variable.

When the floor eventually passes macOS 26 — Apple's last Intel release — the `x86_64-apple-darwin`
entry can be dropped from `TARGETS` in `scripts/macos-dmg.sh`. The artifact keeps its name and its
shape; the merge simply has one input.

## The `spctl` diagnostic

```sh
spctl --assess --type execute --verbose=4 /Applications/micold-ai-ide.app
```

Expect it to **reject** the application, with `source=no usable signature` or similar. That is the
correct answer for an ad-hoc signature and it is the first-launch block users are told about in
[Installing on macOS](../user-guide/install-macos.md) — not a defect and not a build failure.

For that reason `spctl` is not run by any script here, and
`crates/micold-core/tests/macos_signature_gate.rs` asserts it never becomes one: a gate on `spctl`
would fail every build today, and a gate asserting rejection would fail on the day the project
starts notarizing, which is the day it should go green.

What *is* checked automatically, everywhere the bundle is signed:

```sh
codesign --verify --strict --verbose=2 micold-ai-ide.app   # the seal covers what is inside
codesign -dvv micold-ai-ide.app 2>&1 | grep Signature=adhoc
```

`--strict` is the flag that notices a file added to `Contents/` after signing. A bundle modified
after the fact reaches the user as *"the application is damaged and can't be opened"*, which names
nothing — so it is caught here instead.

## What Developer ID + notarization would change

The release process is built so that this is a change of *identity*, not a change of shape
(FR-017). Nothing about the bundle layout, the scripts' interfaces, the job graph, or the install
gesture moves.

| | Today | With a Developer ID |
|---|---|---|
| Signing identity | `codesign --sign -` (ad-hoc) | `codesign --sign "Developer ID Application: …"` |
| Secrets needed | none | certificate + password, and an App Store Connect API key |
| Extra release step | none | `xcrun notarytool submit --wait`, then `xcrun stapler staple` |
| Hardened runtime | already enabled | unchanged — it is already what notarization requires |
| Entitlements | `packaging/macos/entitlements.plist` | unchanged |
| First launch | blocked once; cleared in System Settings | silent |
| `spctl --assess` | rejects | accepts |

Concretely, that is: two `--sign -` occurrences in `scripts/macos-bundle.sh` become the identity,
a keychain-import step and a notarize-and-staple step join the `macos` job in `release.yml`, and the
first-launch sections of the user guide and `.github/release-notice-macos.md` are deleted. The
hardened runtime is enabled today precisely so that this list stays that short.

Until then the documentation carries the weight, which is why the first-launch text is treated as
part of the artifact rather than as a nicety.
