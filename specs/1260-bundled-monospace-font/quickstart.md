# Quickstart: Bundled monospace typeface

Validation guide for feature 1260. The tests named here are in [plan.md](./plan.md#test-strategy-by-layer);
interfaces are in [contracts/fonts.md](./contracts/fonts.md).

## Prerequisites

- A checkout of this branch, `mise trust` done once.
- For §B: a Linux desktop session. For the same-name check (B4), permission to install a user font
  (`~/.local/share/fonts`).

## §A — Automated

```sh
mise run gate
sha256sum assets/fonts/FiraMono-*.ttf
git ls-files | grep -i 'firamono'
```

Expected:

- The gate is green, including `mono.rs` unit tests, the layout guard in
  `tests/layout_apparatus.rs`, the one-copy test, the packaging licence test and
  `scripts/tests/macos-bundle.test.sh`.
- The hashes equal the two in `assets/fonts/PROVENANCE.md`.
- `git ls-files` lists `assets/fonts/FiraMono-Regular.ttf`, `assets/fonts/FiraMono-Bold.ttf` and
  `assets/fonts/LICENSE-FiraMono-OFL.txt`, nothing under `tests/fixtures/`.
- `layout_snapshot.txt` is unchanged, or its diff touches monospace text nodes only (SC-005).

## §B — Recorded pass on Linux (visual-pass)

Screenshot each step that names a look, in light and dark.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Open a session whose transcript has a fenced code block, an inline-code span in prose and a tool result. | Code block, inline code and tool output in Fira Mono (same glyph shapes as the face's specimen); the surrounding prose in Roboto. | US1.1, US1.3, FR-002, FR-009 |
| B2 | Open the Changes view on a worktree with a diff; then an inline diff in the transcript. | Added, removed and context lines in Fira Mono. | US1.2 |
| B3 | Show inline code with strong emphasis (`**\`bold code\`**`). | The span is Fira Mono Bold, visibly heavier, same character width. | FR-011 |
| B4 | Install a different-version `Fira Mono` (or any font renamed to family `Fira Mono`) into `~/.local/share/fonts`, `fc-cache -f`, relaunch, repeat B1. | Identical to B1; screenshots compare equal in the code areas. | US1.5, FR-003 |
| B5 | Show code containing `漢字` and an emoji. | Those characters render through the system fallback, not as boxes; the ASCII around them stays Fira Mono. | FR-004, Edge: coverage |
| B6 | Open two windows with different sessions, each showing code. | Both use Fira Mono. | Edge: multiple sessions |
| B7 | **Terminal (FR-007, step D4):** in a session terminal run `printf 'normal \e[1mbold\e[0m\n'; ls -l`. | Cells in Fira Mono, `bold` in Fira Mono Bold; columns aligned; no gaps or overlap between cells. | FR-007, Edge: terminal |
| B8 | Open Settings and the sidebar. | Interface text unchanged (Roboto, same sizes). | FR-009, SC-005 |

## §C — macOS and Windows

Repeat B1, B3, B7 on macOS and Windows. Expected: the same face and the same line widths as Linux
(Principle VI, SC-001). Inspect the built bundles:

```sh
# macOS
ls "Micold AI IDE.app/Contents/Resources/licenses/fonts/"
# Linux
dpkg -c target/debian/*.deb | grep doc/micold-ai-ide/fonts/
rpm -qlp target/generate-rpm/*.rpm | grep doc/micold-ai-ide/fonts/
# Windows (after install)
dir "%ProgramFiles%\Micold AI IDE\licenses\fonts"
```

Expected: `LICENSE-FiraMono-OFL.txt`, `LICENSE-Roboto-OFL.txt`, `LICENSE`, `PROVENANCE.md` in each
(US3.2, SC-004).
