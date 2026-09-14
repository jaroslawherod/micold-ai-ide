# Cycle Log: Clickable Links in the Terminal

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3073 passed, 2 failed, 6 ignored (310 test binaries)
- commit: `411711c1` (spec merged; no code for this feature yet)
- recorded: cycle 0, before any change, 2026-09-14
- the two failures are in `micold-daemon`, which this feature does not touch, and neither involves links:
  - `exclusivity::one_conversation_one_session::a_second_open_of_a_held_pi_conversation_starts_nothing`
    (`exclusivity.rs:435`, "exactly one `pi` for one conversation", left 0, right 1; red on 3 reruns)
  - `pi_launch_wiring::a_pi_session_carries_the_component_only_while_the_switch_is_on`
    (`pi_launch_wiring.rs:130`, "launch 0 never reached `pi`")
- both pass in CI on `main` (run `CI` on `b27ffe62`, success), so they are local to this machine. The loop
  treats them as pre-existing: a cycle is green when nothing else fails, and they are not fixed here.

## Cycle 1: U1 finds an address inside a sentence

- test: `crates/micold-core/src/link/detect.rs::tests::finds_an_address_inside_a_sentence` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::finds_an_address_inside_a_sentence -- --exact`
  -> `left: []` / `right: ["https://example.com/docs/page.html"]` (0 passed; 1 failed), against a `detect` stub returning no ranges
- green: `detect` scans for `http://` and `https://` and extends to the next whitespace. Suite
  `scripts/build-lock.sh cargo test -p micold-core --all-targets` -> 1059 passed, 0 failed
- refactor: none needed
- commit: `feat(031): find a web address inside a sentence (U1)`
- notes: per ledger decision 14, a cycle's suite is the core fast subset; the workspace suite runs at the milestone's end

## Cycle 2: U2 trims trailing punctuation repeatedly

- test: `crates/micold-core/src/link/detect.rs::tests::trims_trailing_punctuation_repeatedly` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::trims_trailing_punctuation_repeatedly -- --exact`
  -> `left: ["https://example.com/docs/page.html?!."]` / `right: ["https://example.com/docs/page.html"]` (1 failed)
- green: `detect` drops `.,;:!?'*` from the end while any remains. Suite -> 1060 passed, 0 failed
- refactor: folded the scan end into the trimmed end; suite re-run -> 1060 passed
- commit: `feat(031): trim sentence punctuation from an address's end (U2)`

## Cycle 3: U3 keeps a closing bracket balanced inside the address

- test: `crates/micold-core/src/link/detect.rs::tests::keeps_a_closing_bracket_balanced_inside_the_address` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::keeps_a_closing_bracket_balanced_inside_the_address -- --exact`
  -> `1 passed`, because the scanner has no bracket rule yet and so never trims `)`.
  Deliberate mutant: `)` added to `TRAILING_PUNCTUATION` -> `left: ["https://example.com/a_(b"]` / `right: ["https://example.com/a_(b)"]` (1 failed). Mutant removed, file restored.
- green: no implementation change. Suite -> 1061 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin a balanced closing bracket inside an address (U3)`
- notes: green on arrival; it guards the next cycle (U4), whose bracket rule must not trim a balanced closer

## Cycle 4: U4 stops before an unbalanced closing bracket

