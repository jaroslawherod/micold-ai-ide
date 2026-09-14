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
