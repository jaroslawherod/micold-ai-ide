# UI contract: feature 1793

## Top bar (selected worktree/session), left to right after the search field
ring `8% · $1.28` -> status `Idle`/`Working` -> `Changes · no changes` -> worktree `⋮` -> Settings -> app `⋮`. None of the first four on Overview.

## Tab row (40 px)
session tabs (activity dot when working) + shell tabs -> `+` (menu: Another session / Terminal) -> `N hidden` chip (only when >0). Nothing else above the first event.

## Preservation table (FR-012)
| Before | After |
|---|---|
| title row | breadcrumbs only |
| `Open in terminal` button / strip switch slot | worktree `⋮` segmented view control |
| `Show raw events` toggle | worktree `⋮` |
| hidden-events notice `Show` | `N hidden` chip |
| new-session icon + `+` terminal icon | single `+` menu |
| meter / totals / running | ring + tooltip |
| `running` text | tab activity dot |
| Changes button | top bar |

## Accessibility (FR-013)
`+`, worktree `⋮`, ring and chip are focusable with names: "Add session or terminal", "Worktree actions", the ring's tooltip text, "N hidden events, show". Menus close on Escape/outside click.