- test: `crates/micold-core/src/link/detect.rs::tests::stops_before_an_unbalanced_closing_bracket` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::stops_before_an_unbalanced_closing_bracket -- --exact`
  -> `left: ["https://example.com/a)"]` / `right: ["https://example.com/a"]` (1 failed)
- green: `scan_end` tracks `(` and `[` openers and ends the address at a closer with no matching
  opener. Suite -> 1062 passed, 0 failed (U3 still green)
- refactor: none needed; the scan already moved into its own function in the green step
- commit: `feat(031): end an address before an unbalanced closing bracket (U4)`

## Cycle 5: U5 excludes an enclosing quote

- test: `crates/micold-core/src/link/detect.rs::tests::excludes_an_enclosing_quote` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::excludes_an_enclosing_quote -- --exact`
  -> `left: ["https://example.com\""]` / `right: ["https://example.com"]` (1 failed)
- green: a `"` or `'` right before the scheme is passed to `scan_end`, which ends the address at
  that quote. Suite -> 1063 passed, 0 failed
- refactor: none needed
- commit: `feat(031): end a quoted address before its closing quote (U5)`

## Cycle 6: U6 finds only the address of a Markdown link

- test: `crates/micold-core/src/link/detect.rs::tests::finds_only_the_address_of_a_markdown_link` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::finds_only_the_address_of_a_markdown_link -- --exact`
  -> `1 passed`: the unbalanced-closer stop from U4 already ends the address before `)`.
  Deliberate mutant: `scan_end` no longer treats `)` as a closer -> `left: ["https://x.example)"]` / `right: ["https://x.example"]` (1 failed). Mutant removed, file restored.
- green: no implementation change. Suite -> 1064 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin the address of a Markdown link (U6)`
- notes: green on arrival, as research R3 rule 4 predicts ("That handles `[text](https://x.y)`")

## Cycle 7: U7 finds only the address inside angle brackets

- test: `crates/micold-core/src/link/detect.rs::tests::finds_only_the_address_inside_angle_brackets` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::finds_only_the_address_inside_angle_brackets -- --exact`
  -> `left: ["https://x.example>"]` / `right: ["https://x.example"]` (1 failed)
- green: `scan_end` stops at `>`. Suite -> 1065 passed, 0 failed
- refactor: none needed; the rest of research R3 rule 2's stop set is U9's cycle
- commit: `feat(031): end an address at a closing angle bracket (U7)`

## Cycle 8: U8 rejects a scheme preceded by a letter or digit

- test: `crates/micold-core/src/link/detect.rs::tests::rejects_a_scheme_preceded_by_a_letter_or_digit` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::rejects_a_scheme_preceded_by_a_letter_or_digit -- --exact`
  -> `left: ["https://a.example", "http://a.example"]` / `right: []` (1 failed)
- green: a scheme whose previous character is an ASCII letter or digit is skipped. Suite -> 1066 passed, 0 failed
- refactor: moved the scheme lookup and the glued-word check into `scheme_at`; suite re-run -> 1066 passed
- commit: `feat(031): ignore a scheme glued to the end of a word (U8)`

## Cycle 9: U9 stops the scan at whitespace, control characters and `< > " ` { } | \ ^`

- test: `crates/micold-core/src/link/detect.rs::tests::stops_at_whitespace_control_characters_and_characters_no_address_contains` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::stops_at_whitespace_control_characters_and_characters_no_address_contains -- --exact`
  -> `'\u{7}' cannot appear in an address` / `left: ["https://a.example/x\u{7}y"]` / `right: ["https://a.example/x"]` (1 failed)
- green: `ends_an_address` stops at whitespace, any control character and the nine listed characters. Suite -> 1067 passed, 0 failed
- refactor: re-wrapped the `scan_end` doc comment to name rule 2; `cargo fmt`; suite re-run -> 1067 passed
- commit: `feat(031): end an address at characters no address contains (U9)`

## Cycle 10: U10 accepts web hosts `localhost`, dotted, bracketed IPv6 or with a port

- test: `crates/micold-core/src/link/detect.rs::tests::accepts_a_web_host_only_when_it_is_localhost_dotted_bracketed_or_has_a_port` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::accepts_a_web_host_only_when_it_is_localhost_dotted_bracketed_or_has_a_port -- --exact`
  -> `a dotless name with no port is too likely to be prose to be an address` / `left: ["http://intranet/x"]` / `right: []` (1 failed)
- green: `well_formed` reads the authority up to `/`, `?` or `#`, drops any user info, and accepts
  a bracketed literal, `localhost`, a dotted name, or a name with a numeric port. A rejected
  candidate advances the scan by one character. Suite -> 1068 passed, 0 failed
