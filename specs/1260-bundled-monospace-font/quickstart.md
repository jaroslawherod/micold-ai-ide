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

## Results

Recorded by the close unit on 2026-10-24 against `main` at `12d38521` (M3 merged).

### §A (T028)

- **Hashes**: `sha256sum assets/fonts/FiraMono-*.ttf` printed
  `8c86f2963208a353c3435e28ecb38a99ab68f14d7433ba00c1822cef9a9c1b44` (Regular) and
  `2a28efd740e8da1da75d40ac79f0db4f60a0f1aead0b15ca16b4694a11b45fc6` (Bold), the two values in
  `assets/fonts/PROVENANCE.md`'s Fira Mono table. PASS.
- **One copy**: `git ls-files | grep -i firamono` lists `assets/fonts/FiraMono-Bold.ttf`,
  `assets/fonts/FiraMono-Regular.ttf` and `assets/fonts/LICENSE-FiraMono-OFL.txt`, nothing under
  `tests/fixtures/`. PASS (also guarded by `there_is_one_copy_of_fira_mono_in_the_repository`).
- **Layout snapshot (SC-005)**: no commit of this feature touches
  `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`git log 5d634ede..12d38521` on that
  path lists only other features' re-records). PASS.
- **Gate**: green for each milestone (M1 at `7e49d4ed`, M2 on its final tree, M3 at `a26592e9`; ledger *Review
  rounds*) and again for the close PR's tree.

### §B

Recorded in M1's visual pass: [visual-pass/README.md](./visual-pass/README.md). B1, B2, B4, B7,
B8 PASS in light and dark; B3 and B5 have no showcase pose (covered by the `mono.rs` weight-700
and keep-other-families tests); B6 not applicable to the showcase.

### §C (T029)

What CI's macOS and Windows runners prove (`.github/workflows/ci.yml`, job *build + test*):

| Check | Linux | macOS | Windows |
|---|---|---|---|
| `mono.rs` unit tests (bytes, weights, host-copy removal), terminal cell fonts | yes (full workspace) | no | no |
| `layout_apparatus` Regular+Bold guard, `layout_snapshot` | yes | no | no |
| `packaging_ships_font_licences` (deb, rpm, `.app` script, `.iss` each list the four files) | yes | no | no |
| `packaging_excludes_showcase` (`.iss` allows the licence set only under `{app}\licenses\`) | yes | yes | yes |
| `scripts/tests/macos-bundle.test.sh` (stages the `.app`, checks `Contents/Resources/licenses/fonts/`) | yes (shell suites) | no | no |
| Bundle built by the release script and launched | — | `.app` via `macos-bundle.sh` | x64 and arm64 installer, installed and launched |

The face is the same bytes on every host (`include_bytes!`, loaded into the font system by
`install_monospace` before iced starts, host copies removed), so the Linux tests that resolve and
measure it hold for macOS and Windows by construction; no macOS or Windows runner measures line
widths. CI does not list the installed bundles' `licenses/fonts/` contents on macOS or Windows:
it builds and launches them from the same scripts the Linux tests read.

Needs a host, not run: the look on macOS and Windows (B1, B3, B7 there), and the `ls`/`dpkg -c`/
`rpm -qlp`/`dir` listings of built bundles above. The first release build is the point to check
them.
