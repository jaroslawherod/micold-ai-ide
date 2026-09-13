# Phase 0 Research: macOS Package

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-31

No `NEEDS CLARIFICATION` markers entered this phase — `/speckit-clarify` resolved five open
questions before planning (spec §Clarifications). What follows resolves the *technical* unknowns the
plan depends on. Each entry: Decision → Rationale → Alternatives considered.

---

## R1. How the `.app` bundle is produced

**Decision**: A hand-written `scripts/macos-bundle.sh`, plus a `packaging/macos/Info.plist.in`
template. No third-party bundler.

**Rationale**:
- The bundle must contain **two** executables side by side (`micold-ai-ide` and `micold-daemon`),
  because `micold_core::spawn::daemon_binary()` resolves the daemon as a sibling of
  `current_exe()`. Every off-the-shelf bundler models one binary per bundle and treats anything else
  as a resource, which would put the daemon somewhere the client does not look.
- The showcase-exclusion guarantee (FR-006) is a property of a *declarative, checkable* list. A
  script with an explicit three-line copy list can be text-scanned exactly the way
  `packaging_excludes_showcase.rs` scans `[package.metadata.deb] assets` today. A bundler's implicit
  "ship the package's binaries" behaviour is the same fallback trap that test already guards against
  on the Debian side.
- FR-016 forbids a required account or secret, and FR-035 wants one documented command in the task
  runner. A script invoked by `mise run app` satisfies both with no install step; `cargo install
  cargo-bundle` in CI is a dependency to vet (Constitution: Dependencies) for work that is ~40 lines
  of `mkdir`, `cp`, and `sed`.

**Alternatives considered**:
- **`cargo-bundle`** — the obvious choice, but it has been effectively unmaintained for years, does
  not model a second executable, and its `osx_minimum_system_version` handling would still leave the
  plist substitution to us.
- **`cargo-packager`** — actively maintained and more capable (it can do the DMG too), but it pulls
  a large tool and its own config schema into the build for a bundle this simple, and its
  multi-binary support still assumes one *bundled* binary.
- **`create-dmg`** — see R4.

## R2. Bundle layout

**Decision**:

```text
micold-ai-ide.app/Contents/
├── Info.plist
├── PkgInfo                      # "APPL????"
├── MacOS/
│   ├── micold-ai-ide            # CFBundleExecutable
│   └── micold-daemon            # sibling — what spawn.rs looks for
└── Resources/
    └── icon.icns                # copied from assets/icon/icon.icns
```

**Rationale**: `Contents/MacOS/` is the only directory `current_exe().parent()` can be, so it is the
only place the daemon can go without changing the discovery rule. Putting it in
`Contents/Resources/` or `Contents/Helpers/` would require a macOS-specific branch in
`daemon_binary()` — a platform branch in core logic, which Principle VI pushes back on, in exchange
for nothing. `assets/icon/icon.icns` already exists and is already the Linux/Windows icon source, so
no icon generation step is added.

**Alternatives considered**: `Contents/Library/LaunchServices/` (the layout used for privileged
helpers) — solves a problem we do not have, and still invisible to the sibling lookup.

## R3. Testing the bundle script without a Mac

**Decision**: `scripts/macos-bundle.sh` takes `--stage-only`, which composes the directory tree and
substitutes the plist but runs no `codesign`. That mode uses nothing but `mkdir`, `cp`, and `sed`,
so it runs on Linux, and `scripts/tests/macos-bundle.test.sh` drives it there with dummy binaries in
a temp dir — asserting the layout of R2, the substituted plist values, and that a staging tree built
from a directory containing a `micold-showcase` file does not contain one.

**Rationale**: Principle I applies to the packaging definition, not only to Rust. This repository
already runs `scripts/tests/*.test.sh` in the `lint` job on Linux; putting the layout rules there
means they are Red-Green-testable on any developer's machine, and the macOS-only half of the script
(`codesign`, `hdiutil`) shrinks to the part that genuinely needs a Mac.

**Alternatives considered**: macOS-runner-only shell tests — they would run in CI but not locally
for most contributors, and would make the first Red of a TDD cycle a 5-minute round trip.

## R4. The delivery container

**Decision**: `hdiutil create -volname "Micold AI IDE" -srcfolder <staging> -ov -format UDZO
<out>.dmg`, where `<staging>` holds the `.app` and a symlink named `Applications` pointing at
`/Applications`.

