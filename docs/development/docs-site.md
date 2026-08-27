# The documentation site: how a publication works

<https://cumulocity-iot.github.io/micold-ai-ide/> is this `docs/` directory, rendered. It is
published by a workflow, from a release tag, with every screenshot on it captured from the
application built at that tag — so the site a reader lands on shows the version they can download,
and nobody has to remember to refresh a picture.

This page is for the developer who has to change, trigger, or debug that. The tooling itself lives
under `site/`, and [`site/README.md`](https://github.com/Cumulocity-IoT/micold-ai-ide/blob/main/site/README.md)
is the map of what is in there.

## What runs, in order

A publication is one script — `site/build.sh` — and five steps that only make sense in this order:

1. **Emit the theme.** `micold-tokens-css` writes `site/theme/css/tokens.css` from the same design
   tokens the application renders with. It is generated on every build and never committed, so the
   site cannot drift into looking like a different product.
2. **Capture the media.** Every entry in `site/media.toml` is produced by launching the real client
   against a demonstration project on a private X display. Nothing is drawn by hand.
3. **Stage.** `site/stage.sh` copies `docs/` into `site/build/src`, substitutes the version and the
   download links, and expands each `<!-- media: id -->` directive into the picture, its alt text
   and its caption.
4. **Render.** mdBook turns the staged tree into `site/book`.
5. **Check.** The built site is checked before anything is deployed.

Step 5 is why the workflow runs the script rather than restating the pipeline in YAML: the deploy
step uploads what `build.sh` leaves behind, so nothing can reach a reader that has not been through
the checks.

Locally:

```bash
mise run site-build          # all five steps
site/build.sh --no-media     # skip the capture — the fast loop for prose and theme work
mise run site-check          # the checks alone, against the site already built
```

`--no-media` leaves whatever the last capture produced, which is fine for iterating and wrong for a
publication. `MICOLD_SITE_STRICT=1` — which the workflow sets — refuses the combination outright.

## The checks, and what each one catches

Three run **before a merge**, as steps inside the existing `docs` job of `ci.yml`, where the author
is still looking at the change. Three more run **before a deploy**, where the failure modes are the
renderer's and the capture's rather than the author's. None of them repairs anything: a check that
quietly fixed its input would publish something nobody wrote.

| Check | When | Catches |
|---|---|---|
| `site/checks/page-set.sh` | pre-merge | a page under `docs/` that no `SUMMARY.md` entry lists (it would never be rendered), an entry naming a file that is not there, and any value in `site.css` written as a literal instead of a `--micold-*` variable |
| `site/checks/media-references.sh` | pre-merge | a `<!-- media: id -->` directive naming an id the manifest does not declare, a declared id no page uses, a scene script that was renamed out from under the manifest, and blank alt text |
| `site/checks/links.sh --sources` | pre-merge | an internal link to a page or a heading that does not exist — in the Markdown, before anything is rendered |
| `site/build.sh`'s completeness assertion | pre-deploy | a declared medium missing from the built site, or one older than this run's capture — the way a stale picture would otherwise survive |
| `site/checks/links.sh --built` | pre-deploy | the same links in the rendered HTML, where the ids are the renderer's own and an author's guess at a fragment can be wrong |
| `site/checks/media-budget.sh` | pre-deploy | a page whose still images exceed 1 MB in total, or a clip over 3 MB |
| `site/checks/page-checks.mjs` | pre-deploy | WCAG 2.2 AA violations on the rendered page in both colour schemes, anything loaded off-origin, and video that would play by itself |

`cargo test -p micold-core site_theme_contrast` runs alongside them: it checks the *token pairs*
the theme is derived from, which is the half a rendered-page check cannot reach.

**No external URL is ever fetched.** `lychee` runs with the network off by design — a publication
that fails because somebody else's site is down is a publication nobody trusts, and the failure
would arrive months after the link was written.

## The trigger

`release.yml` calls `.github/workflows/pages.yml` as a reusable workflow, after release-please has
published a release and the packages are up, passing the tag it just created.

**It is deliberately not `on: release: [published]`.** A release created by a workflow authenticates
as `GITHUB_TOKEN`, and GitHub does not start workflow runs from events raised by that token. The
release-event form looks correct, passes review, and then silently never publishes anything. A
`workflow_call` from the workflow that made the release has no such rule.

The site is therefore only ever as new as the last release — which is the point. The version badge,
the download links and the screenshots all describe one shipped version.

## Republishing without a release

Both inputs are optional on a manual run:

```bash
gh workflow run pages.yml                                            # newest release + default branch prose
gh workflow run pages.yml -f release_tag=micold-ai-ide-v0.10.0       # that tag, prose included
gh workflow run pages.yml -f docs_ref=main                           # newest release, prose from main
```

- `release_tag` decides which application is built and captured. It defaults to the newest published
  release.
- `docs_ref` decides where the prose comes from, and defaults to the default branch on a manual run
  — so a typo corrected after a release can be published without cutting a new one. On the automatic
  run it defaults to the release tag, so a publication is reproducible from the tag alone.

Runs are serialised with `concurrency: {group: pages, cancel-in-progress: true}`: two releases in
quick succession cannot interleave, and the newer one wins.

## When a publication fails

The deploy is the last step, and every check is before it, so **a failed run leaves the previous
site up.** There is no half-published state to clean up: fix the cause and run the workflow again.

The commonest causes, in rough order:

- **A new page with no `SUMMARY.md` entry.** `page-set.sh` fails before the merge. Add the entry.
- **A renamed scene.** `media-references.sh` names the id and the scene file it could not find.
- **A picture over budget.** `media-budget.sh` prints every asset on the offending page with its
  size and the total. Nothing is downscaled for you.
- **A capture that did not happen.** The completeness assertion names the id and the missing file.
  Locally this usually means `--no-media`; in CI it means the scene failed, and the run's log has
  the scene's own output above it.

Reproduce any of them locally with `mise run site-check`, which runs the pre-deploy checks against
`site/book` without rebuilding.

## What is not automated

Enabling GitHub Pages with source **GitHub Actions** is a repository setting, done once. Until the
first successful publication the address serves GitHub's own 404 page.
