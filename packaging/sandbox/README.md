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

```sh
docker tag micold-daemon:dev ghcr.io/<org>/micold-daemon:<version>
docker push ghcr.io/<org>/micold-daemon:<version>
```

Publish an **immutable version tag**. A moving tag (`:latest`) can change under a running sandbox,
which is why the app detects one and treats it differently — see FR-024b.

## Offline export and import

Principle IV requires the app to work fully offline, and an image reachable only from a registry
would make that nearly-true rather than true. So the offline path is a first-class one (FR-024a):

```sh
# On a machine that can reach the registry
docker pull ghcr.io/<org>/micold-daemon:<version>
docker save ghcr.io/<org>/micold-daemon:<version> -o micold-daemon-<version>.tar

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