**Rationale**: The app plus an `/Applications` symlink *is* the conventional drag gesture (FR-008) —
it is what the user sees when the volume opens, and it needs no window-geometry choreography.
`hdiutil` ships with macOS, so there is no install step and no third-party tool in the release path.
UDZO is compressed and read-only, which is what a downloaded image should be.

**Alternatives considered**:
- **`create-dmg`** (the Homebrew script) — gives a background image and positioned icons, at the
  cost of a `brew install` in the release job and an AppleScript-driven Finder window that is a
  known flake source on headless runners. Not worth it for a first release; the layout it produces
  can be added later without changing the artifact's shape.
- **A `.pkg` installer** — an installer for a self-contained app is a step backwards on macOS, and
  it would make FR-024's "removing it leaves nothing running" harder to state, not easier.
- **A bare `.zip`** — allowed by Gatekeeper and simpler, but it is not the conventional gesture, and
  the app would land in `~/Downloads`, which is precisely the App Translocation case FR-019 exists to
  refuse.

## R5. Signing

**Decision**: Ad-hoc sign at build time, innermost binary first:

```sh
codesign --force --timestamp=none --options runtime --sign - "$APP/Contents/MacOS/micold-daemon"
codesign --force --timestamp=none --options runtime --sign - \
         --entitlements packaging/macos/entitlements.plist "$APP"
```

**Rationale**: Signing nested code before the enclosing bundle is required — the outer signature
seals the inner one, so the reverse order produces a bundle that verifies today and fails after any
nested change. `--deep` would do it in one call but is deprecated by Apple and is documented as
unsuitable for producing a shippable signature.

Enabling the hardened runtime (`--options runtime`) and passing an entitlements file **now**, while
both are inert, is the whole of FR-017: notarization later requires the hardened runtime, and
discovering then that it breaks something is exactly the "reshaping how the artifact is built" that
requirement forbids. The risk it introduces is bounded — library validation only restricts loading
unsigned *libraries into this process*, and the app loads none; spawning `git`, a shell, or `claude`
as separate child processes is unaffected. The quickstart's Part B verifies that claim on a real Mac
rather than assuming it (FR-018).

`entitlements.plist` is an empty `<dict/>` today. It is the file a future Developer ID + notarization
change edits, so that change is credentials plus one `xcrun notarytool submit --wait` step plus
`xcrun stapler staple`, with no restructuring.

**Alternatives considered**:
- **No signature at all** — macOS reports an unsigned app on Apple silicon as *damaged*,
  indistinguishable from a corrupt download, and the app cannot launch at all on arm64. Not viable.
- **Ad-hoc without the hardened runtime** — marginally safer today, and defers the whole
  compatibility question to the release where it would be most expensive to discover.

## R6. Verifying the signature (FR-015)

**Decision**: `codesign --verify --strict --verbose=2 "$APP"` must exit 0, and
`codesign -dvv "$APP" 2>&1` must report `Signature=adhoc`. The release script fails on either.

**Rationale**: This catches the two regressions that matter — an unsigned or malformed bundle, and a
bundle whose signature was invalidated by a later edit to its contents. Both reach the user as
"damaged".

**Explicitly NOT a gate**: `spctl --assess --type execute`. An ad-hoc signature is *supposed* to be
rejected by Gatekeeper assessment — that rejection is the first-launch block FR-013 documents. Gating
on `spctl` acceptance would fail every build; gating on its *rejection* would silently invert the
day a Developer ID arrives. It is documented in `docs/development/macos-packaging.md` as a diagnostic
to run by hand, not as a check.

## R7. `Info.plist` contents

**Decision**: `packaging/macos/Info.plist.in` with `@VERSION@` substituted at build time. Keys, and
why each is present:

