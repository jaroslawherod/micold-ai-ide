# The sandbox image

The image the session daemon runs in when the sandbox placement is selected (feature 027).

## Build

The image needs a **Linux** `micold-daemon` binary beside the `Containerfile`. `mise run image`
does the whole thing — build the binary, stage it, build the image, tag it `micold-daemon:dev` —
and is the loop FR-024c makes a supported path rather than a maintainer's private trick.

By hand:

```sh
cargo build --release -p micold-daemon --target x86_64-unknown-linux-gnu
cp target-shared/x86_64-unknown-linux-gnu/release/micold-daemon packaging/sandbox/
docker build -t micold-daemon:dev packaging/sandbox/
```

`podman build` works identically; nothing in the `Containerfile` is Docker-specific.

## Publish

**Publishing is the release's job, not a hand step.** The `image` and `image-manifest` jobs in
`.github/workflows/release.yml` build this image for amd64 and arm64 on every release and push:

```
ghcr.io/jaroslawherod/micold-daemon:<version>          # multi-architecture, what the app pulls
ghcr.io/jaroslawherod/micold-daemon:<version>-amd64    # the halves, kept for reproductions
ghcr.io/jaroslawherod/micold-daemon:<version>-arm64
```

`<version>` is the application's own version, because that is what the client compiles into
`DEFAULT_IMAGE` (`crates/micold-core/src/sandbox/image.rs`). The release job checks that agreement
before it builds — the tag must match `[workspace.package] version`, and this repository's GHCR
namespace must be the one that source file names — because when they drift the result is a release
that looks entirely successful and points every first-time user at a tag with nothing behind it.
That was the state of things up to and including 0.11.0, when nothing published an image at all.

Only immutable version tags are pushed. A moving tag (`:latest`) can change under a running
sandbox, which is why the app detects one and treats it differently — see FR-024b.

### Visibility needs no hand step

This was written the other way round before 0.12.0 actually ran, on the widely-repeated rule that a
GHCR package is private when first created and must have its visibility flipped by hand. That rule
does not apply here. 0.12.0 created the package and it came out **public**, pullable with no
credentials at all:

```sh
tok=$(curl -s 'https://ghcr.io/token?scope=repository:jaroslawherod/micold-daemon:pull&service=ghcr.io' \
      | jq -r .token)
curl -sI -H "Authorization: Bearer $tok" \
     https://ghcr.io/v2/jaroslawherod/micold-daemon/manifests/0.12.0   # 200
```

The reason is that the release job pushes with `GITHUB_TOKEN` from a workflow in this repository, so
GitHub creates the package already *linked* to the repository — and a linked package inherits the
repository's visibility, which here is public. The `org.opencontainers.image.source` label in the
`Containerfile` is what makes that link legible afterwards, on the package page.

The check above is the one worth keeping, because it is the only one that distinguishes the two
states a user meets: it uses an anonymous token, which is exactly what an unauthenticated
`docker pull` does. Run it after any release that creates a *new* package name; a private package
answers `denied`, which from outside is indistinguishable from a package that was never pushed —
that is, from the bug FR-024 exists to prevent. **If this repository were ever made private, the
package would follow it**, and that failure would look identical.

## Offline export and import

Principle IV requires the app to work fully offline, and an image reachable only from a registry
would make that nearly-true rather than true. So the offline path is a first-class one (FR-024a):

```sh
# On a machine that can reach the registry
docker pull ghcr.io/jaroslawherod/micold-daemon:<version>
docker save ghcr.io/jaroslawherod/micold-daemon:<version> -o micold-daemon-<version>.tar

# On the machine that cannot
docker load -i micold-daemon-<version>.tar
```

Then point Settings → Daemon → Image at the imported reference. The app never requires a registry
to reach a working sandbox.

## Rebuilding during development

A `:dev` image built yesterday and a client built today share identical `PROTOCOL_VERSION`,
`SCHEMA_HASH` and `PACKAGE_VERSION` — so the handshake's three existing checks cannot tell they
disagree. The build fingerprint added in protocol v6 can, and a stale `:dev` image is refused with
the rebuild command rather than connecting and misbehaving (FR-024d, research R8). If you see
`StaleDevImage`, run `mise run image` again.
