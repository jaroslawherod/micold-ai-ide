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

## Cycle 35: U33 a web or mail link resolves to `Url(address)` with `display == address` and no confirmation

- test: `crates/micold-core/src/link/resolve.rs::tests::a_web_or_mail_link_opens_its_address_as_shown` (new), with `local()` and `detected(address)` fixtures; a `resolve` stub returning `None` was declared so it compiled
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::resolve::tests::a_web_or_mail_link_opens_its_address_as_shown -- --exact`
  -> `assertion failed: https://a.example/x?y=1 opens as written, the hint shows it as written, and nothing asks first` / `left: None` / `right: Some(ResolvedLink { ..., display: "https://a.example/x?y=1", target: Url("https://a.example/x?y=1"), needs_confirmation: false })` (1 failed)
- green: `resolve` classifies the address; `Web` and `Mail` give `Url(address)` shown as the address with no confirmation, anything else `None`. The context is unused until file links (`_ctx`). Suite -> 1093 passed, 0 failed; clippy clean
- refactor: none needed
- commit: `feat(031): resolve a web or mail link to its address (U33)`

## Cycle 36: U34 a declared non-followable URI resolves to `None`

- test: `crates/micold-core/src/link/resolve.rs::tests::a_declared_uri_that_is_not_followable_resolves_to_nothing` (new). Also T006's interim guard `a_file_link_resolves_to_nothing_before_file_links_land`, which carries no behavior id and is replaced by T042
- red: both passed on arrival (`test ... ok`), since U33's `resolve` returns `None` for `NotFollowable`. Deliberate mutant: `NotFollowable` resolves to `Url(address)` like a web link. `scripts/build-lock.sh cargo test -p micold-core --lib link::resolve::tests::a_declared_uri_that_is_not_followable_resolves_to_nothing -- --exact`
  -> `assertion failed: a program may declare vscode://file/home/u/a.rs, but it is never offered as a link` / `left: Some(ResolvedLink { ..., target: Url("vscode://file/home/u/a.rs"), ... })` / `right: None` (1 failed); the file guard failed under the same mutant (`assertion failed: file links open nothing until their resolution exists`). Mutant reverted from the backup
- green: no implementation change. Suite -> 1095 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that a non-followable declared URI resolves to nothing (U34)`

## Cycle 37: U46 for every resolved link, `display` equals the string its target carries

- test: `crates/micold-core/src/link/resolve.rs::tests::what_the_hint_shows_is_exactly_what_opens` (new), sampled by hand over web, upper-case web, mail with a query, an application scheme and a `file:` address, and asserting that three of them resolve
- red: passed on arrival (`test ... ok`), since U33's `resolve` shows the address it opens. Deliberate mutant: `display` is the address lower-cased. `scripts/build-lock.sh cargo test -p micold-core --lib link::resolve::tests::what_the_hint_shows_is_exactly_what_opens -- --exact`
  -> `assertion `left == right` failed: HTTP://LOCALHOST:8080/: the hint shows exactly the string handed to the opener (SC-006)` / `left: "http://localhost:8080/"` / `right: "HTTP://LOCALHOST:8080/"` (1 failed). Mutant reverted from the backup
- green: no implementation change. Suite (`cargo test -p micold-core --all-targets`) -> 1096 passed, 0 failed
- refactor: none needed
- commit: `test(031): pin that the hint shows exactly what opens (U46)`
- notes: T006 and T012 ticked in this commit (U28–U30, U33, U34, U46 DONE). T042 extends the sample to `HostPath`

## Cycle 38: U49 no source in `link/` names `std::net`, `std::fs` or `std::process`

