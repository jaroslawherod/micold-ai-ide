# FR-018: a failed check does not deploy

**Date**: 2026-09-13 · **Workflow**: `.github/workflows/pages.yml`, dispatched on `main` ·
**Release**: `micold-ai-ide-v0.13.1` · **Prose**: branch `test/028-t084-broken-link` at `b150fdd6`

Quickstart A9's second half, carried as T084: introduce a deliberate broken link, dispatch, and
confirm the run fails at the link check and the live site is unchanged.

## The break

One commit on top of `main` (`3cc50284`), on a throwaway branch, appending to
`docs/user-guide/appearance-theming.md`:

```markdown
This sentence links to [a page that does not exist](t084-deliberately-missing.md) (T084).
```

It was dispatched as the prose, not the source, so the tag's `site/` built it exactly as a release
would:

```sh
gh workflow run pages.yml --ref main -f docs_ref=test/028-t084-broken-link
```

`release_tag` took its default, the newest release. `links.sh --sources` on that branch reports the
same single error locally, so the break would also have been refused before a merge.

## The run: `34754847522`

| Job / step | Result |
|---|---|
| publish site → Resolve the release tag and the prose ref | success |
| publish site → Take the prose from docs_ref | success |
| publish site → Build and check the site | **failure** |
| publish site → upload-pages-artifact | skipped |
| deploy site | **skipped** |

Every stage before the check ran and passed. `capture.sh: 12 media`, `stage.sh: staged … at version
0.13.1`, and the mdBook render all succeeded. Media completeness passed, then the internal link
check failed:

```text
-- internal links
[ERROR] file:///…/site/book/user-guide/t084-deliberately-missing.html (at 387:33) | File not found.
🔍 772 Total 🔗 397 Unique ✅ 695 OK 🚫 1 Error 👻 76 Excluded
##[error]Process completed with exit code 2.
```

The one error is the deliberate one and nothing else. That ruled out a coincidental failure (see
below).

## The live site, before and after

Each page was fetched with a cache-busting query. The deployment is the newest in the
`github-pages` environment.

| | Before the dispatch | After the run |
|---|---|---|
| `/` | 200 · sha256 `aeffbdf71a7d0e0c` · etag `"6aa68a5f-88dd"` · 0.13.1 | identical |
| `/user-guide/appearance-theming.html` | 200 · sha256 `16682f36baef21b2` · etag `"6aa68a5f-a21f"` · 0.13.1 | identical |
| occurrences of `t084-deliberately-missing` | 0 | 0 |
| newest deployment | `6421068071`, 2026-09-13T11:34:49Z, `3cc50284` | unchanged |

## What it took to get a clean run

Three earlier attempts failed for reasons other than the break. They are recorded because each one
is itself a failed build that did not deploy:

- **`34749228323`, `34751672497`** failed at the media capture. The demonstration project's
  worktrees had no provenance record, so feature 029's sidebar hid them and six media were never
  produced. Fixed in #296. The second attempt still failed because the newest release, 0.13.0,
  predated that fix.
- **Release run `34753074922`** (0.13.1) captured all 12 media and then failed at the internal link
  check. `docs/development/macos-packaging.md` linked into `specs/`, which the site does not stage.
  `links.sh --sources` passes such a link, because it resolves in the repository. Fixed in #303.
  After that, run `34754269022` published the site that the "before" column above measures.

Deploy was skipped in all three, and the live site did not change in any of them.
