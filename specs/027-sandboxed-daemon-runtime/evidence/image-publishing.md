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

## What this does not settle

- **The release path itself has not run.** Everything above tests the pieces on this host; the jobs
  are exercised for the first time by the release after this one lands. That is unavoidable — a
  release workflow has no dry run — and it is why the guards fail loudly and early rather than
  producing a half-pushed image.
- **arm64 is untested here.** This host is x86_64. The per-architecture CLI check exists precisely
  because an npm package with native components can be present on one and absent on the other, and
  that check runs for the first time on the arm64 runner.
- **`docker buildx imagetools create` is unexercised.** Composing the index needs two pushed
  per-architecture manifests, which needs the registry.
- **Package visibility is not covered by anything.** See below.

## The one manual step

A GHCR package is **private** when first created. A private package answers a user's first pull
with `denied` — indistinguishable, from outside, from a package that was never pushed, which is to
say indistinguishable from the bug this whole phase fixes. There is no API that sets visibility at
push time.

After the first release that runs these jobs, the package at
`github.com/users/jaroslawherod/packages/container/micold-daemon/settings` must be switched to
**Public**, once. Later releases inherit it. `packaging/sandbox/README.md` carries the same
instruction where a maintainer will look for it.

Until that switch is made, FR-024 is implemented but not yet observable: the image exists and the
reference is correct, and an anonymous pull still fails.