| Key | Value | Why |
|---|---|---|
| `CFBundleIdentifier` | `io.github.cumulocity-iot.micold-ai-ide` | Reverse-DNS of the repository host — honest for an ad-hoc-signed open-source app that owns no Apple team ID. TCC grants are keyed on it, so it is chosen once and then stable (R10). |
| `CFBundleName` / `CFBundleDisplayName` | `Micold AI IDE` | FR-002: the name macOS shows in the Dock, menu bar, Spotlight, force-quit list. |
| `CFBundleExecutable` | `micold-ai-ide` | The binary in `Contents/MacOS/`. |
| `CFBundleIconFile` | `icon` | Resolves to `Resources/icon.icns`. |
| `CFBundleShortVersionString` | `@VERSION@` | FR-003 — the user-visible version. |
| `CFBundleVersion` | `@VERSION@` | Same value; there is no separate build number. |
| `CFBundlePackageType` | `APPL` | Matches `PkgInfo`. |
| `LSMinimumSystemVersion` | `15.0` | FR-004 — LaunchServices refuses to open the app below this and says why, which is the "enforced by the operating system, not left to the user" half of the requirement. |
| `NSHighResolutionCapable` | `true` | Without it the app renders upscaled and blurry on every modern display. |
| `NSDocumentsFolderUsageDescription`, `NSDesktopFolderUsageDescription`, `NSDownloadsFolderUsageDescription`, `NSRemovableVolumesUsageDescription`, `NSNetworkVolumesUsageDescription` | one sentence each | FR-027 — the reason shown *inside* the system's own prompt. Without a string, macOS shows a bare generic prompt; with one, the user is told why an IDE wants their Documents folder. |

`LSUIElement` is deliberately absent — this is a normal windowed app.

**Rationale for the version being substituted rather than hard-coded**: FR-003 requires the plist,
the release, and the About dialog to agree. All three then derive from one source,
`[workspace.package] version`, which release-please already bumps.

**Alternatives considered**: reading the version with `cargo metadata --format-version 1 | jq`
(needs `jq` on the runner) versus `cargo metadata | python3 -c ...` versus a `grep` of the workspace
manifest. Decision: `cargo metadata` piped through `python3` — both are present on every runner and
on any machine that can build this project, and it reads the *resolved* version rather than a line
that happens to look like one.

## R8. Universal binary

**Decision**: Build both targets in one cargo invocation and merge with `lipo`:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release -p micold-client -p micold-daemon \
  --target aarch64-apple-darwin --target x86_64-apple-darwin
lipo -create -output "$STAGE/micold-ai-ide" \
  target/aarch64-apple-darwin/release/micold-ai-ide \
  target/x86_64-apple-darwin/release/micold-ai-ide     # and again for micold-daemon
