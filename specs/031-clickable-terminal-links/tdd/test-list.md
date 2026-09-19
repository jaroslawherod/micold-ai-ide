---
feature: 031-clickable-terminal-links
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 22
planned_at: 411711c1
updated_at: 411711c1
suite_baseline: red # locally only: 2 out-of-flow daemon `pi` tests; CI on main (b27ffe62) green — see cycle-log.md
---

# Test List: Clickable Links in the Terminal

Derived from `spec.md` (US1–US4 acceptance scenarios, FR-001–FR-023, SC-001–SC-007) and `plan.md`
with its contracts. It was not derived from the code.

**Trace ids.** spec.md numbers acceptance scenarios inside each story, so `US2.5` means User Story 2,
acceptance scenario 5. `FR-`, `SC-` and contract row ids (`C4`, `G3b`, `T7`, `L6`) refer to
`spec.md` and `contracts/`.

**Milestones.** Inner behaviors are grouped by component, not by milestone. `tasks.md` carries each
id on the tasks that write and implement it, and its `## Milestones` section says which PR ships
them.

## Outer loop: acceptance behaviors

**Entry point.** The desktop client has no end-to-end GUI runner; the profile's acceptance runner is
the sandbox real-runtime suite. So these are **integration tests of the composed client**, the
highest level the repository can test:

- a `GridCache` holding the printed lines;
- the real `TerminalPane` widget driven headless (tiny-skia, as the pane's existing `dispatch`
  harness does), with real mouse and modifier events;
- the published `Message`s run through `crate::update_inner` on `base_app()`;
- recording `LinkOpener` and clipboard capabilities.

