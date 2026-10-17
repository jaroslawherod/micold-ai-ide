# Contract: the daemon image source (shared with #806)

The image-source part of this feature, kept separable (ledger Decisions 1-2 are orchestrator defaults
pending the owner): a different FR-004 answer replaces this contract, `micold-core/src/daemon_image/`
and `micold-client/src/kube/image.rs`, and nothing else. Decisions: [../research.md](../research.md)
K1-K3.

## Choices

| Choice | Form fields | Kubernetes behaviour |
|---|---|---|
| **Build** (default) | Dockerfile (optional file; empty = the app's template), AI provider CLIs (checkboxes, default = the providers enabled in Settings), **Registry** (required for Kubernetes) | build with the local runtime, push to the registry, pod pulls by digest |
| **Pull** | Image reference | pod uses it as given; the cluster pulls it, with the `RegistryPull` secret when allowed |

No published release image is offered or defaulted (FR-004). Resources (CPU, memory requests and
limits) are the existing `ResourceBudget` fields in the same section.

## Build

1. Daemon binary: Linux, `KubeSettings.arch`, chosen by 489's artefact rule (own on a matching Linux
   client, else `<install>/remote-daemons/linux-<arch>/micold-daemon`); absent →
   `no daemon build for linux-<arch> ships with this app`.
2. Context: a fresh temporary directory with `micold-daemon` and, for the template, the rendered
   `Containerfile`. With a user Dockerfile: its directory is the context and the temporary directory
   is passed as `--build-context micold=<tmp>`; the Dockerfile copies `--from=micold micold-daemon`.
3. Tag: `<registry>/micold-daemon:<version>-<inputs12>` (data model `ImageInputs`).
4. `<runtime> image inspect <tag>` present → skip build; else
   `<runtime> build --platform linux/<arch> -f <file> -t <tag> <context>`.
5. `<runtime> push <tag>`; then `<runtime> image inspect --format '{{json .RepoDigests}}' <tag>` →
   the digest for `<registry>/micold-daemon`; none → `Push` failure "the registry returned no
   digest".
6. Output of 4-5 streams to the daemon's log; bound 30 min from step 4; Cancel kills the child.

## Template invariants (tested)

The rendered Containerfile installs exactly the selected providers' npm packages at the versions
pinned in `packaging/sandbox/Containerfile`, creates UID/GID 10001, sets `USER 10001`,
`HOME=/data/home`, `XDG_DATA_HOME=/data/state`, contains no `COPY` or `ENV` of any credential, and
ends with the daemon as `ENTRYPOINT`. A test fails when an `AiCli` has no pinned `ARG` in
`packaging/sandbox/Containerfile`.

## Rebuild rule

No rebuild on edit. On start, a changed provider set, Dockerfile content, architecture or client
version yields a new tag, hence a build and push; an unchanged one yields the present tag and only a
push (no layer transfer).