- refactor: `cargo fmt` only
- commit: `feat(031): recognise a web address only when it names a host (U10)`

## Cycle 11: U11 rejects `https://` alone and `http://exa mple`

- test: `crates/micold-core/src/link/detect.rs::tests::rejects_a_scheme_alone_and_a_host_broken_by_a_space` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::rejects_a_scheme_alone_and_a_host_broken_by_a_space -- --exact`
  -> `1 passed`: U10's host rule already rejects an empty host and the dotless, portless `exa`.
  Deliberate mutant: `well_formed` returns `true` first -> `left: ["https://", "http://exa"]` / `right: []` (1 failed). Mutant removed, file restored.
- green: no implementation change. Suite -> 1069 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that a scheme alone or a broken host is not an address (U11)`
- notes: green on arrival; U10 made the rule this behavior pins

## Cycle 12: U12 finds `mailto:team@example.com`; rejects `mailto:@example.com` and `mailto:team@`

- test: `crates/micold-core/src/link/detect.rs::tests::finds_a_mail_address_only_with_text_on_both_sides_of_the_at_sign` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::finds_a_mail_address_only_with_text_on_both_sides_of_the_at_sign -- --exact`
  -> `left: []` / `right: ["mailto:team@example.com"]` (1 failed)
- green: `mailto:` joins the recognised schemes; `well_formed` takes the scheme and, for `mailto:`, requires text on both sides of the first `@`. Suite -> 1070 passed, 0 failed
- refactor: the web host rule moved unchanged into `names_a_host`, so `well_formed` reads as one rule per scheme; suite re-run green
- commit: `feat(031): recognise a mail address with a mailbox and a domain (U12)`

## Cycle 13: U13 finds `file:///tmp/x`; rejects a `file:` address whose path does not start with `/`

- test: `crates/micold-core/src/link/detect.rs::tests::finds_a_file_address_only_when_its_path_starts_with_a_slash` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::finds_a_file_address_only_when_its_path_starts_with_a_slash -- --exact`
  -> `left: []` / `right: ["file:///tmp/x", "file://localhost/tmp/y"]` (1 failed)
- green: `file://` joins the recognised schemes; `well_formed` accepts it when a `/` follows the optional host (research R3: "`file` needs `file://` followed by an optional host and a path starting with `/`"). Suite -> 1071 passed, 0 failed
- refactor: `well_formed` became one `match` arm per scheme; suite re-run green
- commit: `feat(031): recognise a file address with an absolute path (U13)`

## Cycle 14: U14 never finds scheme-less text (`example.com/docs`, `www.example.com`)

- test: `crates/micold-core/src/link/detect.rs::tests::never_finds_an_address_without_a_scheme` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::never_finds_an_address_without_a_scheme -- --exact`
  -> `1 passed`: only the four scheme prefixes start an address.
  Deliberate mutant: `"www."` added to `SCHEMES` -> `left: ["www.example.com"]` / `right: []` (1 failed). Mutant removed, file restored.
- green: no implementation change. Suite -> 1072 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that text without a scheme is never an address (U14)`
- notes: green on arrival; the scheme list from U1/U12/U13 already excludes it (FR-001, SC-002)

## Cycle 15: U15 never finds `javascript:…`, `data:…` or `vbscript:…`

- test: `crates/micold-core/src/link/detect.rs::tests::never_finds_a_script_or_data_address` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::never_finds_a_script_or_data_address -- --exact`
  -> `1 passed`: none of the three is a recognised scheme.
  Deliberate mutant: `"javascript:"` added to `SCHEMES` -> `left: ["javascript:alert(document.cookie)"]` / `right: []` (1 failed). Mutant removed, file restored.
- green: no implementation change. Suite -> 1073 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that script and data addresses are never found (U15)`
- notes: green on arrival; FR-011's exclusion follows from the closed scheme list. The fixture's `javascript:` address contains a dot on purpose, so the mutant cannot be masked by the host rule

## Cycle 16: U16 matches the scheme ASCII case-insensitively (`HTTPS://EXAMPLE.COM`)