They live in `crates/micold-client/src/shell/links.rs` `#[cfg(test)] mod acceptance`, which is in
the binary crate beside `update_inner`. What they cannot see — the real browser, file manager and
rendered pixels — is covered by the quickstart §B visual pass.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| A1  | Hovering any character of `https://example.com/docs/page.html` in `See https://example.com/docs/page.html for details.` marks exactly the address's cells (not "See", the space or the full stop) and the pane's pointer is `Pointer` | US1.1, FR-001, FR-005, FR-007 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::hovering_an_address_marks_exactly_its_cells_and_the_pointer_is_a_hand` |
| A2  | A Ctrl/Cmd press and release on that address calls the opener once with `https://example.com/docs/page.html` and publishes no `TerminalBytes` | US1.2, FR-004, FR-010, FR-014, SC-001 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::a_command_click_on_an_address_opens_it_once_and_writes_nothing_to_the_program` |
| A3  | An address the terminal soft-wrapped across two rows opens complete when activated on either row | US1.3, FR-003 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::a_soft_wrapped_address_opens_complete_from_either_row` |
| A4  | Activating `(https://example.com/a_(b))` opens `https://example.com/a_(b)`, and `"https://example.com"` opens `https://example.com` | US1.4, FR-005 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::surrounding_punctuation_is_left_out_of_what_opens` |
| A5  | A plain drag, double-click or triple-click starting on a link selects as today and calls no opener | US1.5, FR-004 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::selecting_on_a_link_selects_and_opens_nothing` (guard; mutant `command: _` in `link_gesture` fails it) |
| A6  | An address scrolled into scrollback, with the view scrolled up to it, opens the same address as on the live screen | US1.6 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::an_address_in_scrollback_opens_the_same_address` |
| A7  | Hovering `docs` declared as `https://example.com/manual` marks the run and the hint shows `https://example.com/manual` | US2.1, FR-002, FR-008 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::hovering_a_declared_run_marks_it_and_shows_its_declared_address` |
| A8  | Activating that run calls the opener with `https://example.com/manual` | US2.2, FR-002 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::activating_a_declared_run_opens_its_declared_address` |
| A9  | Two adjacent runs with different declared addresses each resolve to their own address when hovered | US2.3 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::adjacent_declared_runs_open_their_own_addresses` |
| A10 | With the same declared address on two runs separated by undeclared text, hovering one marks only that run | US2.4 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::same_address_runs_apart_are_marked_one_at_a_time` |
| A11 | Declared text that reads `https://a.example` but declares `https://b.example` shows and opens `https://b.example` | US2.5, FR-008 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::declared_text_that_reads_as_another_address_shows_and_opens_the_declared_one` |
| A12 | A `file://<this host>/<tmp>/readme%20a.txt` declared name, as `ls --hyperlink=always` prints it, calls `opener.open` with the decoded existing path | US2.6, FR-012 | example | PENDING | |
| A13 | Activating `mailto:team@example.com` calls the opener with `mailto:team@example.com` | US3.1, FR-010 | example | DONE | `crates/micold-client/src/shell/links.rs::acceptance::a_mail_address_reaches_the_opener_verbatim` |
| A14 | Activating a `file://` link to an existing document calls `opener.open` with its host path | US3.2, FR-010 | example | PENDING | |
| A15 | Activating a `file://` link to an existing folder calls `opener.open` with the folder path | US3.3, FR-010 | example | PENDING | |
| A16 | Activating a `file://` link to a runnable file calls `opener.reveal` and never `opener.open` | US3.4, FR-013 | example | PENDING | |
| A17 | Activating a `file://` link to a missing file calls no opener and raises `Couldn't open <address>: the file doesn't exist on this machine` | US3.5, FR-015 | example | PENDING | |
| A18 | `file://otherhost/x`, `javascript:alert(1)` and `vscode://x` are not marked on hover and activating them calls no opener | US3.6, FR-011, FR-012 | example | PENDING | |
| A19 | In a sandboxed session with a shared project, activating `file:///work/<project>/readme.txt` opens a confirmation naming the host path; **Open** calls `opener.open` with that host path | US3.7, FR-018, FR-018a | example | PENDING | |
| A20 | A right press over a link opens the terminal menu with **Open Link** and **Copy Link Address** ahead of today's items | US4.1, FR-020 | example | PENDING | |
| A21 | Choosing **Copy Link Address** on a declared link writes its declared address to the clipboard, and on a wrapped detected link the complete unwrapped address | US4.2, FR-021 | example | PENDING | |
| A22 | A right press over plain text opens the menu with no link items | US4.3, FR-020 | example | PENDING | |

Guards, green on arrival, each with the deliberate mutant that shows its red:

- A5: selection already works and nothing opens before T029. Mutant: `link_gesture` activates on a press without the Ctrl/Cmd modifier.
- A18: after T011 these addresses are already `NotFollowable`. Mutants: `classify` returns `Web` for `javascript:`; the `File` branch of `resolve` accepts any host.
- A22: the menu has no link items before T073. Mutant: `link_menu_items(None)` returns both items.

## Inner loop: unit behaviors

### `crates/micold-core/src/link/detect.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U1  | Finds `https://example.com/docs/page.html` inside a sentence, without the final full stop | FR-001, FR-005 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::finds_an_address_inside_a_sentence` |
| U2  | Trims trailing `.,;:!?'*` repeatedly (`…page.html?!.` ends at `html`) | FR-005 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::trims_trailing_punctuation_repeatedly` |
| U3  | Keeps a closing bracket balanced inside the address (`https://example.com/a_(b)`) | FR-005, US1.4 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::keeps_a_closing_bracket_balanced_inside_the_address` |
| U4  | Stops before an unbalanced closing bracket (`(https://example.com/a)` excludes `)`) | FR-005, US1.4 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::stops_before_an_unbalanced_closing_bracket` |
| U5  | Excludes an enclosing quote (`"https://example.com"`) | FR-005, US1.4 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::excludes_an_enclosing_quote` |
| U6  | In `[text](https://x.example)` only the address is found | FR-005 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::finds_only_the_address_of_a_markdown_link` |
| U7  | In `<https://x.example>` only the address is found | FR-005 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::finds_only_the_address_inside_angle_brackets` |
| U8  | Rejects a scheme preceded by a letter or digit (`xhttps://a.example`) | FR-001 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::rejects_a_scheme_preceded_by_a_letter_or_digit` |
| U9  | Stops the scan at whitespace, control characters and each of `< > " \` { } \| \ ^` | FR-001 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::stops_at_whitespace_control_characters_and_characters_no_address_contains` |
| U10 | Accepts web hosts `localhost`, `a.example`, `[::1]` and `intranet:8080`; rejects `http://intranet` (no dot, no port) | FR-001 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::accepts_a_web_host_only_when_it_is_localhost_dotted_bracketed_or_has_a_port` |
| U11 | Rejects `https://` alone and `http://exa mple` | FR-001 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::rejects_a_scheme_alone_and_a_host_broken_by_a_space` |
| U12 | Finds `mailto:team@example.com`; rejects `mailto:@example.com` and `mailto:team@` | FR-001 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::finds_a_mail_address_only_with_text_on_both_sides_of_the_at_sign` |
| U13 | Finds `file:///tmp/x`; rejects a `file:` address whose path does not start with `/` | FR-001 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::finds_a_file_address_only_when_its_path_starts_with_a_slash` |
| U14 | Never finds scheme-less text (`example.com/docs`, `www.example.com`) | FR-001, SC-002 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::never_finds_an_address_without_a_scheme` |
| U15 | Never finds `javascript:…`, `data:…` or `vbscript:…` | FR-011 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::never_finds_a_script_or_data_address` |
| U16 | Matches the scheme ASCII case-insensitively (`HTTPS://EXAMPLE.COM`) | FR-001 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::matches_the_scheme_whatever_its_case` |

### `crates/micold-core/src/link/line.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U17 | Any cell of a maximal same-URI declared run returns that whole run as one link | FR-002, L1 | example | DONE | `crates/micold-core/src/link/line.rs::tests::any_cell_of_a_declared_run_returns_the_whole_run` |
| U18 | Two same-URI runs separated by an undeclared cell are two links | US2.4, L6 | example | DONE | `crates/micold-core/src/link/line.rs::tests::two_same_uri_runs_apart_are_two_links` |
| U19 | Adjacent runs with different URIs are two links with their own addresses | US2.3 | example | DONE | `crates/micold-core/src/link/line.rs::tests::adjacent_runs_with_different_uris_are_two_links` |
| U20 | A declared URI wins over address-shaped visible text in the same cells | US2.5 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_declared_uri_wins_over_the_address_its_text_shows` |
| U21 | A detected address over rows joined by `wrapped = true` returns cells on every row it covers | FR-003, US1.3, L2 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_detected_address_over_soft_wrapped_rows_covers_every_row` |
| U22 | Rows separated by a real line break are never joined; only the first row's well-formed piece is a link | FR-003 | example | DONE | `crates/micold-core/src/link/line.rs::tests::rows_apart_by_a_line_break_are_never_joined` |
| U23 | A wrapped logical line within 64 rows each way is joined; a candidate reaching the 64-row cap is dropped | FR-003, L4 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_line_is_joined_64_rows_each_way_and_a_candidate_reaching_the_cap_is_dropped` |
| U24 | A candidate touching a row whose `text` is `None` is dropped | L7 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_candidate_touching_an_unavailable_row_is_dropped` |
| U25 | A wide character's spacer cell belongs to the link | FR-007, L5 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_wide_characters_spacer_cell_belongs_to_the_link` |
| U26 | A plain-text cell returns `None` | L3 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_plain_text_cell_is_no_link` |
| U27 | A negative (scrollback) row resolves exactly as a viewport row with the same content | US1.6 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_scrollback_row_resolves_like_a_viewport_row` |
| U147 | A detected address on one row returns its cells on that row (split from U21 before its cycle) | FR-001, L2 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_detected_address_on_one_row_returns_its_cells` |
| U148 | A declared run continues across a soft wrap onto the next row, and stops at a real line break | FR-002, FR-007, L1 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_declared_run_continues_across_a_soft_wrap_and_stops_at_a_line_break` |
| U149 | A candidate at a cut (the cap, or an unavailable row) is dropped even when only trailing punctuation separates it from the cut, since the address may continue past it | FR-003, L4, L7 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_candidate_only_punctuation_away_from_a_cut_is_dropped` |
| U150 | The padding cell a wide char leaves at a row's end when it wraps onto the next row is not text: an address reads across it and the padding is covered by the link | FR-003, FR-007, L5 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_wide_char_wrapped_onto_the_next_row_leaves_padding_the_address_reads_across` |
| U151 | Above a cut (the cap, or an unavailable row), a candidate preceded on the logical line only by characters an address may contain is dropped, since it may sit inside an address that starts past the cut (`?next=https://…`); a character that ends an address before it keeps it | FR-003, L4, L7 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_candidate_that_may_sit_inside_an_address_cut_above_is_dropped` |
| U152 | A logical line at the L4 cap packed with schemes that are not addresses (`mailto:` or `http://x/` repeated) is scanned in time: no candidate rescans the rest of the line | FR-001, L4 | example | DONE | `crates/micold-core/src/link/detect.rs::tests::a_line_packed_with_schemes_that_are_not_addresses_is_scanned_in_time` |
| U153 | The padding before a wide char that wrapped onto the next row declares what the char before it on its row declares, not the URI the terminal wrote into it: a plain char's padding is no link, and the declared run starts on the next row | FR-002, FR-007, L1, L5 | example | DONE | `crates/micold-core/src/link/line.rs::tests::padding_before_a_wrapped_wide_char_takes_the_link_of_the_char_before_it` |
| U154 | Below a cut, an address closed by the `'` that opened right before it is kept even when only that quote lies between it and the cut: the quote is a hard end | FR-003, L7 | example | DONE | `crates/micold-core/src/link/line.rs::tests::a_quoted_address_whose_closing_quote_ends_a_wrapped_row_is_a_link` |
| U155 | Linux reveal names the file by its own bytes: a name that is not UTF-8 is percent-encoded as it is on disk, not as a lossy copy | FR-013 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::a_file_name_that_is_not_utf8_is_encoded_from_its_own_bytes` |
| U156 | The pane's `LinkRows` answers `Some("")` for a row above the first line ever printed or above the alternate screen's top, and `None` for a row trimmed from scrollback or not cached | FR-003, L7, M1 review | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::rows_above_everything_printed_read_as_empty_and_rows_not_held_as_unavailable`, `rows_follow_the_scrollback_offset` |
| U157 | The pane's `LinkRows::spacer` is true exactly for cells whose style-run flags carry `WIDE_CHAR_SPACER` or `LEADING_WIDE_CHAR_SPACER` | FR-003, L5, Decision 15 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::spacer_cells_come_from_the_style_run_flags` |
| U158 | A link press released outside the pane opens nothing, even when the press was on the cell a missing pointer position would clamp to | FR-004, G1, M3 review | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_link_press_released_outside_the_pane_opens_nothing` |
| U159 | While the scrollbar shows, a link under its strip is not marked and the pointer is no hand there, since a press there pages the view | FR-007, M3 review | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_link_under_the_scrollbar_strip_is_not_marked` |
| U160 | Moving to another row of the same marked link repaints (the hint side follows the row); a move within the row does not | FR-008, M3 review | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::moving_to_another_row_of_the_marked_link_repaints` |
| U161 | A hover whose pointer sits past the grid's edge (no rows read) re-resolves whenever the grid moves, so a resize that brings text under the resting pointer marks it | FR-008, M3 review round 2 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_hover_off_the_grid_re_resolves_when_the_grid_moves` |
| U162 | An inherited identity variable whose value is not UTF-8 is dropped from a session too | FR-006, M4 review A | example | DONE | `crates/micold-daemon/tests/session_identity_env.rs::an_identity_variable_that_is_not_utf8_is_dropped_too` |
| U163 | A script that sets `FORCE_HYPERLINK=1`, which the resolver also inherited, still reports it, so the session gets it from the include values | FR-006, M4 review B | example | DONE | `crates/micold-core/tests/env_include_resolve.rs::unix::a_script_setting_an_inherited_identity_variable_still_reports_it` |

### `crates/micold-core/src/link/address.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U28 | `http`/`https` classify as `Web` and `mailto` as `Mail`, verbatim | FR-010, C1, C2 | example | DONE | `crates/micold-core/src/link/address.rs::tests::web_and_mail_addresses_classify_verbatim` |
| U29 | Scheme classification is ASCII case-insensitive | FR-011 | example | DONE | `crates/micold-core/src/link/address.rs::tests::the_scheme_classifies_whatever_its_case` |
| U30 | `vscode:`, `slack:`, `zoommtg:`, `javascript:`, `data:`, `vbscript:` and an unknown scheme classify as `NotFollowable` | FR-011, C3 | example | DONE | `crates/micold-core/src/link/address.rs::tests::application_script_data_and_unknown_schemes_are_not_followable` |
| U31 | `file://host/path` classifies as `File { host, path }` with `%20` decoded to a space | FR-012, C9 | example | PENDING | |
| U32 | An invalid percent escape, or a decoding that is not UTF-8, classifies as `NotFollowable` | FR-012, C10 | example | PENDING | |

U32 is a guard: `file:` is `NotFollowable` until T046. Its red is shown after T046 by a decoder that passes an invalid escape through literally.

### `crates/micold-core/src/link/resolve.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U33 | A web or mail link resolves to `Url(address)` with `display == address` and no confirmation | FR-008, C1, C2 | example | DONE | `crates/micold-core/src/link/resolve.rs::tests::a_web_or_mail_link_opens_its_address_as_shown` |
| U34 | A declared non-followable URI resolves to `None` | FR-011, C3 | example | DONE | `crates/micold-core/src/link/resolve.rs::tests::a_declared_uri_that_is_not_followable_resolves_to_nothing` |
| U35 | A `file` link with host empty, `localhost` or one of `host_names` (any ASCII case) resolves to `HostPath(path)` | FR-012, C4, C5 | example | PENDING | |
| U36 | A `file` link naming another host, including `file://server/share/…` and an unresolvable `file://build-host.invalid/…`, resolves to `None` with no lookup | FR-012, US3.6, C6, C18 | example | PENDING | |
| U37 | With `windows_host`, `/C:/Users/x` resolves to `C:\Users\x`, and a path with no drive resolves to `None` | FR-012, C7, C8 | example | PENDING | |
| U38 | In a sandboxed context, the container id's 12-character prefix and the full id are accepted as hosts | FR-018, C11 | example | PENDING | |
| U39 | A sandboxed path under a shared location resolves to that location's host path, with `needs_confirmation` | FR-018, FR-018a, C11 | example | PENDING | |
| U41 | A sandboxed path under no shared location resolves to `Unreachable`, displayed as `<path> — not reachable from this machine` | FR-018, C12 | example | PENDING | |
| U46 | For every resolved link, `display` equals the string its target carries (`Url` and `HostPath`), sampled over every case above | SC-006 | example | DONE | `crates/micold-core/src/link/resolve.rs::tests::what_the_hint_shows_is_exactly_what_opens` |
| U47 | In a sandboxed context, this machine's `host_names` still translate through the shared locations | FR-018, C17 | example | PENDING | |
| U140 | `host_names_from` lists the full name and its first DNS label (`build.example.com` → both), a dotless name once, an empty name as none | FR-012 | example | PENDING | |
| U141 | `container_host_names` gives a 64-character id's 12-character prefix and the full id, and a 12- or 8-character id once | FR-018, C11 | example | PENDING | |

U36 is a guard: `file:` resolves to `None` until T047. Its red is shown after T047 by a `File` branch that accepts any host.

U40, U42–U44 and U48 were placed on `sandbox/pathmap.rs` below; U46 is sampled by hand because the
profile has no property library.

### `crates/micold-core/src/link/mod.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U49 | No source in `link/` names `std::net`, `std::fs` or `std::process` | FR-019, US3.6 | example | DONE | `crates/micold-core/src/link/mod.rs::tests::link_performs_no_io` |

U49 is a guard: it is green on arrival. Its red is shown by adding `use std::fs;` to a `link/` file.

### `crates/micold-core/tests/link_corpus.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U132 | Every expected link in the ≥50-line corpus is found with its exact span, and no scheme-less text is found | SC-002 | approval | DONE | `crates/micold-core/tests/link_corpus.rs::every_address_in_the_corpus_is_found_with_its_exact_span_and_nothing_else` |
| U133 | A corpus address hard-broken across real line breaks is found on its first row only | SC-002, FR-003 | example | DONE | `crates/micold-core/tests/link_corpus.rs::a_hard_broken_address_is_found_on_its_first_row_only` |

`approval` here is a committed fixture with inline expected spans, not a regenerated snapshot.

### `crates/micold-core/src/link/runnable.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U50 | Linux and macOS: a file with any execute bit reveals; the same file with none opens | FR-013 | example | PENDING | |
| U51 | Linux: each listed launcher extension (`AppImage` in any case) reveals; `.txt` opens | FR-013 | example | PENDING | |
| U52 | macOS: a bundle directory (by extension or `is_bundle`) reveals; an ordinary folder opens | FR-013 | example | PENDING | |
| U53 | macOS: each listed extension reveals | FR-013 | example | PENDING | |
| U54 | Windows: `%PATHEXT%` entries and each listed extension reveal; the execute bit is ignored, so `.txt` with it opens | FR-013 | example | PENDING | |
| U55 | Only the last extension counts: `x.exe.txt` opens and `x.txt.exe` reveals on Windows | FR-013 | example | PENDING | |

### `crates/micold-core/src/sandbox/pathmap.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U40 | The most specific of nested shared locations maps the path | FR-018, C15 | example | PENDING | |
| U42 | A path whose `..` climbs above `/` maps to nothing | FR-018, C13 | example | PENDING | |
| U43 | Locations match whole path components: `/work/projector` is not under `/work/proj` | FR-018 | example | PENDING | |
| U44 | A result equal to or under a denied host path maps to nothing | FR-018a, C16b | example | PENDING | |
| U48 | On a Windows host the remainder joins the host location with `\` | FR-018, C14 | example | PENDING | |
| U145 | Normalisation precedes the location match: with `/work/proj` shared, `/work/proj/../../etc/passwd` maps to nothing and `/work/proj/a/../b` maps to `<host>/b` | FR-018, C13 | example | PENDING | |

### `crates/micold-core/src/sandbox/parse.rs`, `lifecycle.rs`, `mod.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U56 | Mount destinations are read from `Mounts[].Destination` in Docker and in Podman inspect output | FR-018 | example | PENDING | |
| U57 | Inspect output with no `Mounts` gives no destinations | FR-018 | example | PENDING | |
| U58 | `Started.mounted` is `None` when the container was created or replaced | FR-018 | example | PENDING | |
| U59 | `Started.mounted` is the destinations when the container was attached or started | FR-018 | example | PENDING | |
| U60 | `shared_locations` lists projects, state, home and credentials, most path components first | FR-018, C15 | example | PENDING | |
| U61 | The secret mount is never a shared location | FR-018a, C16 | example | PENDING | |
| U62 | With `mounted = Some`, a location the container does not mount is not shared; with `None`, all are | FR-018, C16c | example | PENDING | |
| U63 | The denied list holds the secret mount's host path | FR-018a, C16b | example | PENDING | |

### `crates/micold-core/src/env_include.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U64 | Each of the nine inherited identity keys is matched whatever its value | FR-006 | example | DONE | `crates/micold-core/src/env_include.rs::terminal_identity_tests::each_identity_key_is_matched_whatever_its_value` |
| U65 | `COLORTERM` is matched unless its value is `truecolor` or `24bit` (any ASCII case) | FR-006 | example | DONE | `crates/micold-core/src/env_include.rs::terminal_identity_tests::colorterm_is_matched_unless_it_names_a_colour_depth` |
| U66 | Keys compare exactly when `keys_case_insensitive` is false (`term_program` unmatched) and case-insensitively when true | FR-006 | example | DONE | `crates/micold-core/src/env_include.rs::terminal_identity_tests::keys_compare_exactly_unless_the_platform_folds_their_case` |
| U67 | Given a fixed inherited environment, the Unix `bash` builder and both Windows `powershell.exe` builders remove exactly the matched keys | FR-006 | example | DONE | `crates/micold-core/src/env_include.rs::terminal_identity_tests::each_include_shell_removes_exactly_the_inherited_identity` |

### `crates/micold-daemon/src/supervisor.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U68 | A session spawned by `spawn_shell` or `spawn_ai_cli` does not see inherited `TERM_PROGRAM=WezTerm` or `FORCE_HYPERLINK=1` | FR-006 | example | DONE | `crates/micold-daemon/tests/session_identity_env.rs::a_session_does_not_see_the_inherited_terminal_identity` |
| U69 | `FORCE_HYPERLINK=1` set by the include script is present, also when it was inherited too | FR-006 | example | DONE | `crates/micold-daemon/tests/session_identity_env.rs::the_include_script_can_opt_a_session_into_hyperlinks` |
| U70 | Inherited `COLORTERM=truecolor` is kept | FR-006 | example | DONE | `crates/micold-daemon/tests/session_identity_env.rs::an_inherited_colour_depth_is_kept` |
| U71 | `TERM` is `xterm-256color` as before | FR-006 | example | DONE | `crates/micold-daemon/tests/session_identity_env.rs::term_stays_xterm_256color` |

### `crates/micold-daemon/tests/osc8_passthrough.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U72 | An OSC 8 link printed by a child of the real PTY supervisor reaches the grid cell's hyperlink, on each CI OS (a Windows ignore is recorded with its CI run if ConPTY drops it) | FR-002, spec Edge Cases (terminal layer drops declared hyperlinks) | characterization | DONE | `crates/micold-daemon/tests/osc8_passthrough.rs::an_osc8_link_printed_through_the_pty_reaches_the_grid_cell` (characterization; mutant URI fails it) |

It is a characterization test because it pins what the daemon already does. It must be green against
untouched code, and it is recorded as `BASELINE`.

### `crates/micold-client/src/features/session.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U73 | `LinkActivated` with `Url(u)` emits `OpenLink(Url(u))` | FR-010, T5 | example | DONE | `crates/micold-client/tests/features_session_links.rs::activating_a_url_link_asks_to_open_that_url_verbatim` |
| U74 | `LinkOpenFinished` with `NoApplication` notifies `Couldn't open <address>: no application is set up to open it` | FR-015, T12 | example | DONE | `crates/micold-client/tests/features_session_links.rs::an_open_with_no_application_notifies_that_nothing_is_set_up` |
| U75 | `LinkOpenFinished` with `LaunchFailed(e)` notifies `Couldn't open <address>: <e>` | FR-015, T12 | example | DONE | `crates/micold-client/tests/features_session_links.rs::a_failed_launch_notifies_the_reason` |
| U76 | `LinkOpenFinished` with `Ok` notifies nothing | FR-015, T13 | example | DONE | `crates/micold-client/tests/features_session_links.rs::an_open_that_worked_notifies_nothing` |
| U77 | `LinkOpenFinished` with `NotFound` notifies `Couldn't open <address>: the file doesn't exist on this machine` | FR-015, US3.5 | example | PENDING | |
| U78 | `LinkActivated` with `HostPath(p)` and no confirmation emits `OpenLink(Path { path: p, address })` | FR-010, T6 | example | PENDING | |
| U79 | `LinkActivated` with `Unreachable` notifies `Couldn't open <address>: the sandbox doesn't share that location with this machine` and emits no open | FR-015, FR-018, T8 | example | PENDING | |
| U80 | `LinkActivated` needing confirmation records the pending open and shows the confirm surface with the host path | FR-018a, T7 | example | PENDING | |
| U81 | Confirming while the session exists and the sandbox is live emits `OpenLink(Path)` | FR-018a, T9 | example | PENDING | |
| U82 | Confirming after the session closed notifies `Couldn't open <host path>: the session has closed` and opens nothing | FR-018a, T9 | example | PENDING | |
| U83 | Confirming after the sandbox stopped notifies `Couldn't open <host path>: the sandbox has stopped` and opens nothing | FR-018a, T9 | example | PENDING | |
| U84 | Declining opens nothing and clears the pending open | FR-018a, T10 | example | PENDING | |
| U85 | A menu opened with a link lists **Open Link** and **Copy Link Address** first | FR-020, T14 | example | PENDING | |
| U86 | A menu opened without a link lists today's items only | FR-020, T14 | example | PENDING | |
| U87 | **Open Link** acts as `LinkActivated` on the captured link and closes the menu | FR-017, FR-020, T15 | example | PENDING | |
| U88 | **Copy Link Address** emits `ClipboardWrite` with the captured link's address | FR-021, T16 | example | PENDING | |
| U89 | Closing the menu clears the captured link | FR-017, T17 | example | PENDING | |
| U143 | `link_menu_items` is Open Link then Copy Link Address for a link, and empty for none | FR-020, M1, M2 | example | PENDING | |

### `crates/micold-client/src/features/sandbox.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U90 | `started(started, locations)` makes `locations()` return them, for a created and for an attached container | FR-018, T18 | example | PENDING | |
| U91 | After each exit from `Running`/`Stale` (stopped, failed, container lost, fallback accepted) and after `for_placement`, `locations()` is `None` | FR-018, T19 | example | PENDING | |
| U92 | A `Stale` sandbox still returns its locations | FR-018 | example | PENDING | |

### `crates/micold-client/src/shell/links.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U93 | `LinkActivated` for a URL, through `update_inner`, calls `opener.open` with the URL verbatim | FR-010 | example | DONE | `crates/micold-client/src/shell/links.rs::tests::activating_a_url_through_update_inner_opens_it_verbatim_and_without_delay` |
| U94 | No timer or debounce stands between the `OpenLink` outcome and the opener call | SC-003 | example | DONE | `crates/micold-client/src/shell/links.rs::tests::activating_a_url_through_update_inner_opens_it_verbatim_and_without_delay` |
| U95 | An `OpenLink(Path)` for a missing path finishes with `NotFound` and calls no opener | FR-015, T11 | example | PENDING | |
| U96 | An existing document or folder is passed to `opener.open` | FR-010, T11 | example | PENDING | |
| U97 | On each CI OS, real runnable files of every kind that OS lists (including a symlink to an executable file, the extension-only kinds such as Linux `.desktop`/`.AppImage` and macOS `.command`, and inside a directory whose name has a space) are passed to `reveal` 100%, and `.txt`, `.png`, `.pdf`, `.html` to `open` 100% | FR-013, SC-007 | example | PENDING | |
| U146 | macOS: a real directory without a bundle extension that holds `Contents/Info.plist` is passed to `reveal` | FR-013 | example | PENDING | |
| U98 | `LinkOpenConfirmed` calls the opener when `App.sandbox` is `Running`, and notifies without calling it when the sandbox has stopped | FR-018a | example | PENDING | |
| U99 | `ContextMenuCopyLinkAddress` through `update_inner` returns the clipboard write task rather than dropping it | FR-021 | example | PENDING | |
| U100 | `ContextMenuOpenLink` on a URL link reaches the opener | FR-020 | example | PENDING | |

### `crates/micold-client/src/shell/link_opener.rs`, `shell/capabilities.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U101 | A launcher exiting 0 inside the 2 s window is success | FR-015 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::launch::a_launcher_exiting_zero_inside_the_window_is_success` |
| U102 | Linux: a launcher exiting 3 inside the window is `NoApplication` | FR-015 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::launch::xdg_open_exiting_three_inside_the_window_is_no_application` |
| U103 | A launcher exiting with another non-zero code inside the window is `LaunchFailed` | FR-015 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::launch::another_non_zero_exit_inside_the_window_is_a_launch_failure` |
| U104 | A launcher still running at the end of the window is success and is left running | SC-003 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::launch::a_launcher_still_running_at_the_end_of_the_window_is_success_and_keeps_running` |
| U138 | Linux: when the file manager's select-item call fails, reveal opens the containing folder instead | FR-013 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::linux_reveal_opens_the_folder_when_the_file_manager_cannot_select_the_item` |
| U139 | macOS: an `open` exiting non-zero inside the window is `NoApplication` when its stderr names no application, and `LaunchFailed` otherwise | FR-015 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::macos_open_failing_names_no_application_only_when_stderr_says_so` |
| U144 | Windows: a `ShellExecuteW` return above 32 is success, 31 and 27 are `NoApplication`, and 32 and 2 are `LaunchFailed` | FR-015 | example | DONE | `crates/micold-client/src/shell/link_opener.rs::tests::shell_execute_returns_map_to_success_no_application_and_launch_failure` |
| U105 | `SystemLinkOpener` is named in client `src/` only at its definition and in `Capabilities::real()`, and the definition exists | Constitution (capability seams), FR-019 | example | DONE | `crates/micold-client/tests/no_concrete_implementations.rs::the_system_link_opener_is_named_only_at_its_definition_and_in_capabilities_real` |

### `crates/micold-client/src/ui/material/terminal_pane.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U106 | A command-modified press and release on one cell over a link publishes exactly one `LinkActivated` | FR-004, G1 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_command_press_released_on_its_cell_opens_the_link`, `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_command_click_on_a_link_publishes_one_activation_and_nothing_else` |
| U107 | Moving off the press cell before release publishes no activation and starts a selection at the press cell | FR-004, G2 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::leaving_the_press_cell_turns_the_press_into_a_selection_from_it`, `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_command_press_dragged_off_its_cell_selects_from_the_press_cell` |
| U108 | A plain double or triple click on a link selects and publishes no activation | FR-004, G3 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_plain_press_or_a_repeated_click_is_not_the_gesture` (guard; mutant `command: _` fails it) |
| U109 | A command-modified double click on a link activates once, and its second press counts as a double click that selects | FR-004, G3b | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::plain_drags_and_double_and_triple_clicks_on_links_never_activate` |
| U110 | A plain press on a link publishes no activation | FR-004, G4 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_plain_press_or_a_repeated_click_is_not_the_gesture` (guard; mutant `command: _` fails it) |
| U111 | Under mouse reporting without Shift, a command-modified press is reported to the program and activates nothing | FR-016, G5 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_press_routed_to_the_program_is_not_the_gesture`, `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::under_mouse_reporting_a_command_click_goes_to_the_program` (guards; mutant `routing: _` fails them) |
| U112 | Under mouse reporting with Shift, a command-modified press and release on a link activates it | FR-016, G6 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::under_mouse_reporting_shift_and_command_open_the_link` |
| U113 | A command-modified click on an unfocused pane asks for focus and activates the link | FR-004, G8 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_command_click_on_an_unfocused_pane_focuses_it_and_opens_the_link` |
| U114 | When the link under the pointer changed between press and release, the release-time link opens; when there is none, nothing | FR-017, G9 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::the_release_opens_the_link_under_the_pointer_then_or_nothing`, `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::the_link_at_release_opens_when_the_output_changed_under_the_press` |
| U115 | No gesture step publishes `TerminalBytes`, a selection change or a scroll | FR-014 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_command_click_on_a_link_publishes_one_activation_and_nothing_else` |
| U116 | 100 scripted plain drag, double- and triple-click selections starting on links publish 0 activations | SC-004 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::plain_drags_and_double_and_triple_clicks_on_links_never_activate` |
| U142 | A middle click or wheel over a link behaves as today and publishes no activation | FR-004, G7 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_middle_click_or_wheel_over_a_link_behaves_as_today` (guard; mutant publishing on a middle press fails it) |
| U117 | A right press over a link publishes `TerminalContextMenuOpened` carrying the link resolved at the press; over plain text, `None` | FR-017, FR-020 | example | PENDING | |
| U118 | Hover is recomputed when the pointer moves to another cell | FR-007 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::hover_follows_the_pointer_from_cell_to_cell` |
| U119 | Hover is recomputed on redraw when the grid version moved and the consulted rows' content changed, including an in-place redraw of the same line | FR-007, spec Edge Cases (output moving under the pointer) | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_redraw_after_the_output_changed_under_a_resting_pointer_re_resolves_it` |
| U120 | Hover is reused without re-resolving when the grid version moved but the consulted rows are unchanged | SC-005 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_grid_that_moved_without_touching_the_hovered_rows_keeps_the_hover` |
| U121 | Hover is recomputed when the session shown or the `LinkContext` changes | FR-007 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_switch_of_session_or_context_re_resolves` (guard over `hover_refresh`; mutant ignoring the session fails it) |
| U122 | Under mouse reporting, a link is marked only while Shift is held, and pressing or releasing Shift updates it | FR-016 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::under_mouse_reporting_a_link_is_marked_only_while_shift_is_held` |
| U123 | The mouse interaction is `Pointer` over a followable link only while the link modifier is held, the text pointer over it without the modifier, and unchanged elsewhere | FR-007 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::the_pointer_is_a_hand_over_a_link_only_while_the_link_modifier_is_held` |
| U124 | Hovering and pressing in one pane leaves another pane's hover and press empty, and only the pressed pane activates | FR-022 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::hover_and_press_stay_in_the_pane_under_the_pointer` |
| U125 | The address hint sits bottom-left of the content by default | FR-008 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::the_hint_sits_bottom_left_by_default` |
| U126 | The hint moves top-left when the pointer is within the hint's height of the bottom edge, and stays bottom-left one row above that | FR-008 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::the_hint_moves_top_left_when_the_pointer_nears_the_bottom` |
| U127 | The hint always lies inside the content bounds | FR-008 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::the_hint_always_lies_inside_the_content` |
| U128 | A long address is middle-elided in the label while `ResolvedLink.display` stays complete | FR-008, SC-006 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::a_long_address_is_elided_in_the_middle_and_display_stays_whole` |
| U129 | Hovering a declared run marks exactly that run and resolves the declared address | FR-002, FR-008, US2.1 | example | DONE | `crates/micold-client/src/ui/material/terminal_pane.rs::tests::links::hovering_a_declared_run_marks_exactly_the_run_and_resolves_its_address`, `adjacent_runs_with_different_addresses_are_two_links`, `same_address_runs_apart_are_marked_one_at_a_time`, `address_shaped_text_resolves_to_the_declared_address` |

### `crates/micold-client/src/showcase/samples.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U131 | The terminal sample's grid holds a detected `https://` address and a declared hyperlink run | Constitution (showcase covers visible states), FR-007 | example | DONE | `crates/micold-client/src/showcase/samples.rs::tests::the_terminal_sample_holds_a_detected_address_and_a_declared_link` |

### `crates/micold-client/src/ui/terminal.rs`, `ui/mod.rs`, `main.rs` (glue)

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U134 | The pane's `LinkContext` is sandboxed exactly while the sandbox is `Running`/`Stale`, carrying its locations, denied paths and container-id host names; otherwise it has no sandbox part | FR-018 | example | PENDING | covered at the outer loop by A19 and A14 |
| U136 | Boot fills `app::State.host_names` from `host_names_from(gethostname())` | FR-012 | example | PENDING | glue; the branching is U140, the wiring is covered at the outer loop by A12 |

These are glue (Constitution: glue is covered by the composed tests), so their test is the named
acceptance behavior rather than a unit of their own. AI CLI and Regular Terminal panes are the same
`TerminalPane`, so A1–A6 driving one pane kind covers FR-001's "both pane kinds".

### `crates/micold-client/tests/overlay_*.rs`, `popover_displacement.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U137 | `confirm_link_open` is a registered floating surface: listed in each overlay list, dismissed by Escape and scrim click as a decline, counted among the 10 dialogs | FR-018a | example | PENDING | |

## Invariants and edge cases still to place

None. Every edge case in spec.md maps to a behavior above or to "Out of scope" below:

| Edge case | Behavior |
|---|---|
| Accidental activation during selection | U107, U108, U116 |
| Click to focus | U113 |
| Mouse-reporting programs, and hover under them | U111, U112, U122 |
| Output moving under the pointer | U114, U119 |
| Trailing and enclosing punctuation | U2–U5 |
| Markdown and angle forms | U6, U7 |
| Addresses a program broke itself | U22, U133 |
| Malformed or unsupported addresses | U11, U15, U30, U34 |
| Very long addresses | U23 |
| No handler | U74, U75 |
| Sandboxed sessions | U38–U48, U56–U63 |
| Encoded and Windows paths | U31, U37 |
| Filesystems that mark every file executable, and symlinks | U50, U97 |
| Pending opens | U82, U83 |
| Platforms that drop declared hyperlinks | U72 |
| Multiple panes | U124 |
| Repeated activation | U106 |

## Out of scope

- **FR-009 legibility** (underline colour, cursor and selection still visible in both themes). This
  is a rendered-pixel property, checked by the quickstart §B visual pass, not by a unit.
- **SC-005 frame timing.** It is measured (T079, quickstart §A.3). The structural part, reuse of
  hover without re-resolving, is U120.
- **FR-023 user guide.** Documentation, checked by CI's user-guide gate and by quickstart §B.18.
- **The real browser, mail client and file manager launching.** Platform behaviour outside this
  process. The opener's contract to the OS is U101–U104, and the launch is seen in the visual pass.
- **`localhost` port publishing from a sandbox.** The spec says this feature does not translate or
  publish ports.
- **Characterization baselines for existing selection, focus and mouse reporting.** Not needed:
  `terminal_pane.rs` already carries 44 tests, including double- and triple-click selection, press
  routing under mouse reporting and focus on press, which pin today's behaviour before the gesture
  changes it. `env_include` (`crates/micold-core/tests/env_include*.rs`), the supervisor (11 tests),
  `features::sandbox` (`tests/features_sandbox.rs`) and `features::session`
  (`tests/features_session.rs`) are likewise covered.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` (detected at `cdc473ab`):

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact` (assert on the
  observed `N passed`: a filter matching nothing exits 0)
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Acceptance (real runtime): `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1`
- Coverage: none (no `cargo-llvm-cov`)
- Mutation: none (no `cargo-mutants`); strength is checked by deliberate mutants by hand
- Property: none (no `proptest`)

In-source `#[cfg(test)]` modules run with `scripts/build-lock.sh cargo test -p <crate> --lib <path>`
(or `--bin micold-ai-ide` for `shell/` and `main.rs`).
