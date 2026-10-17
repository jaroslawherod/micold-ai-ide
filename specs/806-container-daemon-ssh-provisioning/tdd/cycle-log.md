# TDD cycle log: 806

The feature has no `tdd/test-list.md` (as 937 and 946): tasks.md's test tasks are the list, and each
row names the task pair it covers.

## M1 — basic image

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001, T004–T006 | `scripts/basic-image-hash.sh`: 16 lowercase hex, stable, order-independent, moves on one byte and on a rename; `--check` → `CURRENT` / `DRIFT <pinned> <tree>`, exit 0; the repository pin matches the tree; the Containerfile copies no key, credential, `.ssh`, token or daemon and runs no `npm install` | `bash scripts/tests/basic-image-hash.test.sh`: `8 case(s), 7 failure(s)` — the script and `packaging/basic/Containerfile` did not exist (`…/scripts/basic-image-hash.sh: No such file or directory`; `…/packaging/basic/Containerfile does not exist`). Only "changes when one byte changes" passed vacuously (two error strings differ) | `8 case(s), 0 failure(s)` after T004–T006 and the T007 pin (now `722a7d9ef0d6aadb`) |
| T002, T007 | `is_released_image` true for both released repositories under no tag / any tag / any digest; false for the basic image, `micold-daemon:dev`, `micold-basic:dev`, alpine, empty; `BASIC_IMAGE_CONTENT_HASH` is 16 lowercase hex | `cargo test -p micold-core --lib sandbox::basic_image` against a `todo!()` body and an empty hash: 3 failed (`not yet implemented` ×2; `assertion left == right failed: ""`). `the_released_repositories_are_the_published_and_the_retired_one` passed at declaration (the constant was written with the module, not red-first) | `cargo test -p micold-core --lib sandbox::`: 134 passed |
| T003, T004 | `sandbox_real_basic_image_has_no_daemon_or_cli`: none of `micold-daemon claude codex gemini opencode pi`; all of `sshd node npm git flock`; glibc ≥ 2.41. Named with the `sandbox_real_` prefix so CI's name filter runs it (the task's `basic_image_has_no_daemon_or_cli` would be skipped there) | `MICOLD_BASIC_IMAGE_TAG=docker.io/library/node:24-trixie-slim cargo test -p micold-core --features sandbox-real-runtime --test sandbox_real_basic_image`: 1 failed, `docker.io/library/node:24-trixie-slim lacks ["sshd", "git"]` (the base image, before T004's packages) | Not run green locally: this container's egress refuses `deb.debian.org`, so `micold-basic:dev` cannot be built here. CI's `sandbox-runtime` job builds it (T009) and runs the test |
| T008, T009 | workflows | structural, no unit behaviour; `basic-image.yml` parses as YAML; it runs on this PR (`pull_request` on `packaging/basic/**`) and builds both arches | CI |
