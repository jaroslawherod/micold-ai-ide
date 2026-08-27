# FR-024: publishing the sandbox image with the release

**Date**: 2026-08-27 · **Host**: Linux 7.0.0-30-generic, Docker 29.5.1 (build 2518b52) ·
**Base commit**: `06018b65` (`chore(main): release micold-ai-ide 0.11.0`)

## What was wrong

FR-024 says the default image "MUST be published and versioned with the application release and
acquired automatically, so a first run requires no manual image preparation." None of that was
true through 0.11.0:

- `.github/workflows/release.yml` had three jobs — `release-please`, `deb`, `publish`. No job built
  a container image and none pushed one. The 0.11.0 release, cut earlier today, carries two `.deb`
  files and nothing else.
- `DEFAULT_IMAGE` resolved to `ghcr.io/micold/micold-daemon:<version>`. This repository's owner is
  `jaroslawherod`; `micold` is a namespace it does not control and in which nothing was ever
  pushed. A user's first sandboxed run would meet a `denied`.

The reason it survived to a release is worth writing down, because none of the feature's own gates
were capable of catching it. The sandbox is opt-in, so no default path resolves `DEFAULT_IMAGE`.
The real-runtime suite — all 23 tests, the ones Phase 14 got running in CI — builds
`micold-daemon:dev` locally and asserts against that, so the whole suite passes with no registry in
existence. And `evidence/performance.md` (T117) had already written the symptom down —
*"the route that would (a registry pull) has nothing published to pull"* — where it read as a
caveat on a measurement rather than as a missing feature.

## What now happens

`release.yml` gains two jobs between `deb` and `publish`:

| job | runs on | produces |
|---|---|---|
| `image (amd64)` | `ubuntu-22.04` | `ghcr.io/jaroslawherod/micold-daemon:<version>-amd64` |
| `image (arm64)` | `ubuntu-22.04-arm` | `ghcr.io/jaroslawherod/micold-daemon:<version>-arm64` |
| `image (version tag)` | `ubuntu-latest` | `ghcr.io/jaroslawherod/micold-daemon:<version>` — the multi-architecture index, and the reference the client compiles in |

`publish` now needs `image-manifest`. The ordering is deliberate: a GitHub release is immutable
once published, so a published release whose version names an unpullable image is permanent, while
a draft one is a re-run.

## The three guards, and why each exists

Each runs before anything is built, and each catches a failure whose natural symptom is a
*successful* release nobody can pull from.

| guard | catches |
|---|---|
| the tag carries the `micold-ai-ide-v` prefix | a tag-prefix change that silently makes `<version>` the whole tag |
| `[workspace.package] version` equals the tag's version | release-please and the manifest disagreeing, so the image is tagged with a version no client asks for |
| `crates/micold-core/src/sandbox/image.rs` names the namespace being pushed to | exactly the 0.11.0 state — the app looking one place, the release pushing another |

The third is why `DEFAULT_IMAGE_REPOSITORY` exists as its own constant: the workflow needs one
string to grep for, and `DEFAULT_IMAGE` is a `concat!` whose literal half ends in a colon. A unit
test (`the_default_names_the_repository_the_release_publishes_to`) binds the two together, because
a grep that passes against a constant nothing reads is worth nothing.

## Verified here

Run against the real Docker daemon and the real image on this host.

| check | result |
|---|---|
| The version/namespace guard, run with `TAG_NAME=micold-ai-ide-v0.11.0` against this tree | passes — `version=0.11.0`, `ref=ghcr.io/jaroslawherod/micold-daemon:0.11.0-amd64` |
| The same guard against a namespace the source does not name | refuses, as it must |
| `mise run image` with the added `LABEL` layer | builds; one warning, the documented `SecretsUsedInArgOrEnv` false positive |
| `org.opencontainers.image.source` on the built image | `https://github.com/jaroslawherod/micold-ai-ide` — the real remote, which is what GHCR needs to attach the package to this repository |
| The daemon executes inside the image (the release job's loader check) | `fatal: MICOLD_TOKEN_PATH names /run/micold/token but it could not be read` — a reason of its own, not a loader failure, which is exactly what the check distinguishes |
| Both AI CLIs on `PATH` inside the image (FR-023a) | `/usr/local/bin/claude`, `/usr/local/bin/copilot` |
| Workspace gate — `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` | 2595 passed, 0 failed |
| Real-runtime suite, `mise run test-sandbox` (both crates, release), against the rebuilt image | 23 named `sandbox_real_*` tests, 23 passed, 0 failed |

## What the release actually did — 0.12.0, the same day

Everything above was measured on this host before the jobs had ever run. `micold-ai-ide-v0.12.0`
(merge `baa50e9c`, run `33081010155`) ran them for the first time, and all four sections below were
open questions until it did.

| was open | result |
|---|---|
| The release path had never run | `release-please` → `deb` ×2 → `image` ×2 → `image-manifest` → `publish`, all green; release `micold-ai-ide-v0.12.0` published, not a draft, marked Latest, both `.deb` assets attached |
| The three guards had only been run by hand | Step *Resolve the version and check the app agrees with it* passed on both architectures against the real tag |
| arm64 was untested — an npm package with native components can be present on one architecture and absent on the other | *The AI CLIs must be present on this architecture* passed on `ubuntu-22.04-arm`: `claude` and `copilot` are both on `PATH` in the ARM image |
| `docker buildx imagetools create` was unexercised | `ghcr.io/jaroslawherod/micold-daemon:0.12.0` is an index over `linux/amd64` (`sha256:c2c4bbf5…`) and `linux/arm64` (`sha256:edadd300…`); the tag list is `0.12.0`, `0.12.0-amd64`, `0.12.0-arm64` |

## Package visibility — the prediction above was wrong

This document previously carried a section titled *The one manual step*, asserting that a GHCR
package is private when first created and that 0.12.0 would therefore need its visibility flipped by
hand before FR-024 became observable. **It did not.** The package was public the moment it existed:

```sh
tok=$(curl -s 'https://ghcr.io/token?scope=repository:jaroslawherod/micold-daemon:pull&service=ghcr.io' \
      | jq -r .token)
curl -sI -H "Authorization: Bearer $tok" \
     https://ghcr.io/v2/jaroslawherod/micold-daemon/manifests/0.12.0
# HTTP 200 — an anonymous token, which is what an unauthenticated `docker pull` uses
```

The rule the prediction came from is real but is about packages pushed with a personal access token,
which arrive unlinked. The release job pushes with `GITHUB_TOKEN` from a workflow in this
repository, so GitHub creates the package already linked to it, and a linked package inherits the
repository's visibility — public. `org.opencontainers.image.source` is what makes the link legible
on the package page afterwards; it is not what establishes it.

Recording the correction rather than deleting the claim is deliberate: the mistake was to write down
a prediction and a hand-step instruction in the same voice as the measurements around it, where a
reader has no way to tell which had been run. The anonymous-pull check above is the part worth
keeping — it is the only check that distinguishes the two states a first-time user can meet, and
it should be run after any release that creates a *new* package name.

## What this still does not settle

- **A first run against the published image has not been performed end to end.** The manifest is
  pullable anonymously and the daemon executes inside the image (both checked), but no client on
  this host has resolved `DEFAULT_IMAGE` to `:0.12.0`, pulled it from GHCR and completed a session
  against it. The real-runtime suite still builds `micold-daemon:dev` locally.
- **The image is public only for as long as the repository is.** A linked package follows its
  repository, so making this repository private would make a user's first pull answer `denied` —
  the exact symptom of FR-024 being unimplemented, from a change nowhere near this code.
