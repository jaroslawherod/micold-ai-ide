# Quickstart: The macOS Package

Three parts. **§A** is what the machine checks on every platform, with no Mac involved. **§B** is
what a macOS runner checks on every pull request — that the bundle composes and the application in
it actually starts. **§C** is what only a real Mac and a real pair of eyes can settle: the download,
the Gatekeeper block, the gesture that clears it, and whether the app still spawns a shell
afterwards.

The split matters here more than usual. §A can prove the *packaging definition* is right; only §C
can prove that a stranger who downloads the file ends up with a running application. A packaging
feature signed off on text scans alone is a feature that has never been installed.

---

## §A — The automated suite (any platform)

```bash
mise run test        # whole workspace, matching CI
mise run test-core   # the two new render-free modules; much faster while iterating

for t in scripts/tests/*.test.sh; do "$t"; done   # the shell suites, as the lint job runs them
```

Green is the gate. What each gate is watching:

| Gate | Watching |
|---|---|
| `micold-core/src/install_location.rs` (unit) | `AppTranslocation` anywhere in the path → `Translocated`; `/Volumes/...` → `MountedImage`; everything else, including relative and empty paths → `Installed` (contracts/install-location.md) |
| `micold-core/src/install_location.rs` (unit) | translocation wins over a `/Volumes/` prefix when a path carries both — the more specific message |
| `micold-core/src/install_location.rs` (unit) | off macOS every input classifies `Installed`, so no other platform's boot path changes |
| `micold-core/src/permission_failure.rs` (unit) | `PermissionDenied` under a protected location classifies; the same error elsewhere does **not** (over-attribution is its own wrong message) |
| `micold-core/tests/macos_permission_strings.rs` | every `ProtectedLocation` variant has a matching usage-description key in `packaging/macos/Info.plist.in` — one list, two consumers. It reads a repository file rather than exercising the module, so it is an integration test, and it is written before the plist is filled (tasks.md T007) |
| `micold-core/tests/macos_minimum_version.rs` | `packaging/macos/Info.plist.in` declares `LSMinimumSystemVersion` with a real value, and that value equals the floor stated in `docs/user-guide/install-macos.md` — the plist and the documentation cannot drift when the floor moves (FR-004) |
| `micold-core/tests/macos_signature_gate.rs` | `scripts/macos-bundle.sh` verifies with `codesign --verify --strict` and reports `Signature=adhoc`, and never gates on `spctl` in either direction (FR-015, research R6) |
| `micold-client/tests/packaging_excludes_showcase.rs` | `scripts/macos-bundle.sh` does not name `micold-showcase`, names both shipped binaries, and contains **no glob copy** into `Contents/MacOS/` (research R14) |
| `micold-client/tests/packaging_excludes_showcase.rs` | the same rules fail against deliberately-broken synthetic scripts — a gate nobody has seen fail is not a gate |
| `micold-client/tests/features_window.rs` | a non-`Installed` verdict routes to the install-me screen and nothing else; there is no path that dismisses it |
| `micold-client/tests/anatomy_call_sites.rs` | the install-me screen is built from shared components with the builder-into-`Element` API (Principle VIII) |
| `micold-core/tests/ci_gate_covers_every_job.rs` | unchanged and still green — §B adds a step, not a job, so the `ci complete` gate's coverage is untouched (research R12) |
| `micold-core/tests/macos_package_gate.rs` | `.github/workflows/ci.yml`'s macOS leg still carries the bundle-and-launch step, and §B below still describes it — an enumerated list in two places drifts silently otherwise (FR-036) |
| `micold-core/tests/release_publishes_complete_sets.rs` | `.github/workflows/release.yml`'s `publish` job lists every artifact-producing job in `needs:`, and fails against a synthetic workflow that omits one — this *is* FR-011's draft-hold |
| `micold-core/tests/macos_registers_nothing.rs` | the bundle definition carries no `LaunchAgents`/`LaunchDaemons` payload and no source file registers a login item or launch agent (`SMAppService`, `launchctl load`) — FR-025 as a checked property, not a claim |
| `micold-core/tests/macos_logout_claims_agree.rs` | `docs/user-guide/install-macos.md`, `docs/daemon.md`, and the module docs of `micold-core/src/logout_survival.rs` make the same claim about macOS logout (FR-032) |
| `scripts/tests/macos-bundle.test.sh` | `--stage-only` produces exactly the layout in contracts/bundle-layout.md from dummy binaries, on Linux |
| `scripts/tests/macos-bundle.test.sh` | a `--bin-dir` that also contains `micold-showcase` still yields a bundle without one |
| `scripts/tests/macos-bundle.test.sh` | `@VERSION@` is fully substituted, and the script exits non-zero naming the missing file when a binary or the icon is absent |
| `scripts/tests/macos-bundle.test.sh` | the universal merge is driven by a target-list variable and fails naming the target when one architecture's binary is missing (FR-020, research R8) |

**What §A cannot tell you**: whether macOS accepts any of it. Every assertion above is over text and
directory entries.

---

## §B — The per-pull-request macOS gate