- test: `crates/micold-core/src/link/mod.rs::tests::link_performs_no_io` (new). It reads every `link/*.rs` through `include_str!`, builds its needles at run time, and first asserts that its list names exactly the `pub mod`s `link/mod.rs` declares
- red: passed on arrival (`test ... ok`), as the list expects of a guard. Deliberate mutants, each run with `scripts/build-lock.sh cargo test -p micold-core --lib link::tests::link_performs_no_io -- --exact` and reverted from the backup:
  - the list's mutant, `use std::fs;` in `line.rs` -> `link/line.rs names std::fs: recognising and resolving a link does no I/O (FR-019)` (1 failed). The first attempt inserted it above the `//!` header and did not compile (`error[E0753]: expected outer doc comment`), so no red was recorded from it
  - `pub mod runnable;` declared in `mod.rs` with an empty `runnable.rs` -> `assertion `left == right` failed: the scan reads exactly the modules link/mod.rs declares` / `left: ["address", "detect", "line", "resolve"]` / `right: ["address", "detect", "line", "resolve", "runnable"]` (1 failed)
- green: no implementation change. Suite (`cargo test -p micold-core --all-targets`) -> 1097 passed, 0 failed; clippy clean
- refactor: none needed
- commit: `test(031): pin that link recognition does no I/O (U49)`
- notes: T007 ticked in this commit. T043 adds `runnable.rs` to `SOURCES`, as the second mutant shows it must

## Cycle 39: U132 every expected link in the ≥50-line corpus is found with its exact span, and no scheme-less text is found

