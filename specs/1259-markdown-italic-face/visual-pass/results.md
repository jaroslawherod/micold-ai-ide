# Feature 1259 visual pass (T018, quickstart §B)

Date 2026-10-24. Debug build of `e4c4ba50` (HEAD `17182a40` differs only in `autopilot.md`): `micold-showcase`,
`micold-ai-ide` and `micold-daemon` from one build, pinned copies. Private Xvfb 1600x1400, `ICED_BACKEND=tiny-skia`
(CPU rasteriser; no lavapipe on this host), private `XDG_DATA_HOME`/`XDG_RUNTIME_DIR`. Light and dark scheme
(showcase toggle; client Settings > Appearance > Theme).

B3, B4, B6, B7 ran in the real client: a structured Claude Code session whose `claude` is a stand-in script on a
private `PATH` that answers `initialize` and replies to each user message with fixed markdown (the B3/B4/B6 text
from the quickstart). Daemon log: `client attached to daemon`. B5 not in scope.

| Check | Verdict | Evidence |
|---|---|---|
| B1 MarkdownView paragraph | **FAIL**: `italic` is Roboto Italic, but `bold italic` draws in a monospace bold oblique (DejaVu Sans Mono Bold Oblique), not Roboto SemiBold Italic. Same in light and dark | b1-markdownview-light/dark.png, b1-bold-italic-zoom-light.png, font-specimen-vs-rendered.png |
| B2 LinkText | PASS: `italic` slanted Roboto Italic; links (underlined, link colour), bold, strike and code look as before | b2-linktext-light/dark.png |
| B3 reply in a session | **FAIL** (one of four): `not` and `underscore` slanted alike; `cfg` upright Fira Mono on its fill, `see`/`here` slanted; link slanted, underlined, link colour. But `***both***` draws in DejaVu Sans Mono Bold Oblique | b3-reply-light/dark.png, font-specimen-vs-rendered.png |
| B4 heading and table header | PASS: `slanted` is italic at the heading's weight and size; header `x` slanted and heavy like `plain` | b4-heading-table-light/dark.png |
| B6 `*漢字 and 🙂*` | **FAIL**: `and` is slanted and the emoji draws, but 漢字 draws as two missing-glyph boxes. Control reply in the same session: upright and bold 漢字 draw (host has WenQuanYi/IPAGothic); only the italic run shows boxes | b6-cjk-emoji-light/dark.png, b6-control-upright-italic-bold-light/dark.png |
| B7 two sessions, Overview | PASS: B3's reply in the second worktree's session looks the same as in the first (stacked crops). Overview cards: upright Roboto, no emphasis. No pre-change baseline was built, so "unchanged" is by inspection only | b7-session1-vs-session2-light/dark.png, b7-overview-light/dark.png |

## Finding: weight-700 runs leave Roboto

`font-specimen-vs-rendered.png` puts ImageMagick specimens above the client's own pixels. Upright `**bold**`
matches **DejaVu Sans Bold**, not Roboto SemiBold. `***both***` matches **DejaVu Sans Mono Bold Oblique**, not
Roboto SemiBold Italic. The renderer does not resolve a `Weight::Bold` (700) request to the 600 Roboto faces
the way `roboto.rs` describes. It takes a system face with an exact 700 match instead. The unit tests use a
fontdb `Query`, so they pass. For italic, the nearest exact match is a monospace face. That is the B1/B3 failure,
and it likely causes B6 too: the italic CJK fallback resolves to a face with no CJK glyphs. Upright bold falling
back to DejaVu may predate this feature, because no baseline was built to check.

## Not covered

- B5 (host-installed Roboto Italic): out of scope for this run.
- Real-display rendering (GPU/wgpu) and macOS/Windows. tiny-skia uses the same cosmic-text font matching, so this
  run should still reproduce the fallback.

## Re-run after the fix (D10, D11)

