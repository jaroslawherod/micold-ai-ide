# Contract: Link recognition

**Feature**: 031-clickable-terminal-links | **Module**: `micold_core::link` (new, render-free) |
**Research**: R2–R5, R8, R10

What the pane asks core, and what core promises back. Everything here is pure, so every clause is a
unit test in `mise run test-core`.

## 1. Surface

```text
pub trait LinkRows {
    fn text(&self, row: i64) -> Option<&str>;                 // None: row not available
    fn wrapped(&self, row: i64) -> bool;                      // this row soft-wraps into row + 1
    fn hyperlink(&self, row: i64, col: u16) -> Option<&str>;  // declared URI at a cell
    fn spacer(&self, row: i64, col: u16) -> bool;             // the cell holds no char of its own
}
pub fn link_at(rows: &impl LinkRows, row: i64, col: u16) -> Option<Link>
pub fn detect(text: &str) -> Vec<Range<usize>>          // char-index ranges, FR-001/FR-005
pub fn classify(address: &str) -> Address               // FR-011
pub fn resolve(link: Link, ctx: &LinkContext) -> Option<ResolvedLink>   // FR-008, FR-012, FR-018
```

`row` is relative to the viewport's top line, so it may be negative (scrollback) or at least the
viewport height (below it). The client implements `LinkRows` over `GridCache` as
`LineId(viewport_top - display_offset + row)` (as the pane already computes it, `terminal_pane.rs:545`), which reaches every cached line, not only the visible ones. `text` is
`None` for a line the cache does not hold. Types are in [data-model.md §1](../data-model.md).

## 2. `link_at` — which link is under a cell

| # | Given | Then |
|---|---|---|
| L1 | the cell carries a declared URI | the link is the maximal run of cells with that exact URI over the logical line; `origin = Declared`; detection is not run |
| L2 | the cell carries no URI and lies inside a `detect` range of the logical line's text | that range mapped back to cells; `origin = Detected` |
| L3 | neither | `None` |
| L4 | the logical line | rows joined while the upper row has `wrapped`, at most 64 rows before and after the pointer row, reading beyond the viewport when the line continues there; a candidate reaching the cap is dropped |
| L5 | a wide character | its spacer cell belongs to the same link as its lead cell. A spacer (`spacer` is `true`: `WIDE_CHAR_SPACER`, or `LEADING_WIDE_CHAR_SPACER` at a row's end) is left out of the logical line's text and covered by the char before it on its row |
| L6 | two runs with the same URI separated by any other cell | two links |
| L7 | the walk stops at a row whose `text` is `None`: below a row that has `wrapped`, or above the first available row | a candidate touching that boundary is dropped: nothing but trailing punctuation (research R3 rule 3) lies between its end and the last column of the lower row, or nothing but characters an address may contain (research R3 rule 2) lies between column 0 of the upper row and its start, so it may sit inside an address that began above. The same drop applies at the L4 cap. A truncated address is never recognised (FR-003) |

## 3. `detect` — examples (all in the SC-002 corpus)

| Text | Link text |
|---|---|
| `See https://example.com/docs/page.html for details.` | `https://example.com/docs/page.html` |
| `(https://example.com/a_(b))` | `https://example.com/a_(b)` |
| `"https://example.com"` | `https://example.com` |
| `[text](https://x.y/z)` | `https://x.y/z` |
| `<https://x.y>` | `https://x.y` |
| `dev server at http://localhost:5173/` | `http://localhost:5173/` |
| `mailto:team@example.com,` | `mailto:team@example.com` |
| `file:///home/u/My%20Doc.pdf` | `file:///home/u/My%20Doc.pdf` |
| `example.com`, `team@example.com`, `src/main.rs:42` | *(none — FR-001)* |
| `https://`, `http://exa mple`, `javascript:alert(1)`, `data:text/html,x` | *(none)* |
| `xhttps://example.com` | *(none — rule 1)* |

## 4. `classify` and `resolve`

| # | Address | Context | `ResolvedLink` |
|---|---|---|---|
| C1 | `https://…`, `http://…` | any | `target = Url(address)`, `display = address` |
| C2 | `mailto:a@b.c` | any | `target = Url(address)`, `display = address` |
| C3 | `vscode://…`, `slack://…`, `zoommtg://…`, `javascript:…`, `data:…`, `vbscript:…`, other | any | `None` |
| C4 | `file:///home/u/a.txt` | not sandboxed, not Windows | `HostPath("/home/u/a.txt")`, `needs_confirmation = false` |
| C5 | `file://localhost/…`, `file://<host_names[i]>/…` (any case) | not sandboxed | as C4 |
| C6 | `file://otherhost/…`, `file://server/share/…` | any | `None` (FR-012) |
| C7 | `file:///C:/Users/u/a.txt` | not sandboxed, Windows | `HostPath("C:\Users\u\a.txt")` |
| C8 | `file:///home/u/a.txt` | not sandboxed, Windows | `None` (no drive) |
| C9 | `file:///p/My%20Doc.pdf` | any | path decoded to `/p/My Doc.pdf` before C4–C12 |
| C10 | `file:///p/%ZZ`, or non-UTF-8 after decoding | any | `None` |
| C11 | `file:///work/proj/a.md`, or `file://<container-id-prefix>/work/proj/a.md` (the prefix is `id.get(..12).unwrap_or(id)`, so an id shorter than 12 is its own prefix) | sandboxed; a location `/work/proj` → `/home/u/proj` | `HostPath("/home/u/proj/a.md")`, `display` the same, `needs_confirmation = true` |
| C12 | `file:///tmp/x` | sandboxed; no location contains `/tmp` | `Unreachable`, `display = "/tmp/x — not reachable from this machine"` |
| C13 | `file:///work/proj/../../etc/passwd` | sandboxed | lexically `/etc/passwd`; C12 unless a location contains it |
| C14 | `file:///mnt/host/c/Users/u/p/a.md` | sandboxed, Windows host; location `/mnt/host/c/Users/u/p` → `C:\Users\u\p` | `HostPath("C:\Users\u\p\a.md")` |
| C15 | nested locations `/home/u` → `<state>/sandbox-home` and `/home/u/.claude` → `/home/u/.claude` | sandboxed | `/home/u/.claude/x` → `/home/u/.claude/x` (most specific) |
| C16 | the secret mount's container path | sandboxed | C12 (never a shared location) |
| C16b | `file:///var/lib/micold-ai-ide/sandbox.token` | sandboxed; the state location `/var/lib/micold-ai-ide` → `<state>`; `denied = [<state>/sandbox.token]` | C12: the reverse-mapped path is denied |
| C16c | `file:///srv/other/a.md` | sandboxed; attached to a container whose `mounted` lacks `/srv/other`, which this client's `MountSet` lists as a project; no other location contains it | C12. (A project under the home container path would instead fall to the home location, C15, and resolve to `<state>/sandbox-home/…`, where the container really wrote it) |
| C17 | `file://<host_names[i]>/…` | sandboxed | accepted by FR-012's host rule. The path is still a container path, so it goes through C11–C16c |
| C18 | `file://build-host.invalid/…` where that name is unresolvable and not in `host_names` | any | `None`, decided with no lookup: `link` has no I/O or network dependency (FR-019) |

**SC-006 invariant**: `display` equals the string handed to the opener for every `Url` and
`HostPath` result.
