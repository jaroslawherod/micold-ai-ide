# Contract: the media manifest and its directive

Two halves of one contract. `site/media.toml` declares what the publication must produce; an HTML
comment in the prose declares where it appears. `site/checks/media-references.sh` holds them to each
other in both directions, before merge (FR-022, FR-011a).

## The directive, in the prose

```markdown
Opening a project shows its worktrees in a sidebar on the left.

<!-- media: worktree-sidebar-light -->
```

- Exactly one directive per line, nothing else on that line.
- The id matches `[a-z0-9]+(-[a-z0-9]+)*`.
- It renders as nothing on GitHub, which is the point: the media does not exist in the repository,
  so a Markdown image link would render broken on every page.
- `site/stage.sh` replaces the line with the figure markup — `<img>` for a still, a poster-plus-
  `<video>` with a play control for a clip — reading alt text and caption from the manifest.

## The manifest

```toml
# site/media.toml — the declared capture set. Code, not documentation:
# it is a build input, so a change here takes the full pipeline.

[media.worktree-sidebar-light]
kind   = "still"
scene  = "worktree-sidebar"
scheme = "light"
alt    = "The application with a project open. A sidebar on the left lists three worktrees; the second is selected."
caption = "Worktrees for one project, in the light theme."

[media.session-terminal-dark]
kind   = "clip"
scene  = "session-terminal"
scheme = "dark"
alt    = "A session starting in a worktree: the terminal opens, the provider greets, and output scrolls."
caption = "Starting a session in a worktree."
```

### Fields

| Field | Required | Values | Enforced by |
|---|---|---|---|
| `kind` | yes | `still` \| `clip` | manifest parse |
| `scene` | yes | a file `site/capture/scenes/<scene>.sh` that exists | `media-references.sh` |
| `scheme` | yes | `light` \| `dark` | manifest parse |
| `alt` | yes | non-empty | `media-references.sh` (FR-014, SC-009) |
| `caption` | no | free text | — |

### Rules

1. Every directive id in `docs/**/*.md` is a key in `[media.*]`. A directive with no entry fails.
2. Every `[media.*]` key is referenced by at least one page. An orphan entry fails — an unreferenced
   capture is a capture nobody notices has broken.
3. Every `scene` names an existing script. A missing scene fails.
4. `alt` is present and non-empty for every entry.
5. Where a page's subject is appearance, the same view is declared in both schemes (FR-012).

Rules 1–4 run pre-merge in CI's `docs` job. The publication additionally fails if any declared entry
was not produced (FR-011a) or exceeds its budget (FR-015c).

## Output paths

Produced into the staging tree, never into `docs/`:

```text
<staging>/media/<id>.png          # a still, or a clip's poster
<staging>/media/<id>.mp4          # a clip (H.264, muted)
<staging>/media/<id>.webm         # a clip (VP9, muted)
```

## Figure markup

A still:

```html
<figure class="media"><img src="media/<id>.png" alt="<alt>" loading="lazy" width="…" height="…">
<figcaption><caption></figcaption></figure>
```

A clip — `preload="none"` is load-bearing for FR-028 (nothing moving is fetched until the reader
asks), and the absence of `autoplay` is load-bearing for FR-015a:

```html
<figure class="media"><video controls loop muted playsinline preload="none"
  poster="media/<id>.png" aria-label="<alt>" width="…" height="…">
  <source src="media/<id>.webm" type="video/webm">
  <source src="media/<id>.mp4" type="video/mp4">
</video><figcaption><caption></figcaption></figure>
```