- test: `crates/micold-core/src/link/detect.rs::tests::matches_the_scheme_whatever_its_case` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::matches_the_scheme_whatever_its_case -- --exact`
  -> `left: []` / `right: ["HTTPS://EXAMPLE.COM", "Mailto:Team@Example.com", "FILE:///TMP/X"]` (1 failed)
- green: `starts_with` compares each character with `eq_ignore_ascii_case`; `scheme_at` still returns the lower-case constant, so `well_formed` needs no change. Suite -> 1074 passed, 0 failed
- refactor: `starts_with` zips the prefix's characters directly instead of collecting them into a `Vec`; suite re-run green (1074 passed, 0 failed)
- commit: `feat(031): match an address's scheme in any case (U16)`

## Cycle 17: U17 any cell of a maximal same-URI declared run returns that whole run as one link

- test: `crates/micold-core/src/link/line.rs::tests::any_cell_of_a_declared_run_returns_the_whole_run` (new), over a fake `LinkRows` (`Rows`/`Row` in the test module)
- red: `link_at` stubbed to `None` so the test compiles. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::any_cell_of_a_declared_run_returns_the_whole_run -- --exact`
  -> `left: None` / `right: Some(Link { address: "https://a.example", origin: Declared, cells: [CellSpan { row: 0, cols: 5..13 }] })` (1 failed)
- green: `link_at` takes the pointer cell's declared URI and extends left and right along the row while the neighbouring cells carry the same URI. Suite -> 1075 passed, 0 failed
- refactor: none needed
- commit: `feat(031): return the whole declared run under a cell (U17)`
- notes: the run is one row for now; joining soft-wrapped rows into the logical line arrives with U21. T004 ticked in this commit: all of U1–U16 are DONE

## Cycle 18: U18 two same-URI runs separated by an undeclared cell are two links

- test: `crates/micold-core/src/link/line.rs::tests::two_same_uri_runs_apart_are_two_links` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::two_same_uri_runs_apart_are_two_links -- --exact`
  -> `1 passed`: U17's extension already stops at the first cell without the URI.
  Deliberate mutant: the run spans from the row's first to its last cell with the URI -> `left: ... cols: 0..13` / `right: ... cols: 0..4` (1 failed). Mutant removed, file restored.
- green: no implementation change. Suite -> 1076 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that separated same-URI runs are two links (U18)`
- notes: green on arrival; contract L6

## Cycle 19: U19 adjacent runs with different URIs are two links with their own addresses

- test: `crates/micold-core/src/link/line.rs::tests::adjacent_runs_with_different_uris_are_two_links` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::adjacent_runs_with_different_uris_are_two_links -- --exact`
  -> `1 passed`: U17's run compares the exact URI.
  Deliberate mutant: a neighbouring cell counts as the same run when it carries any URI -> `left: ... cols: 0..6` / `right: ... cols: 0..3` (1 failed). Mutant removed, file restored.
- green: no implementation change. Suite -> 1077 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that adjacent runs with different URIs are two links (U19)`
- notes: green on arrival; US2 scenario 3

## Cycle 20: U147 a detected address on one row returns its cells on that row

- test: `crates/micold-core/src/link/line.rs::tests::a_detected_address_on_one_row_returns_its_cells` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_detected_address_on_one_row_returns_its_cells -- --exact`
  -> `left: None` / `right: Some(Link { address: "https://a.example/x", origin: Detected, cells: [CellSpan { row: 0, cols: 4..23 }] })` (1 failed)
- green: with no declared URI at the cell, `link_at` runs `detect` on the row's text and returns the range holding the column, one char per cell. Suite -> 1078 passed, 0 failed
- refactor: none; U21 reshapes `link_at` around the logical line
- commit: `feat(031): return a detected address under a cell on one row (U147)`
- notes: split before its cycle. U21 (detection over soft-wrapped rows) needed both detection in `link_at` and the logical line, two new behaviors at once. This one-row step was appended to the list as U147 and run first, so U20's precedence rule can also be proven against real detection. Order now U147, U21, U20, U22…U27