- test: `crates/micold-core/tests/link_corpus.rs::every_address_in_the_corpus_is_found_with_its_exact_span_and_nothing_else` (new), over the new `crates/micold-core/tests/fixtures/link_corpus.txt` (78 output lines: help text from gh, mise, docker, jq, wget, grep, gpg and cargo-deb as printed on this machine; git push, gh, dev-server, build-tool and AI CLI session output; contract §3's rows; five soft-wrapped entries; three hard-broken entries, which this test skips and U133 reads). Expected links are delimited inline with `⟦…⟧`; every cell of every entry is asked for its link, so scheme-less text is checked as "no link" cell by cell. Each expected link is also resolved: its hint shows exactly what opens, and a web or mail link opens exactly as marked
- red: passed on arrival (`test ... ok`): T009 and T010's rules already cover the corpus. Deliberate mutants, each run with `scripts/build-lock.sh cargo test -p micold-core --test link_corpus every_address_in_the_corpus_is_found_with_its_exact_span_and_nothing_else -- --exact` and reverted from the backup:
  - `detect` never trims trailing punctuation -> `corpus line 24, char 36 ('h'): the link under it is exactly the one marked, or none` / `left: Some(Link { address: "https://jqlang.org/.", ... cols: 36..56 })` / `right: Some(Link { address: "https://jqlang.org/", ... cols: 36..55 })` (1 failed)
  - `link_at` never joins a soft-wrapped row below -> `corpus line 101, char 14 ('h'): ...` / `left: None` / `right: Some(Link { address: "http://localhost:5173/some/very/long/route/that/wraps", ..., cells: [CellSpan { row: 1, cols: 14..40 }, CellSpan { row: 2, cols: 0..27 }] })` (1 failed). Line 101 was the `@wrap` directive's number, not the output line's, which the refactor fixed
- green: no implementation change. Suite (`cargo test -p micold-core --all-targets`) -> 1098 passed, 0 failed; clippy clean
- refactor: test only. Each output line carries its own corpus line number for failure messages instead of its entry's first line plus an offset (wrong for `@wrap` and `@hard` entries); the resolve check asserts SC-006 over `Url` and `HostPath` targets alike, so file links resolving in T047 do not break it for the wrong reason. Suite green after each move
- commit: `test(031): check links over a 78-line corpus of real output (U132)`

## Cycle 40: U133 a corpus address hard-broken across real line breaks is found on its first row only

- test: `crates/micold-core/tests/link_corpus.rs::a_hard_broken_address_is_found_on_its_first_row_only` (new), over the corpus's `@hard` entries: every cell of the first row gives exactly its marked piece or nothing, the piece's hint shows exactly the piece, and every cell of a continuation row gives nothing
- red: the first run did not compile (`error[E0716]: temporary value dropped while borrowed`, filtering `corpus().iter()` into a `Vec<&Entry>`), so no red was recorded from it. It then passed on arrival (`test ... ok`): only `wrapped` joins rows. Deliberate mutant: `link_at` joins the row below whenever it is available, soft wrap or not. `scripts/build-lock.sh cargo test -p micold-core --test link_corpus a_hard_broken_address_is_found_on_its_first_row_only -- --exact`
  -> `corpus line 126, char 5: the first row's piece is a link on that row alone` / `left: Some(Link { address: "https://example.com/docs/page.html", ..., cells: [CellSpan { row: 1, cols: 5..13 }, CellSpan { row: 2, cols: 0..26 }] })` / `right: None` (1 failed). A first attempt at this run died on a full disk (`printf: write error: No space left on device` from `build-lock.sh`) before the test ran; `target-shared/debug/incremental` (30G) was deleted with no cargo or rustc running, and the run repeated
- the mutant was caught only by the entry whose first piece is malformed: the other two continue on indented rows, and a leading space ends the joined address anyway. So the corpus gained a fourth `@hard` entry (line 126, `https://ci.example.com/builds/2026/09/14/run-` / `48213/logs?step=test`) with an unindented continuation, as `fold` prints it; the corpus now holds 80 output lines. Re-run with the corpus extended: unmutated `ok`; joining below -> `corpus line 126, char 0: the first row's piece is a link on that row alone` / `left: Some(Link { address: "https://ci.example.com/builds/2026/09/14/run-48213/logs?step=test", ... })` / `right: Some(Link { address: "https://ci.example.com/builds/2026/09/14/run-", ... })` (1 failed); joining above whenever available -> `corpus line 118, char 22: ...` / `left: None` / `right: Some(Link { address: "https://grafana.example.com/d/abc123/service-", ... })` (1 failed). Both mutants reverted from the backup
- green: no implementation change. Suite (`cargo test -p micold-core --all-targets`) -> 1099 passed, 0 failed; clippy clean
- refactor: none needed
- commit: `test(031): pin that a hard-broken address is a link on its first row only (U133)`
- notes: T008, T009 and T010 ticked in this commit (U1–U27, U132, U133, U147–U150 DONE). Every M1 behavior is DONE

## Cycle 41: U151 above a cut, a candidate preceded only by address characters is dropped

- origin: review A (code-review, high) on M1 found that the top-cut check dropped a candidate only at index 0, so `https://app.example/…` nested in `?next=https://app.example/…` was offered once the outer address's start lay past the cap or above the first available row. The existing cap test passed only because its nested scheme fell on a row start. Added to the list as U151; contract L7's upper edge now reads "nothing but characters an address may contain lies between column 0 of the upper row and its start"; T005 and T010 carry `[U151]` and were unticked until it was DONE
- test: `crates/micold-core/src/link/line.rs::tests::a_candidate_that_may_sit_inside_an_address_cut_above_is_dropped` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_candidate_that_may_sit_inside_an_address_cut_above_is_dropped -- --exact`
  -> `assertion `left == right` failed: the row above is unavailable and only address characters come before the scheme, so it may be the tail of `?next=https://…`` / `left: Some(Link { address: "https://b.example/y", origin: Detected, cells: [CellSpan { row: 1, cols: 1..20 }] })` / `right: None` (1 failed)
- green: `detect::ends_an_address` became `pub(super)`, and `detected_at` treats a candidate as reaching the top cut when no char before it on the logical line ends an address. The first run after the change failed on the test's own keep case: `left: ... cols: 2..21` / `right: ... cols: 2..20`. The expected span was miscounted (the address is 19 chars), so the test was corrected to `2..21`, with no implementation change; the drop half's red above was recorded before the implementation. Suite (`cargo test -p micold-core --all-targets`) -> 1100 passed, 0 failed; clippy clean
- refactor: none needed
- commit: `feat(031): drop an address that may sit inside one cut above (U151)`
- notes: T005 and T010 re-ticked

## Review fixes: M1 review B strengthens the U49 and U132 tests (no behavior added)

- origin: review B (conformance) on M1, MINOR F3 and F4
- U49, `crates/micold-core/src/link/mod.rs::tests::link_performs_no_io`: the module list now also reads private and `pub(…)` `mod x;` declarations, and every `std::{…}` import group (nested groups included) is searched for `net`, `fs` and `process`. It stayed green on the tree. Mutants, each run with `scripts/build-lock.sh cargo test -p micold-core --lib link::tests::link_performs_no_io -- --exact` and reverted from the backup:
  - `use std::{fs, io};` in `line.rs`, which the old needles missed -> `link/line.rs imports ["fs", "io"] from std: recognising and resolving a link does no I/O (FR-019)` (1 failed)
  - a private `mod runnable;` in `mod.rs` with an empty `runnable.rs` -> `assertion `left == right` failed: the scan reads exactly the modules link/mod.rs declares` / `left: ["address", "detect", "line", "resolve"]` / `right: [..., "runnable"]` (1 failed)
- U132, `crates/micold-core/tests/link_corpus.rs::every_address_in_the_corpus_is_found_with_its_exact_span_and_nothing_else`: the size check counts lines carrying a marked link, as SC-002 says ("50 real lines … containing addresses"), not every output line. With the bound raised to 500 as a probe the run printed `the corpus holds at least 50 real lines containing addresses, not 58`; the bound was restored to 50
- Suite (`cargo test -p micold-core --all-targets`) -> 1100 passed, 0 failed; clippy clean; `cargo fmt --all -- --check` clean
- commit: `test(031): tighten the no-I/O guard and the corpus size check`

## Cycle 42: U152 a capped line packed with schemes that are not addresses is scanned in time

- origin: review A round 2 (code-review, high) on M1: a candidate failing `well_formed` stepped one char on, and each candidate had already run `scan_end` to the end of its run, so `mailto:` repeated with no `@` costs quadratic time on the 129-row cap. A probe before the change: `mailto:` over 38,696 chars took 3.49 s (`http://x:y` took 5 ms only because its nested schemes follow a letter and are never candidates). Replacing the per-candidate `String` in the `mailto:` check alone still took 2.34 s, so that edit was discarded
- test: `crates/micold-core/src/link/detect.rs::tests::a_line_packed_with_schemes_that_are_not_addresses_is_scanned_in_time` (new): `mailto:` and `http://x/` repeated over 129 × 300 chars find nothing within 500 ms
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::detect::tests::a_line_packed_with_schemes_that_are_not_addresses_is_scanned_in_time -- --exact`
  -> `mailto: repeated over 38700 chars took 3.40662737s: every candidate rescans the rest of the line, so a hover stalls` (1 failed)
- green: `detect` builds a `Scan` once per text: each start's stop (a character no address contains, or a closer no opener at or after the start balances, settled in one pass with a stack of pending starts), the next `'`, `@` and `/`, and the run of trailing punctuation before each index. `end` and `well_formed` read them in O(1); `names_a_host` still reads only the authority. A `file:/` case drafted into the test was dropped: `file:/` is not a scheme, and a `file://` chain cannot fail, since each nested scheme supplies the outer one's `/`. U152 -> `ok` (0.02 s). Differential probe (a temporary test, deleted after): the new `detect` against the old one from `HEAD` on 200,000 random texts of up to 24 tokens from schemes, `x.y`, `@ / ( ) [ ] ' " . , : ? #`, space, `<`, `é` -> equal on all. Suite (`cargo test -p micold-core --all-targets`) -> 1129 passed, 0 failed (main's tests included after the rebase); clippy clean; `cargo fmt --all -- --check` clean
- refactor: none beyond the green change
- commit: `perf(031): scan a line packed with non-address schemes in linear time (U152)`

## Cycle 43: U153 padding before a wrapped wide char declares what the char before it declares

- origin: review A round 3 (code-review, high) on M1: alacritty 0.26 writes the `LEADING_WIDE_CHAR_SPACER` padding with the cursor template's `extra` (`term/mod.rs` `write_at_cursor`), so the padding carries the URI of the wide char that wrapped to the next row. `link_at` read the hovered cell's `hyperlink` and `declared_at` fell back to a run of one char, the plain one the padding belongs to (L5)
- test: `crates/micold-core/src/link/line.rs::tests::padding_before_a_wrapped_wide_char_takes_the_link_of_the_char_before_it` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::padding_before_a_wrapped_wide_char_takes_the_link_of_the_char_before_it -- --exact`
  -> `assertion `left == right` failed: the padding is part of a plain char, so it is no link` / `left: Some(Link { address: "https://a.example", origin: Declared, cells: [CellSpan { row: 0, cols: 3..5 }] })` / `right: None` (1 failed)
- green: `link_at` reads `hyperlink` at the lead cell (`cells[index].start`) of the char holding the hovered cell, so a spacer declares what its char declares. Contract L5 now says so. U153 -> `ok`. Suite (`cargo test -p micold-core --all-targets`) -> 1130 passed, 0 failed; clippy clean; fmt clean
- refactor: none needed
- commit: `fix(031): a spacer declares what its char declares (U153)`
- notes: `mise run gate` steps at the previous commit `a2450d0e` (fmt, clippy core and workspace, `cargo test --workspace --no-fail-fast`, `scripts/tests/*.test.sh`) -> all green; the `micold-daemon` `exclusivity` failure of the first gate run did not recur

## Cycle 44: U154 a quoted address closed at a bottom cut is kept

- origin: review A round 4 (code-review, high) on M1, low: `detected_at` dropped an address when only trailing punctuation lay between it and a bottom cut, and `'` is trailing punctuation, so `'https://a.example/x'` with its closing quote in the last column of a row wrapping into an unavailable row was never offered. The same review's other low (a column-0 address with no row above, including the alternate screen's top row) is the edge already routed to T028; its bullet now names the alternate screen
- test: `crates/micold-core/src/link/line.rs::tests::a_quoted_address_whose_closing_quote_ends_a_wrapped_row_is_a_link` (new)
- red: a first draft put the quoted row at row 0 with nothing above, so it measured U151's top cut instead; it was rewritten with an empty row above before any implementation. That uncommitted draft was then lost when review B restored its own backup of `line.rs` after a probe; the test was rewritten and committed as WIP before running. `scripts/build-lock.sh cargo test -p micold-core --lib link::line::tests::a_quoted_address_whose_closing_quote_ends_a_wrapped_row_is_a_link -- --exact`
  -> `assertion `left == right` failed: the closing quote ends the address, so nothing on the next row could extend it` / `left: None` / `right: Some(Link { address: "https://a.example/x", origin: Detected, cells: [CellSpan { row: 1, cols: 1..20 }] })` (1 failed)
- green: `closed_by_its_quote` (the char before the address and the char at its end are both `'`) exempts the address from the bottom-cut drop. Contract L7 says so. U154 -> `ok`. Suite (`cargo test -p micold-core --all-targets`) -> 1131 passed, 0 failed; clippy clean; fmt clean
- refactor: none needed
- commit: `fix(031): keep a quoted address closed at a bottom cut (U154)`

## M2: opening pipeline (T013–T022)

Per ledger decision 14, a cycle's suite is the tests of the files it touches; the workspace suite and `mise run gate` run at the milestone's end. The M2 tests were written together with compiling stubs (`OpenLink`, `OpenFailure`, the two `SessionMsg` variants with empty reducers, a `link_opener.rs` whose functions answer `LaunchFailed("unimplemented")`, a `links.rs` returning `Task::none()`) and committed before any implementation, so each red below is an assertion, not a compile error.

## Cycle 45: U73–U76 the session feature asks to open a URL and reports a failed open

- test: `crates/micold-client/tests/features_session_links.rs` (new): `activating_a_url_link_asks_to_open_that_url_verbatim`, `an_open_with_no_application_notifies_that_nothing_is_set_up`, `a_failed_launch_notifies_the_reason`, `an_open_that_worked_notifies_nothing`
- red: `scripts/build-lock.sh cargo test -p micold-client --test features_session_links`
  -> U73 `left: []` / `right: [OpenLink(Url("https://example.com/docs?q=a%20b#top"))]`; U74 `left: []` / `right: [(Error, "Couldn't open https://example.com/docs?q=a%20b#top: no application is set up to open it")]`; U75 `left: []` / `right: [(Error, "Couldn't open …: xdg-open exited with status 4")]` (1 passed; 3 failed)
- U76 passed on arrival: an empty reducer notifies nothing. Mutant (review B F2), run after green and reverted: `Ok(()) => "mutant".to_string()` in `link_open_finished` -> `an_open_that_worked_notifies_nothing` panicked with `the browser opening is the feedback; a notification would be noise` (1 failed)
- green: `link_activated` maps `Target::Url(u)` to `OpenLink(OpenRequest::Url(u))`; `link_open_finished` calls `notify_error` with the contract §5 texts. -> 4 passed
- commit: red `test(031): opening pipeline tests against compiling stubs (T013–T016)`; green `feat(031): opening pipeline from LinkActivated to the system opener (U73–U76, U93, U94, U101–U105, U138, U139, U144)`

## Cycle 46: U93–U94 `LinkActivated` through `update_inner` reaches the opener verbatim and at once

- test: `crates/micold-client/src/shell/links.rs::tests::activating_a_url_through_update_inner_opens_it_verbatim_and_without_delay` (new); it runs the returned task through `iced_runtime::task::into_stream` on a tokio runtime (new dev-dependency `iced_runtime = "=0.14.0"`, already in the lock through `iced`)
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide shell::link`
  -> `left: []` / `right: ["https://example.com/a(b)?q=%20x#frag"]` (the stub route returned no task)
- green: `shell::links::on_link_message` runs the session reducer, sends `OpenLink` to `perform` (`Task::perform` over `spawn_blocking` calling `opener.open`, answered by `LinkOpenFinished`), `ClipboardWrite` to `shell::clipboard::interpret`, and drains the rest; `update_inner` routes `LinkActivated` to it; `Capabilities` gains `link_opener` and `with_link_opener`, and every test `App` in `main_tests.rs` (not only `base_app()`) holds `NoopLinkOpener`. -> ok (under 250 ms, and one `LinkOpenFinished { address, result: Ok(()) }`)
- commit: red `test(031): opening pipeline tests against compiling stubs (T013–T016)`; green `feat(031): opening pipeline from LinkActivated to the system opener (U73–U76, U93, U94, U101–U105, U138, U139, U144)`

## Cycle 47: U101–U104, U138, U139, U144 the system opener's launch window and classifiers

- test: `crates/micold-client/src/shell/link_opener.rs::tests` (new): `launch::{a_launcher_exiting_zero_inside_the_window_is_success, xdg_open_exiting_three_inside_the_window_is_no_application, another_non_zero_exit_inside_the_window_is_a_launch_failure, a_launcher_still_running_at_the_end_of_the_window_is_success_and_keeps_running}` (`#[cfg(unix)]`, `sh -c` stub children), `macos_open_failing_names_no_application_only_when_stderr_says_so`, `shell_execute_returns_map_to_success_no_application_and_launch_failure`, `linux_reveal_opens_the_folder_when_the_file_manager_cannot_select_the_item`
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide shell::link`
  -> U101 `left: Err(LaunchFailed("unimplemented"))` / `right: Ok(())`; U102 `… / right: Err(NoApplication)`; U103 panicked at `link_opener.rs:128` (no status named); U104 `left: Err(LaunchFailed("unimplemented"))` / `right: Ok(())`; U138 `left: 0` / `right: 2` (calls); U139 and U144 `left: Err(LaunchFailed("unimplemented"))` / `right: Ok(())` (8 failed with U93)
- green: `launch_window` polls `try_wait` every 10 ms up to the window, classifies an exit (stderr read when piped), and hands a still-running child to a reaping thread; `classify_xdg_open` (0, 3, other), `classify_macos_open` (stderr naming no application or -10814), `classify_shell_execute` (> 32, 31/27, other); `reveal_linux` calls `dbus-send --print-reply … ShowItems array:string:<file URI>` and falls back to `xdg-open <parent>`. `SystemLinkOpener` has Linux (`xdg-open`, null stdio), macOS (`open`, stderr piped; `open -R`) and Windows (`ShellExecuteW`; `explorer.exe` with `raw_arg`) arms; `micold-client` gains `windows-sys` for Windows, and the workspace entry `Win32_UI_Shell` and `Win32_UI_WindowsAndMessaging`. -> 8 passed. `cargo clippy -p micold-client --all-targets -- -D warnings` clean on the host, `--target aarch64-apple-darwin` and `--target x86_64-pc-windows-msvc`
- commit: red `test(031): opening pipeline tests against compiling stubs (T013–T016)`; green `feat(031): opening pipeline from LinkActivated to the system opener (U73–U76, U93, U94, U101–U105, U138, U139, U144)`

## Cycle 48: U105 the system link opener is chosen only in `Capabilities::real()`

- test: `crates/micold-client/tests/no_concrete_implementations.rs::the_system_link_opener_is_named_only_at_its_definition_and_in_capabilities_real` (new)
- red: none on arrival, since the stub definition and its `Capabilities::real()` site landed with the tests. Two mutants, each run against the built test binary and reverted:
  - `let _probe = shell::link_opener::SystemLinkOpener;` at the top of `update_inner` -> `` `SystemLinkOpener` is chosen outside `Capabilities::real()`: ["main.rs×1"] `` (1 failed)
  - the definition renamed to `SystemOpener` -> `the definition exists, so this guard is not vacuous` (1 failed)
- green: -> 14 passed (whole file)
- commit: `test(031): opening pipeline tests against compiling stubs (T013–T016)`
- notes: `tests/clipboard_request.rs`, `tests/outcome_termination.rs` and `tests/features_are_render_free.rs` still pass with the new `Outcome` and `SessionMsg` variants (4, 2 and 6 passed)

## Review fixes: M2 reviews A and B

- `tests/feature_registration_cost.rs::only_the_root_drives_a_feature` failed in `mise run gate`: `shell/links.rs calls session::update`. The root now owns the call: `State::update_session_for_effects(msg)` runs the session reducer, drains every outcome but `ClipboardWrite` and `OpenLink` through `app::interpret`, and returns those two; `shell::links::on_link_message` performs them. U93/U94 stay green
- review A (low): `ShellExecuteW` ran without COM on the blocking thread. The Windows arm now calls `CoInitializeEx(COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)` first, ignoring `S_FALSE` and `RPC_E_CHANGED_MODE`; workspace `windows-sys` gains `Win32_System_Com`. Not testable on the Linux gate; `cargo clippy --target x86_64-pc-windows-msvc` checks it compiles
- review B F2: U76's mutant run recorded in cycle 45
- `mise run gate` (second run): `micold-core/tests/background_spawns_hide_console.rs::every_background_spawn_hides_its_console` flagged the Windows `reveal`'s `explorer.exe` spawn; it is now wrapped in `micold_core::process::no_window`

## Cycle 49: U155 a reveal names a non-UTF-8 file by its own bytes

- origin: review A (low) on M2: `file_uri` percent-encoded `path.to_string_lossy()`, so a Linux name with a byte that is not UTF-8 asked `ShowItems` for U+FFFD instead
- test: `crates/micold-client/src/shell/link_opener.rs::tests::a_file_name_that_is_not_utf8_is_encoded_from_its_own_bytes` (new, `#[cfg(unix)]`)
- red: with the lossy encoding restored for the run, `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide shell::link`
  -> `left: "file:///home/u/a%20b%2C%EF%BF%BD"` / `right: "file:///home/u/a%20b%2C%FF"` (1 failed)
- green: on Unix `file_uri` encodes `OsStrExt::as_bytes`; elsewhere the lossy text. -> 9 passed
- commit: `fix(031): M2 review and gate fixes (U155)`