Runs automatically in `ci.yml`'s `test (macos-latest)` leg, after `Build (workspace)`. To reproduce
on a Mac:

```bash
mise run app                       # compose + ad-hoc sign the bundle from the current build
open target/macos/micold-ai-ide.app # or run Contents/MacOS/micold-ai-ide directly, as CI does
```

| Step | Expected |
|---|---|
| Compose the bundle from the binaries the build already produced | `micold-ai-ide.app` exists; `Contents/MacOS/` holds exactly two files |
| Ad-hoc sign it | `codesign --verify --strict` exits 0; `codesign -dvv` reports `Signature=adhoc` |
| Launch `Contents/MacOS/micold-ai-ide` | Still running a few seconds later — no dyld failure, no plist parse failure, no crash report |
| Wait for `$HOME/.micold/run/d.sock` | It appears. This is the sibling-layout constraint (contracts/bundle-layout.md) failing loudly instead of silently |
| Terminate both | No process left behind |

Single architecture, no disk image, no signature *policy* check — those are the release's job
(FR-037). This step exists so that a change which breaks packaging is caught by the pull request that
introduces it, not by the release it would otherwise block (SC-013).

> **Carried risk**: whether a GitHub macOS runner can create a window at all. If iced cannot obtain
> a surface there, the step treats a window-surface error as a pass and says so in its log — the
> bundle still loaded, dyld still resolved, the daemon still started. Research R12 records the
> contingency; a spike task settles which case applies before the rest of the gate is written.

---

## §C — The manual pass (needs a real Mac)

Nothing below can be automated: it is what a stranger experiences. Record the result in this file's
table when the feature is implemented.

### C1 — Install, the way a user does it

1. Download the `.dmg` from the releases page **with a browser** (curl does not set the quarantine
   flag, so a curl'd file will not reproduce anything in C2).
2. Open it. Expect: a window with the application and an `Applications` folder beside it.
3. Drag the application onto `Applications`. Eject the image.

| Check | Expected |
|---|---|
| The download page told you what would happen next | Yes — the trust notice is beside the download (FR-014, SC-001) |
| Time from releases page to a running app | Under 5 minutes, no terminal (SC-001) |

### C2 — The first launch is blocked, exactly as documented

1. Double-click the installed application.
2. Expect a block: macOS cannot attribute it to a registered developer.
3. Follow the documented gesture — System Settings → Privacy & Security → **Open Anyway** — then
   launch again.

| Check | Expected |
|---|---|
| The block matched what the documentation predicted, wording included | Yes (FR-013) |
| The gesture needed no terminal | Yes (FR-014) |
| It was **one** gesture, not one per macOS version | Yes across the supported floor (FR-004) |
| After it, launching is normal and silent | Yes (FR-013) |

### C3 — Running it from the disk image is refused

1. Re-mount the `.dmg` and launch the application **from inside it**.
2. Expect: one screen saying it is running from the downloaded image and to install it first. No
   session UI, no dismissal (FR-019).

### C4 — The application actually works after the block is cleared

| Check | Expected |
|---|---|
| The window renders | Yes |
| A session starts; a shell runs in it | Yes — the daemon was found beside the app (FR-005) |
| `git` runs inside a session | Yes |
| An AI CLI (`claude`) starts inside a session | Yes — the hardened runtime does not interfere (FR-018, research R5) |
| Dock, menu bar, Spotlight, force-quit list all show the name and icon | Yes (FR-002) |
| About reports the same version as the release and as `Info.plist` | Yes (FR-003) |

### C5 — Permissions

1. Open a project stored in `~/Documents`.
2. Expect the system's own prompt, carrying **our** sentence explaining why (FR-027).
3. Decline it, then retry the same action.

| Check | Expected |
|---|---|
| The prompt gave a reason in the user's terms | Yes |
| After declining, the failure named the permission and how to grant it | Yes — not a generic error, not "broken project" (FR-028) |
| The documented Full Disk Access route is optional and clearly labelled as such | Yes (FR-029) |

### C6 — Lifecycle

| Check | Expected |
|---|---|
| Installing registers no login item and no launch agent | `~/Library/LaunchAgents` unchanged; nothing new in Login Items (FR-025) |
| Replacing the app with a newer copy preserves projects, settings, history | Yes (FR-022) |
| A stale daemon from the previous version does not require a manual `kill` | Yes (FR-023) |
| Dragging the app to the Trash leaves nothing running | Yes; the documentation lists what state remains and where (FR-024) |
| Logging out ends sessions; closing the window does not | Yes, and the documentation says so and points at the container placement (FR-026) |

### C7 — The floor is enforced by the system

On a Mac older than the declared floor, expect LaunchServices to refuse with its own "requires
macOS 15 or later" message — not a crash, and not our own check (FR-004).

---

## Record of the pass

| Part | Date | Machine / macOS | Result |
|---|---|---|---|
| §C1 | | | |
| §C2 | | | |
| §C3 | | | |
| §C4 | | | |
| §C5 | | | |
| §C6 | | | |
| §C7 | | | |
