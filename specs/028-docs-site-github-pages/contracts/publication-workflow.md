# Contract: the publication workflow

`.github/workflows/pages.yml`. Two jobs — `build` and `deploy` — two entry points, one deploy.
They are separate because the `github-pages` environment carries the rule that only the default
branch may deploy, and an environment is declared on a job: one job carrying it is rejected
outright when the workflow is dispatched on a branch, before a step of it runs.

## Interface

```yaml
on:
  workflow_call:
    inputs:
      release_tag: { required: true,  type: string }   # build + capture + label from this tag
      docs_ref:    { required: false, type: string }   # prose source; defaults to release_tag
  workflow_dispatch:
    inputs:
      release_tag: { required: false, type: string }   # default: the newest published release
      docs_ref:    { required: false, type: string }   # default: the ref dispatched on

concurrency:
  group: pages
  cancel-in-progress: true          # FR-019: the newer release wins

permissions:
  contents: read
  pages: write
  id-token: write

environment:
  name: github-pages
```

| Input | Release publication | Manual republish |
|---|---|---|
| `release_tag` | the tag just published | newest published release |
| `docs_ref` | the same tag | the ref the dispatch names |

The asymmetry is the whole of FR-017: a republish exists to carry a documentation correction that
landed after the release, so its prose defaults to where that correction is. A maintainer who wants
the tag's prose passes the tag. See research §10 for the tension this admits.

## How it is triggered — and the trap

`release.yml` gains a final job:

```yaml
  pages:
    name: publish site
    needs: [release-please, publish]
    if: ${{ needs.release-please.outputs.release_created == 'true' }}
    uses: ./.github/workflows/pages.yml
    with:
      release_tag: ${{ needs.release-please.outputs.tag_name }}
    permissions: { contents: read, pages: write, id-token: write }
```

**`pages.yml` must not listen for `on: release: [published]`.** `release.yml`'s `publish` job flips
the draft with the workflow's own `GITHUB_TOKEN`, and GitHub does not start a new workflow run from
an event raised by `GITHUB_TOKEN`. Such a trigger would fire in every manual test and never on a
real release. Calling the reusable workflow keeps it in the same run, makes the ordering explicit,
and makes a publication failure a failed job in the release run (FR-018).

Third-party actions are pinned to full commit SHAs with the version in a trailing comment, as
`release.yml` already requires.

## Steps, in order

| # | Step | Fails the publication when |
|---|---|---|
| 1 | Check out the source ref (build source) and `docs_ref` (prose) | either ref is missing |
| 2 | Install capture and site dependencies: `xvfb`, `mesa-vulkan-drivers`, `xdotool`, `imagemagick`, `ffmpeg`, the X11/Wayland dev libraries, and `libxkbcommon-x11-0` (dlopened at startup rather than linked: without it the client panics before it opens a window and every scene fails), mdBook, lychee, Node 20 | any install fails |
| 3 | `cargo build --release -p micold-client -p micold-daemon`, both binaries in one invocation, copied out of the target directory | the build fails, or either binary is absent from the copy |
| 4 | `cargo test -p micold-core site_theme_contrast` | a derived pair fails contrast (FR-033) |
| 5 | Emit `site/theme/css/tokens.css` via `micold-tokens-css` | the emitter fails |
| 6 | Capture every declared asset (`site/capture/`) | any declared asset was not produced (FR-011a) |
| 7 | Encode clips and posters | encoding fails |
| 8 | Stage `docs/` → `src/`, substitute version, release assets, media directives | a directive names an undeclared id |
| 9 | `mdbook build` | the build fails |
| 10 | `links.sh` over the built HTML | any internal link is broken (FR-005, SC-003) |
| 11 | `media-budget.sh` | a page exceeds 1 MB of stills, or a clip exceeds 3 MB (FR-015c) |
| 12 | `page-checks.mjs` — WCAG 2.2 AA both schemes, the SC-001/SC-006 structural proxies, off-origin request scan | any violation (FR-027a, FR-023a, SC-015) |
| 13 | `upload-pages-artifact`, then `deploy-pages` — the deploy only from the default branch or a `workflow_call` | — |

Every check precedes the deploy. A failure at any step means no deploy, so the previously published
site stands untouched and reachable (FR-018). There is no partial-publish state.

The deploy is also the one step a dispatch from a branch does not reach. Everything above it runs —
which is how a change to this pipeline is tried before it is merged — and the built site leaves the
run as the uploaded artifact, to be downloaded and served for review. Without the guard, dispatching
the workflow on a branch would put that branch's site in front of every reader, silently.

## The dispatched ref

A dispatch takes `docs_ref` from the ref it was dispatched on, not from the default branch. The two
agree for the ordinary case -- republishing the prose from `main` -- and they part company for the
one this exists to serve: dispatching on a branch to try a change to this pipeline before merging
it. Defaulting to the default branch there built `main`'s `site/` and reported on a change that was
not in the run at all, which is a green or red that means nothing.

## The source ref

The site's own machinery — `site/`, the scenes, the checks, and the token-to-CSS emitter step 5
compiles — normally comes from `release_tag`, so the pictures are of the release the pages describe.
`docs_ref` overlays `docs/` alone on top of it, and that asymmetry is FR-017.

A release published before this feature existed carries none of that machinery, so the source ref
falls back to `docs_ref` and the run says so. There is exactly one such publication, the first: every
tag cut from here on contains `site/`, so the condition cannot be true again. It is a branch in the
resolve step rather than an input precisely because nobody should have to remember it.

## Independence from the merge pipeline

The workflow is triggered by a release, not by `ci-complete`, and it builds its own copy of the
application. A documentation-only change that skipped the three-platform matrix is publishable
exactly like any other (FR-020). Nothing in this workflow is added to `ci-complete`'s `needs:`, and
`ci.yml`'s job set is unchanged — the new pre-merge checks are steps inside the existing `docs` job,
so `ci_gate_covers_every_job.rs` and the default branch's required-check name are both untouched.

## Prerequisite, outside this change

Pages must be enabled on the repository with source **GitHub Actions** before the first run. Until
the first successful publication the address serves GitHub's own 404 — which is the edge case's
"unpublished", since nothing is deployed.