Second M1 unit, same host. Debug showcase (`CARGO_PROFILE_DEV_DEBUG=0`) built from the fix tree with
one temporary, uncommitted change: `markdown-subset.md` line 11 gained `*漢字 and 🙂*` after
`***bold italic***`, so B6's text draws through the same `MarkdownView` → `LinkText` path as B1 and
B3. Private Xvfb, `ICED_BACKEND=tiny-skia`, light scheme.

| Check | Verdict | Evidence |
|---|---|---|
| B1 / B3 bold italic | PASS: `bold` is Roboto SemiBold (proportional, Roboto letterforms) and `bold italic` Roboto SemiBold Italic, not a monospace oblique | rerun-b1-b6-light.png |
| B6 `*漢字 and 🙂*` | PASS: 漢字 draws upright in a host CJK face, no boxes; `and` slanted; the emoji draws | rerun-b1-b6-light.png |

Dark scheme not re-run: the scheme changes colours, not font matching, and the worker's FAIL was the
same in both schemes. The renderer-path unit tests (`roboto::tests::the_renderer_draws_each_weight_and_bold_in_the_bundled_faces`,
`link_text::tests::emphasis_the_italic_lacks_draws_upright`) cover both fixes in the gate.

## B5: host-installed Roboto (T022)

Date 2026-10-25. Debug `micold-showcase` (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`) built from
`2983ec92` (HEAD `4281f0f4` differs only in `autopilot.md`), pinned copy. Control: the same binary built
from `dbe13ae2` (before the pin), exported with `git archive`. Private Xvfb 1600x1400, `ICED_BACKEND=tiny-skia`,
private `XDG_RUNTIME_DIR`/`XDG_DATA_HOME`. Light and dark (showcase toggle).

Host fonts went into the private `$XDG_DATA_HOME/fonts` (fontconfig's `<dir prefix="xdg">fonts</dir>`), one
set per launch, and were deleted with the run directories afterwards. Nothing went into `~/.local/share/fonts`.

- **real**: `Roboto-Italic.ttf`, `Roboto-Bold.ttf` (700) and `Roboto-BoldItalic.ttf` from the googlefonts/roboto
  v2.138 release. These files differ from the bundled ones (sha256 `2b4076dd…` vs `2f700a53…` for the italic).
- **decoy**: FreeSerif Italic, Bold and Bold Italic renamed to family `Roboto` with the same styles and weights
  (fontTools, `fc-scan`: Roboto Italic 400, Roboto Bold 700, Roboto Bold Italic 700). If the app picks a host
  face, the text turns serif, which the eye catches at once.

Each run captured the B1 paragraph (MarkdownView) and the B2 line (LinkText). The crops are pixel-compared
(`compare -metric AE`) with the no-host-font baseline at identical geometry.

| Check | Verdict | Evidence |
|---|---|---|
| B5 real host Roboto, light | PASS: B1 paragraph AE 0, LinkText AE 0 against the baseline | b5-host-roboto-light.png (row 2) |
| B5 real host Roboto, dark | PASS: B1 paragraph AE 0, LinkText AE 0 | b5-host-roboto-dark.png (row 2) |
| B5 decoy host Roboto, light | PASS: B1 paragraph AE 0, LinkText AE 0. `italic`, `bold`, `bold italic` stay bundled Roboto | b5-host-roboto-light.png (row 3) |
| B5 decoy host Roboto, dark | PASS: B1 paragraph AE 0, LinkText AE 0 | b5-host-roboto-dark.png (row 3) |
| Control: pre-fix build with the decoy | Decoy picked up as expected: `bold`, `italic` and `bold italic` draw in FreeSerif (B1 AE 4032, LinkText AE 1775, both schemes). This shows the app reads the fonts directory and the pin is what keeps the host faces out | b5-host-roboto-light/dark.png (row 4) |

Not covered: host upright Roboto at 400/500/600 (FR-004 keeps it on purpose). B3 in the client, which
uses the same `install_roboto` font system. Real GPU display, macOS and Windows.
