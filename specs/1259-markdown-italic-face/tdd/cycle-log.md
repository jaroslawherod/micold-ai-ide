# TDD cycle log: 1259 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers. The first M1 unit ran its red steps without recording their output; where
that is so, the row says so, and red was shown afterwards by mutation in the second M1 unit
(review B F1). Nothing below is back-filled.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001, T002 | the six `Roboto-*.ttf` parse, report 400/500/600 upright and italic, family `Roboto`, static, cover the interface's text, distinct files | not recorded (the guard fails to compile until T002 adds the `include_bytes!` files; the output was not kept) | `cargo test -p micold-client --test roboto_font` ok |
| T005, T006 | a bare database resolves each shipped weight to its own face, upright and italic | not recorded (fails to compile until T006); mutation in unit 2: `install_roboto` registering only `ROBOTO_FACES[..3]` → `the_renderer_draws_each_weight_and_bold_in_the_bundled_faces ... FAILED` ("Italic ROBOTO at Normal is not drawn in the bundled Roboto-Italic") | restored: ok |
| T007, T008 | the in-crate renderer draws italic `ROBOTO` in the bundled italics | not recorded; same mutation as the row above covers it | ok |
| T012, T013 | an italic span is slanted at the base weight; bold italic is bold and slanted; code inside emphasis is monospace and upright | not recorded; mutation in unit 2: `painted` slanting code (`&& !s.style.code` removed) → `code_inside_emphasis_stays_upright ... FAILED` ("code inside emphasis is slanted", left `Italic`, right `Normal`). The italic and bold-italic cases pass on `origin/main`'s `painted` (characterization: it already set the slant and the weight) | restored: ok |
| T014, T015 | the shared fixture has a `***bold italic***` span, bold and italic | not recorded; mutation in unit 2: fixture reverted to `origin/main` → `emphasis_strikethrough_and_inline_code_are_styled_spans ... FAILED` ("a span reads \"bold italic\"") | restored: `cargo test -p micold-core --test markdown_subset` 18 passed |
| T016 | `layout_snapshot` unchanged | — | unit 1: fixture untouched; unit 2: see the gate |

## M1 fix round (visual pass T018, review A/B round 1)

| Behaviour | Red evidence | Green |
|---|---|---|
| bold (700) and bold italic draw in Roboto SemiBold / SemiBold Italic through cosmic-text's own matching, not a fontdb `Query` (`resolves_to_bundled` now shapes a sample; `the_renderer_draws_each_weight_and_bold_in_the_bundled_faces`) | with `alias_bold` a no-op: `... FAILED` ("Normal ROBOTO at Bold is not drawn in the bundled Roboto-SemiBold"). Upright registration is unchanged from `origin/main`, so upright bold already left Roboto there | after `alias_bold`: ok |
| `alias_bold` adds one 700 face upright and one italic over the 600 bytes, idempotent (`bold_aliases_the_bundled_semibold_faces`) | stub: `... FAILED` (left 0, right 2) | ok |
| characters Roboto's italic lacks leave emphasis upright, per grapheme, keeping the link (`emphasis_the_italic_lacks_draws_upright`) | stub returning the spans: `... FAILED` (left `[("漢字 and 🙂", true)]`, right `[("漢字", false), (" and ", true), ("🙂", false)]`) | after `upright_where_italic_lacks`: ok |
| the three italics cover the same characters; `italic_has_glyph` answers `a`, `é` yes and `漢`, `🙂` no | stub returning `true`: `... FAILED` (`!italic_has_glyph('漢') && ...`) | ok |
| `ui::ROBOTO_FACES` is the six shipped files, one per weight and style, in `ROBOTO_FAMILY` (review A F2) | written after the code it guards (characterization) | `cargo test -p micold-client --test roboto_font` ok |