## Cycle 21: U21 a detected address over rows joined by `wrapped = true` returns cells on every row it covers

- test: `crates/micold-core/src/link/line.rs::tests::a_detected_address_over_soft_wrapped_rows_covers_every_row` (new); the fake `Row` gained `wrapped()`
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_detected_address_over_soft_wrapped_rows_covers_every_row -- --exact`
  -> `left: Some(Link { address: "https://a.exa", origin: Detected, cells: [CellSpan { row: 0, cols: 4..17 }] })` / `right: Some(Link { address: "https://a.example/x", ..., cells: [CellSpan { row: 0, cols: 4..17 }, CellSpan { row: 1, cols: 0..6 }] })` (1 failed)
- green: a `LogicalLine` joins the rows around the pointer row (back while the row above has `wrapped`, forward while the current row has it), keeping each char's cell. `detected_at` runs `detect` on the joined text and maps the range back to one span per row. Suite -> 1079 passed, 0 failed
- refactor: the test module's explicit `Range`/`CellSpan`/`LinkOrigin` imports dropped, since `use super::*` already brings them; suite re-run green (1079), `cargo clippy -p micold-core --all-targets -D warnings` clean
- commit: `feat(031): detect an address across soft-wrapped rows (U21)`
- notes: no cap yet (U23) and no unavailable-row drop yet (U24). The declared run is still per row: contract L1 and research R5 say it continues across a soft wrap, which no listed behavior pinned, so U148 was appended for it

## Cycle 22: U20 a declared URI wins over address-shaped visible text in the same cells

- test: `crates/micold-core/src/link/line.rs::tests::a_declared_uri_wins_over_the_address_its_text_shows` (new)
- red: passed on arrival (`test ... ok`, 1 passed), since the declared branch already returns before detection. Deliberate mutant: `link_at` consults `detected_at` first and returns its link when found. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_declared_uri_wins_over_the_address_its_text_shows -- --exact`
  -> `left: Some(Link { address: "https://b.example/x", origin: Detected, cells: [CellSpan { row: 0, cols: 0..19 }] })` / `right: Some(Link { address: "https://a.example", origin: Declared, ... })` (1 failed). Mutant reverted from the backup
- green: no implementation change. Suite -> 1080 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that a declared URI wins over the address its text shows (U20)`
- notes: taken before U148 (declared run across a soft wrap), which stays next

## Cycle 23: U148 a declared run continues across a soft wrap onto the next row, and stops at a real line break

- test: `crates/micold-core/src/link/line.rs::tests::a_declared_run_continues_across_a_soft_wrap_and_stops_at_a_line_break` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_declared_run_continues_across_a_soft_wrap_and_stops_at_a_line_break -- --exact`
  -> `left: Some(Link { address: "https://a.example", origin: Declared, cells: [CellSpan { row: 0, cols: 9..11 }] })` / `right: ... cells: [CellSpan { row: 0, cols: 9..11 }, CellSpan { row: 1, cols: 0..2 }] })` (1 failed)
- green: `link_at` builds the `LogicalLine` once for both branches; the new `declared_at` extends the same-URI run over the line's cells instead of one row's columns and maps it back with `spans`. `LogicalLine::index_of` finds the pointer cell for both `declared_at` and `detected_at`. Suite -> 1081 passed, 0 failed
- refactor: none beyond sharing `index_of` in the green step; `cargo clippy -p micold-core --all-targets -D warnings` clean
- commit: `feat(031): continue a declared run across a soft wrap (U148)`

## Cycle 24: U22 rows separated by a real line break are never joined; only the first row's well-formed piece is a link

