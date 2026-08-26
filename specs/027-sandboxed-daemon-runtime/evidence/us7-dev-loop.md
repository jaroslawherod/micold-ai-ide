# US7 — the development loop, against real images

Quickstart §B.7 (FR-024c, FR-024d, research R8). Run on 2026-08-26 on the branch
`feat/run-daemon-inside-an-container-sandbox`, against Docker 29.5.1 (storage driver `overlayfs`)
on Linux.

§B.7 is the only part of the manual pass whose subject is the *developer*, not the user, and it is
the part most easily assumed rather than checked: everyone who works on this feature builds a `:dev`
image, so "it obviously works" is exactly the belief that lets it stop working. It did not obviously
work — see box 3.

## Box 1 — `mise run image` builds a `:dev` image from the working tree

```
#9 naming to docker.io/library/micold-daemon:dev done
#9 unpacking to docker.io/library/micold-daemon:dev 0.0s done
Built micold-daemon:dev -- select it in Settings > Daemon > Image.
EXIT=0
```

The task cross-compiles `--release --target x86_64-unknown-linux-gnu`, copies the binary beside the
`Containerfile`, builds it, and smoke-runs the result so a loader failure surfaces here rather than
as a container that exits on start. One warning is emitted and is expected:
`SecretsUsedInArgOrEnv … ENV "MICOLD_TOKEN_PATH"` — that variable holds a *path*, not a secret; the
token itself arrives as a read-only bind mount (FR-005).

## Box 2 — running against it works, with the staleness check armed

The client arms `require_fingerprint_match` for a `LocalBuild` image
(`micold-client/src/shell/startup.rs`, `image.refuses_fingerprint_mismatch()`). So "running against
it works" is not the same claim as the lenient handshake already covered by
`sandbox_real_handshake.rs`: it additionally requires that the fingerprint the image carries equals
the one the client computes from the tree it was built from.

That is now a standing test rather than a one-off, because it can fail silently on a stale layer
cache or a `build.rs` that did not re-run:

```
$ cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_a_freshly
running 1 test
test sandbox_real_a_freshly_built_image_passes_the_strict_fingerprint_check ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
```

`crates/micold-daemon/tests/sandbox_real_fingerprint.rs`.

## Box 3 — a stale image is refused, naming the tag and the rebuild command

**This is where the feature was broken, in two places, and neither was reachable without a real
image.**

*The refusal never reached the developer as advice.* `StaleDevImage` fell through
`micold-client/src/daemon.rs`'s catch-all refusal arm and was reported as a `{:?}` dump — two hex
strings and a tag as debug noise, with no rebuild command — to an audience that is by definition
mid-rebuild and has a one-line remedy. It now has its own arm and renders:

> the sandbox is running `micold-daemon:dev`, built from a different working tree than this client
> (image f7399c7084d61f60, client 3ae4daf28e819a0f). Rebuild it with `mise run image`, then restart
> the sandbox.

Held by `daemon.rs::the_stale_image_advice_names_the_tag_and_the_rebuild_command`.

*And the tag in it was empty.* The daemon fills the `image` field from `MICOLD_IMAGE_REFERENCE`
(`micold-daemon/src/state.rs`) and has no other way to learn it — a container cannot see the
reference it was created from. Nothing ever set that variable. Every `StaleDevImage` therefore
carried `image: ""`, so the sentence above would have been missing precisely the word the developer
needs to act on. `sandbox::argv::create` now passes it, and
`the_image_reference_is_passed_in_so_a_stale_image_can_name_itself` holds it at the argv layer.

The refusal itself, measured against a real container. The staleness here was not synthetic: the
argv fix above is itself a change to `micold-core/src/`, so the image built before it *is* stale
with respect to the tree that contains it, and the strict handshake said so —

```
thread 'sandbox_real_a_freshly_built_image_passes_the_strict_fingerprint_check' panicked at
crates/micold-daemon/tests/sandbox_real_fingerprint.rs:75:49:
a `:dev` image built from this very tree was refused as stale: StaleDevImage {
  client_fingerprint: "3ae4daf28e819a0f",
  daemon_fingerprint: "f7399c7084d61f60",
  image: "micold-daemon:dev" }
```

— and after `mise run image`, the same connection was accepted (box 2). That is both directions of
the check against one image, which is what §B.7 asks for.

An earlier synthetic run reached the same refusal by appending `pub const _B7_PROBE: u8 = 1;` to
`micold-core/src/lib.rs` and reverting afterwards. Worth recording because the *first* attempt at
that probe passed and proved nothing: the edit was a comment, and `BUILD_FINGERPRINT` hashes
`micold-core/src/` **canonicalized** — `hashing.rs::canonicalize` drops blank lines and `//`
comments, so a comment-only change does not move the fingerprint and must not be used to fake
staleness.

## Box 4 — `docker save`/`load` round-trip, and enabling with the network off

**The round-trip is verified; "the network off entirely" is not, and is stated here as unverified.**

The round-trip is covered as a standing test rather than by hand:
`crates/micold-core/tests/sandbox_real_enable.rs` saves the image to a tar, `docker rmi`s it so the
local store genuinely lacks it, acquires it back through `ImageSourceKind::ImportedFile` — the
documented no-network procedure of SC-004a — and then creates, starts, and connects to a daemon
from the restored image. It passes (see `evidence/performance.md` for its timings). The archive is
68 MB.

What could not be checked on this machine is the literal instruction "with the network off
entirely": taking the host off the network would sever the session this ran from, and the ordinary
substitutes do not test the right thing — running the *client* in a network namespace of its own
gives it a different loopback, so it could not reach the published control port for reasons that
have nothing to do with the image.

What *was* measured is the part of the claim that is about registries, and it is the part that could
plausibly be false:

```
$ docker pull micold-daemon:dev
Error response from daemon: pull access denied for micold-daemon, repository does not exist
  or may require 'docker login'

$ docker create --pull never --name micold-offline-probe micold-daemon:dev
created with --pull never: OK
```

No registry can serve this tag, so every success above was served from the local store; and the
create path works under `--pull never`, which forbids consulting a registry at all. Combined with
the enable test starting from a store that does not contain the image, the offline claim holds for
everything except a network outage's effect on the runtime daemon itself, which remains unchecked.

## What this leaves standing

- Boxes 1–3: verified, and two defects found and fixed in the course of it.
- Box 4: round-trip verified; the "network off entirely" half is **unverified** for the reason
  above. Principle IV's offline claim should be read as "consults no registry", which is measured,
  rather than "works with the machine offline", which is not.
