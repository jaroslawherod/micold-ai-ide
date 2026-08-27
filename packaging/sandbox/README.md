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

### The one manual step

A GHCR package is **private** when it is first created, and a private package is a `denied` on a
user's first pull — indistinguishable, from the outside, from a package that was never pushed. The
first release to publish the image therefore needs its visibility flipped once, by hand, at
`github.com/users/jaroslawherod/packages/container/micold-daemon/settings` → *Change visibility* →
Public. Later releases inherit it; there is no API that sets it at push time.

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
