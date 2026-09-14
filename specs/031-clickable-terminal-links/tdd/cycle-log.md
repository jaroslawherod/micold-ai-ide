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