- test: `crates/micold-core/src/link/line.rs::tests::rows_apart_by_a_line_break_are_never_joined` (new)
- red: passed on arrival (`test ... ok`, 1 passed), since U21's `LogicalLine` already joins only across `wrapped`. Deliberate mutant: `LogicalLine::around` joins every available row, ignoring `wrapped` in both directions. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::rows_apart_by_a_line_break_are_never_joined -- --exact`
  -> `left: Some(Link { address: "https://a.example/docs", origin: Detected, cells: [CellSpan { row: 0, cols: 4..24 }, CellSpan { row: 1, cols: 0..2 }] })` / `right: Some(Link { address: "https://a.example/do", ..., cells: [CellSpan { row: 0, cols: 4..24 }] })` (1 failed). Mutant reverted from the backup
- green: no implementation change. Suite -> 1082 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that rows apart by a line break are never joined (U22)`

## Cycle 25: U23 a wrapped logical line within 64 rows each way is joined; a candidate reaching the 64-row cap is dropped

- test: `crates/micold-core/src/link/line.rs::tests::a_line_is_joined_64_rows_each_way_and_a_candidate_reaching_the_cap_is_dropped` (new), with a `soft_wrapped(text, width)` fixture; the fake rows were first made to own their text in the structural commit `test(031): let the link_at test rows own their text`
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_line_is_joined_64_rows_each_way_and_a_candidate_reaching_the_cap_is_dropped -- --exact`
  -> the first assertion (an address reaching exactly 64 rows each way) passed; `assertion failed: an address running past 64 rows below the pointer row is cut there, so it is dropped` / `left: Some(Link { address: "https://a.example/ppp…` / `right: None` (1 failed)
- green: `LogicalLine::around` stops 64 rows above and below the pointer row and records `cut_above`/`cut_below` when the line continues past the cap; `detected_at` drops a candidate starting at the line's first char under `cut_above` or ending at its last char under `cut_below`. Suite -> 1083 passed, 0 failed
- refactor: the continuation checks named as `continues_above`/`continues_below` closures; suite re-run green (1083); clippy clean. Mutant check on the third assertion (`cut_above: false`) -> `assertion failed: the cap 64 rows above cuts the address; the part below it is not offered as a link`, reverted
- commit: `feat(031): cap the logical line at 64 rows each way (U23)`
- notes: declared runs are not dropped at the cap, since their URI is whole whatever cells are visible. Appended U149: a candidate whose trimmed end is separated from the cut only by trailing punctuation is still truncated and must be dropped; the contract's "ends in the last column" wording misses it

## Cycle 26: U24 a candidate touching a row whose `text` is `None` is dropped

- test: `crates/micold-core/src/link/line.rs::tests::a_candidate_touching_an_unavailable_row_is_dropped` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_candidate_touching_an_unavailable_row_is_dropped -- --exact`
  -> `assertion failed: the row below is unavailable, so the address may go on there and is dropped` / `left: Some(Link { address: "https://a.exa", origin: Detected, cells: [CellSpan { row: 0, cols: 4..17 }] })` / `right: None` (1 failed)
- green: the forward walk stops before an unavailable row, and `cut_below` is set whenever the last joined row has `wrapped`; `cut_above` is also set when the row above the first joined row is unavailable. Suite -> 1084 passed, 0 failed
- refactor: `LogicalLine::around` finds `first` and `last` with two symmetric loops, then builds the text and cells once, so `cut_above` = the row above is unavailable or wraps, `cut_below` = the last row wraps; the field docs now name both cuts. Suite re-run green (1084), clippy clean. Mutant check on the column-0 assertion (`cut_above` ignoring an unavailable row) -> `assertion failed: the row above the first available row is unknown, so an address at column 0 may have started there`, reverted
- commit: `feat(031): drop a detected address cut by an unavailable row (U24)`

## Cycle 27: U149 a candidate at a cut is dropped even when only trailing punctuation separates it from the cut

- test: `crates/micold-core/src/link/line.rs::tests::a_candidate_only_punctuation_away_from_a_cut_is_dropped` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_candidate_only_punctuation_away_from_a_cut_is_dropped -- --exact`
  -> `assertion failed: the full stop may be the middle of an address going on below, so the address is cut` / `left: Some(Link { address: "https://a.example/x", origin: Detected, cells: [CellSpan { row: 0, cols: 4..23 }] })` / `right: None` (1 failed)