```

**Rationale**: One artifact removes FR-020's problem entirely — there is no download choice to get
wrong, so no user has to know which Mac they have. Cross-compiling `x86_64-apple-darwin` from an
Apple silicon runner needs no extra SDK; the toolchain ships both.

**Note on lifespan** (already recorded in the spec's Assumptions): macOS 26 (Tahoe) is Apple's last
Intel release. Once the two-most-recent floor moves past it, the `x86_64` half stops being required
and `lipo` can be dropped without changing the artifact's name or shape. The script therefore takes
the target list as a variable rather than hard-coding two `lipo` inputs.

**Alternatives considered**: two separate per-architecture `.dmg`s — cheaper to build, but it puts
the choice in front of the user, which FR-020 forbids unless the download page makes it unambiguous.
A universal binary makes the requirement moot rather than satisfied-with-effort.

## R9. Detecting "not installed" (FR-019)

**Decision**: A pure predicate in `micold-core`:

```rust
pub enum InstallLocation { Installed, MountedImage, Translocated }
pub fn classify(executable: &Path) -> InstallLocation
```

`Translocated` when any path component is `AppTranslocation`; `MountedImage` when the path starts
with `/Volumes/`; `Installed` otherwise (and always, off macOS). The client calls it once at boot
with `std::env::current_exe()` and, on anything but `Installed`, renders the install-me screen
instead of the session UI.

**Rationale**: Path classification is a total function over a string, so it is unit-testable on
every platform with no macOS API and no filesystem — which is what lets it live in the render-free
core and be covered by the three-platform matrix. `SecTranslocateIsTranslocatedURL` is the official
API, but it requires linking `Security.framework`, only answers the translocation half, and would
make the decision untestable off macOS: a worse trade for a check whose input is a path either way.

**Known limits, accepted**: a user who genuinely installs the app onto an external volume mounted at
`/Volumes/...` is told to install it properly. That is a rare case, the message says what to do, and
the alternative — trying to distinguish a mounted DMG from a mounted external disk — trades a clear
false positive for an unclear false negative. Recorded here so it is a decision rather than a bug.

## R10. File-access permissions (FR-027, FR-029)

**Decision**: Ship purpose strings for the five protected locations (R7) and document Full Disk
Access as an optional one-time grant. Request nothing programmatically.

**Rationale**: There is no API to request TCC access; the first access triggers the prompt, and the
purpose string is the only thing the app controls about it. Full Disk Access likewise cannot be
requested — it is granted by the user in System Settings — so FR-029's escape hatch is inherently
documentation, which is what that requirement says.

**Consequence worth stating in the docs**: TCC grants are keyed on bundle identifier *and*
signature. An ad-hoc signature differs between builds, so macOS may re-prompt after an update. This
is a property of not having a Developer ID, not a defect, and `docs/user-guide/install-macos.md`
says so — the same way it explains why the first-launch block returns after an update (FR-013,
already an edge case in the spec).

## R11. Attributing a permission failure (FR-028)

**Decision**: A second render-free module classifying an `io::Error` from a project-directory
operation into `PermissionFailure { location, how_to_grant }` when the OS reports
`PermissionDenied` (or `Operation not permitted`, `EPERM`, which is what TCC actually returns) and
the path lies under a protected location. The client shows that message instead of a generic error.

**Rationale**: TCC denial is reported as an ordinary permission error, so without this the user sees
"failed to read directory" for something no file mode explains — the exact failure FR-028 exists to
prevent. Classifying by `ErrorKind` plus path prefix keeps it a pure function of two inputs and
therefore unit-testable everywhere.

**Alternatives considered**: probing with a test read at startup — that *causes* the prompt at a
moment the user has no context for, which is the opposite of FR-027's "at the moment it needs them".

## R12. Where the per-PR gate runs (FR-036)

**Decision**: A new step inside the **existing** `test` job in `ci.yml`, guarded by
`if: runner.os == 'macOS'`, running after `Build (workspace)`. It stages the bundle from the debug
binaries that step just produced, ad-hoc signs it, launches
`micold-ai-ide.app/Contents/MacOS/micold-ai-ide`, and asserts the process is still alive after a
few seconds and that the daemon endpoint appeared — then terminates both.

**Rationale**: FR-036 says to reuse the macOS build that already runs, and a step inside that job is
the literal reading: no second checkout, no second compile, no cache miss. It also avoids editing
the `ci-complete` gate — `crates/micold-core/tests/ci_gate_covers_every_job.rs` asserts every job
appears in that gate's `needs:`, so a *new* job would be a correct-but-larger change touching the
one check the branch ruleset requires by name.

Asserting the daemon endpoint appeared is what makes this a *packaging* test rather than a launch
test: it is the sibling-layout constraint (R2) failing loudly instead of silently.

**Open risk, carried into implementation**: whether a GitHub macOS runner can create a window at
all. If iced cannot obtain a surface there, the launch fails for a reason that has nothing to do
with packaging. Contingency, in order of preference: (a) it works — the runner's virtualised GPU
supports Metal, and iced falls back to `tiny_skia` if not; (b) if it does not, the step asserts the
process reached window creation (bundle loaded, dyld resolved, plist parsed, daemon spawned) and
treats a window-surface error as a pass, with the reason written into the step's log. A spike task
in `/speckit-tasks` settles which, on the runner, before the rest of the gate is written.

**Outcome (T001, recorded 2026-09-13)**: contingency (a). CI run
[34747999622](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/34747999622) on PR #284,
commit `3da33207`, `build + test (macos-latest)` on runner image `macos-26-arm64` version
`20260907.0351.1`: the bundle was staged and ad-hoc signed (`codesign --verify` printed "valid on
disk" and "satisfies its Designated Requirement" at 08:35:27.11 UTC), and the launch took the step's
first branch — "Bundle launched; daemon endpoint /Users/runner/.micold/run/d.sock appeared." at
08:35:28.26, about 1.15 s later. The step's (b) branch, which greps the launch log for a surface
error, was not reached, so it stays in `ci.yml` as a guard for a future runner image rather than as
the path this one takes.

What that proves, precisely: the binary started from inside the bundle, found its sibling daemon
through the R2 layout, spawned it, and was still alive when the endpoint appeared — the packaging
claim FR-036 is after. It does **not** by itself prove a drawing surface was obtained. iced_winit
0.14 tracks the program's subscriptions at boot (`lib.rs`, the `runtime.track` just after the window
`open` task is queued), before the event loop creates the window and, lazily with it, the compositor;
the daemon is spawned from one of those subscriptions (`daemon::connection`). The two run
concurrently, so the endpoint can in principle appear before the compositor is built. A compositor
failure reaches the event loop as `Control::Crash` and ends the process, so a failure *before* the
check would have sent the step down the second branch; one after it would go unseen, because the
step checks liveness once, when the endpoint appears, not "after a few seconds" as the Decision
above describes. The launch log is printed only on the failure branches, so this run left no
positive evidence of the window either way. The observation is consistent with (a) — and contradicts
the premise of (b), that the runner refuses a surface outright — without closing the narrow ordering
window; seeing the window itself is quickstart §C on a real Mac (T059).

## R13. Where the release gate runs (FR-037)

**Decision**: A new `macos` job in `release.yml`, mirroring `deb`: `runs-on: macos-latest`, builds
both targets, runs `scripts/macos-dmg.sh`, verifies the signature, uploads to the draft with
`gh release upload "$TAG_NAME" ... --clobber`. `publish` gains `macos` in `needs:`.

**Rationale**: FR-011 needs no new machinery — `publish` already runs only if every job it needs
succeeded, and release-please already creates the release as a *draft*. Adding one name to `needs:`
makes a failed macOS build hold the entire release as an unpublished draft, visible as a red job,
recoverable by re-running it, consuming no version number. The alternative designs (a
publish-anyway-then-patch flow) are impossible anyway: a published release is immutable.

## R14. Keeping the showcase out (FR-006)

**Decision**: Extend `crates/micold-client/tests/packaging_excludes_showcase.rs` with a third input —
`scripts/macos-bundle.sh` — asserting it (a) does not name `micold-showcase`, (b) does name both
shipped binaries, and (c) contains no glob copy into `Contents/MacOS/`. Synthetic broken inputs
drive each rule, as the existing tests do for the Debian list.

**Rationale**: `cargo build --release` puts all three binaries in one directory. A `cp
target/release/* .../MacOS/` would work, pass a naive check, and ship the showcase — the exact shape
of the cargo-deb fallback trap that file's header already explains. Rule (c) is the one that would
have caught it.

## R15. Toolchain on the runner

**Decision**: `dtolnay/rust-toolchain@stable` with `targets: aarch64-apple-darwin,
x86_64-apple-darwin` in the release job; the CI job needs no extra target (single-arch by FR-036).

**Rationale**: The action installs targets itself, so no separate `rustup target add` step, and the
release job is the only place the second architecture is needed.

## R16. Documentation placement

**Decision**: `docs/user-guide/install-macos.md` (user-facing: download, install, the first-launch
block and the one gesture that clears it, the macOS floor, permissions and the optional broad grant,
updating, removing, where data lives, the long-`$HOME` endpoint limit) and
`docs/development/macos-packaging.md` (how the artifact is produced, how to reproduce it locally
with `mise run app` / `mise run dmg`, what changes to add Developer ID + notarization). Both are
added to the `docs` job's `test -f` lists so Principle VII is enforced rather than asserted.

**Rationale**: This mirrors how feature 020 and feature 023 split user-facing from developer-facing
documentation, and the `docs` job is deliberately unconditional — it is the one gate that still runs
on a documentation-only change.

**Note on `.gitattributes`**: `docs/**` is already declared `micold-docs`, so both new pages are in
the documentation set automatically. The new packaging scripts, plist, and workflow edits are *not*
documentation, and are correctly treated as code-affecting by `scripts/classify-change.sh`.

One declaration does have to change, though. `specs/**` is also `micold-docs`, and this feature's
per-PR gate (R12) compares `ci.yml` against §B of *this feature's own quickstart* — so that file
becomes a test input rather than prose. Two things break otherwise: `documentation_is_not_read.rs`
forbids a test reading a `micold-docs` path, and an edit to §B would skip the whole pipeline, making
the one edit the gate exists to catch the one edit it never sees. Feature 027 hit this first and
carved its quickstart out with a trailing `-micold-docs` line; this feature needs the same line, and
it must come after `specs/**` because the last matching line wins.