- green: under `cut_below`, a candidate reaches the cut when every char after its end is trailing punctuation (none at all included); `detect::TRAILING_PUNCTUATION` became `pub(super)` so both files share the one list. Contract L7 now words the drop that way and says it applies at the L4 cap. Suite -> 1085 passed, 0 failed
- refactor: none needed; clippy clean
- commit: `feat(031): drop an address only punctuation away from a cut (U149)`
- notes: only the lower edge needs this. The upper edge compares the candidate's exact start, and a closing bracket or quote before a cut ends an address whatever follows

## Cycle 28: U25 a wide character's spacer cell belongs to the link

- test: `crates/micold-core/src/link/line.rs::tests::a_wide_characters_spacer_cell_belongs_to_the_link` (new). Before it could compile, `LinkRows` gained `spacer(row, col) -> bool` (minimal declaration) and the fake `Row` a `spacer(col)` builder
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_wide_characters_spacer_cell_belongs_to_the_link -- --exact`
  -> `assertion failed: column 6: the address reads past the wide chars' spacers and covers them` / `left: None` / `right: Some(Link { address: "https://例え.jp", origin: Detected, cells: [CellSpan { row: 0, cols: 4..19 }] })` (1 failed)
- green: `LogicalLine.cells` holds each char's cell range; a spacer cell adds no char to the text and extends the range of the char before it on its row. `index_of` finds the char whose range holds the cell, `spans` joins ranges, and `declared_at` compares URIs at each char's lead cell. Suite -> 1086 passed, 0 failed
- refactor: the spacer branch reduced to one `if`/`else if`; suite re-run green (1086), clippy clean
- commit: `feat(031): read an address past wide-char spacer cells (U25)`
- notes: design gap found here and resolved as ledger decision 15. The wire's spacer char is a space (alacritty 0.26 writes `' '`), so `text` alone made detection stop inside `https://例え.jp`. Contract §1 and L5, data-model §1 and T028 now carry `spacer`, which the client answers from `WIDE_CHAR_SPACER | LEADING_WIDE_CHAR_SPACER`. Appended U150 for the leading spacer a wide char leaves at a row's end when it wraps

## Cycle 29: U150 the padding a wide char leaves at a row's end when it wraps is not text, and the link covers it

- test: `crates/micold-core/src/link/line.rs::tests::a_wide_char_wrapped_onto_the_next_row_leaves_padding_the_address_reads_across` (new)
- red: the first run failed for a broken fixture, not for the behavior: the second row `"例 え now"` put `now` right after the spacer, so `left` read `"https://a.example/例えnow"`. Not counted as red; the fixture was corrected to `"例 え  now"`. It then passed on arrival (`test ... ok`), since U25's mapping already covers a spacer with the char before it on its row. Deliberate mutant: a spacer in the last column of a wrapped row is kept as text. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_wide_char_wrapped_onto_the_next_row_leaves_padding_the_address_reads_across -- --exact`
  -> `assertion failed: the padding is not a space ending the address, and it is covered by the link` / `left: None` / `right: Some(Link { address: "https://a.example/例え", ..., cells: [CellSpan { row: 0, cols: 4..23 }, CellSpan { row: 1, cols: 0..4 }] })` (1 failed). Mutant reverted from the backup
- green: no implementation change. Suite -> 1087 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that an address reads across a wrapped wide char's padding (U150)`

## Cycle 30: U26 a plain-text cell returns `None`

- test: `crates/micold-core/src/link/line.rs::tests::a_plain_text_cell_is_no_link` (new)
- red: passed on arrival (`test ... ok`). Deliberate mutant: `detected_at` takes the line's first detected range whether or not it holds the cell. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_plain_text_cell_is_no_link -- --exact`
  -> `assertion failed: column 1 is plain text beside a detected address and a declared run` / `left: Some(Link { address: "https://a.example", origin: Detected, cells: [CellSpan { row: 0, cols: 4..21 }] })` / `right: None` (1 failed). Mutant reverted from the backup
- green: no implementation change. Suite -> 1088 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that a plain-text cell is no link (U26)`

## Cycle 31: U27 a negative (scrollback) row resolves exactly as a viewport row with the same content

- test: `crates/micold-core/src/link/line.rs::tests::a_scrollback_row_resolves_like_a_viewport_row` (new)
- red: the first run did not compile (`the method concat exists for array [Vec<Row>; 2], but its trait bounds were not satisfied`: the fake `Row` is not `Clone`), so no red was recorded from it; the rows are now built with `chain`. It then passed on arrival (`test ... ok`), since rows are `i64` throughout. Deliberate mutant: the walk back never goes above row 0 (`(row - MAX_ROWS_EACH_WAY).max(0)`). `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_scrollback_row_resolves_like_a_viewport_row -- --exact`
  -> `assertion failed: the same rows three lines up, in scrollback, give the same link on the rows above` / `left: None` / `right: Some(Link { address: "https://a.example/x", ..., cells: [CellSpan { row: -2, cols: 4..17 }, CellSpan { row: -1, cols: 0..6 }] })` (1 failed). Mutant reverted from the backup
- green: no implementation change. Suite -> 1089 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that a scrollback row resolves like a viewport row (U27)`

## Cycle 32: U28 `http`/`https` classify as `Web` and `mailto` as `Mail`, verbatim

- test: `crates/micold-core/src/link/address.rs::tests::web_and_mail_addresses_classify_verbatim` (new); `Address` (data-model §1, without `File` until T046) and a `classify` stub returning `NotFollowable` were declared so it compiled
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::address::tests::web_and_mail_addresses_classify_verbatim -- --exact`
  -> `assertion failed: https://a.example/x?y=1#z is a web address, passed on exactly as written` / `left: NotFollowable` / `right: Web("https://a.example/x?y=1#z")` (1 failed)
- green: `classify` matches the `http://`/`https://` and `mailto:` prefixes and returns the whole string. Suite -> 1090 passed, 0 failed; clippy clean
- refactor: none needed
- commit: `feat(031): classify web and mail addresses verbatim (U28)`
- notes: T005 ticked in this commit, since every behavior it names (U17–U27) is DONE

## Cycle 33: U29 scheme classification is ASCII case-insensitive

- test: `crates/micold-core/src/link/address.rs::tests::the_scheme_classifies_whatever_its_case` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::address::tests::the_scheme_classifies_whatever_its_case -- --exact`
  -> `assertion failed: an upper-case web scheme is still a web address, and keeps its case` / `left: NotFollowable` / `right: Web("HTTPS://A.EXAMPLE/X")` (1 failed)
- green: `classify` compares the leading `scheme.len()` bytes with `eq_ignore_ascii_case`, through a local `starts_with` closure. Suite -> 1091 passed, 0 failed; clippy clean
- refactor: none needed
- commit: `feat(031): classify a scheme whatever its case (U29)`

## Cycle 34: U30 `vscode:`, `slack:`, `zoommtg:`, `javascript:`, `data:`, `vbscript:` and an unknown scheme classify as `NotFollowable`

- test: `crates/micold-core/src/link/address.rs::tests::application_script_data_and_unknown_schemes_are_not_followable` (new)
- red: passed on arrival (`test ... ok`), since `classify` only names `http`, `https` and `mailto`. Deliberate mutant: any `://` address is `Web`. `scripts/build-lock.sh cargo test -p micold-core --lib link::address::tests::application_script_data_and_unknown_schemes_are_not_followable -- --exact`
  -> `assertion failed: vscode://file/home/u/a.rs opens an application, runs code, carries its own content or is unknown` / `left: Web("vscode://file/home/u/a.rs")` / `right: NotFollowable` (1 failed). Mutant reverted from the backup
- green: no implementation change. Suite -> 1092 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that other schemes are not followable (U30)`
- notes: T011 ticked in this commit (U28–U30 DONE)
