# Autopilot ledger — 034-github-issue-worktree

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: when creating new worktree add another option github issues. Application should fetch github issues for current project and allow to select one for working with it. The ticket should define an worktree name. The type should be resolved by mapping labels into issues types . The mapping should be global for application for now.
- **Kind**: feature
- **Worktree branch**: feat/allow-to-create-worktree-from-github-issue
- **Started**: 2026-09-29
- **Phase**: 4-milestones
- **Next step**: Wait for M4's PR CI and merge; then run milestone M5 (T053–T062, T086–T092).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #452 | Spec (PR 1) — reviewed: round 1 CHANGES (10 findings, all fixed), round 2 CLEAN (4 MINOR, fixed) | merged | d6c2f33e |
| #459 | Design (PR 2) — plan review r1 CHANGES (18, 17 fixed, F10 declined), r2 CHANGES (4 LOW, fixed); analyze 0 critical/high (fixed); tasks review r1 CHANGES (11, fixed), r2 CHANGES (2 LOW, fixed) | merged | 13967080 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T009, T093, T010–T017 | Issue source core (`gh` locate/load/classify, `name_from_title`) + `RemoteList` RPC, protocol 16; no UI yet | #460 | merged (454c716c) |
| M2 | T018–T031, T066–T075, T032 | 🎯 MVP: **GitHub issue** source in the form — list, search, pick fills ticket/name, create | #468 | merged (a17e6cde) |
| M3 | T033–T036, T094, T076, T037–T042, T077 | Search beyond the 1,000 loaded issues via GitHub | #472 | merged (691510de) |
| M4 | T043–T052, T078–T085 | Issue labels choose the type (default mapping in settings.json) | #478 | open — rebased onto main (tool-server `tool_server_enabled` merged beside `issue_label_types`), gate green |
| M5 | T053–T062, T086–T092 | Settings → GitHub issues mapping editor | — | pending |
| M6 | T063–T065 | Architecture doc + quickstart §B10, B11, B13 recorded | — | pending |

Cut notes (milestones.md rule 3): US1 is split by layer into M1 (render-free core + RPC, UI unreachable per rule 6; M2 completes it) and M2, plus M3 along AS10. M2 stays whole (~25 tasks, 9 of them outer-loop-green ticks): an acceptance-scenario split would put a list on `main` that cannot be picked from.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | 2-clarify r1 | Which labels does the default label-to-type mapping cover? | Stock type-bearing labels only: bug→fix, enhancement→feat, documentation→docs; other stock labels unmapped | agent-resolved | `gh label list` on origin (stock set); spec.md#FR-021 |
| 2 | 2-clarify r1 | Without a GitHub sign-in, list a public repo's issues anonymously? | No — show "not signed in" (FR-022) | user | AskUserQuestion 2026-09-29 |
| 3 | 2-clarify r1 | Which GitHub hosts are supported? | github.com only (FR-002) | user | AskUserQuestion 2026-09-29 |
| 4 | 2-clarify r1 | Which issues does the picker list? | All open issues, most recently updated first, with search (FR-004) | user | AskUserQuestion 2026-09-29 |
| 5 | 2-clarify r1 | Keep 50-char name cap and 1,000-issue load cap? | Keep both (FR-010, FR-004) | user | AskUserQuestion 2026-09-29 |
| 6 | 2-clarify r1 | What does the issue picker's search cover? | Type-ahead like 021 (keyboard too) over number, title, labels of loaded issues; when the list is capped, also GitHub search for open issues beyond the cap (FR-005, FR-005a) | user | user messages 2026-09-29: "allow to search items", "I mean search issues" |
| 7 | 2-clarify r2 | Show a beyond-cap search result that matched only in its body/comments? | No — held to FR-005's rule (number, title, labels) | agent-resolved | spec.md#FR-005; specs/021-branch-typeahead-search (one matching rule) |
| 8 | 2-clarify r3 | Any critical ambiguity left? | None — clarify round 3 reported no critical ambiguities; Phase 2 done | agent-resolved | spec.md coverage scan, all categories Clear or deferred to plan |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| design | plan r1 F10 | Add the new `micold-client` tests to CI's all-platform test list | Editing `.github/workflows/ci.yml` needs a workflow-scoped token (would be an escalation for a docs PR); every platform-sensitive decision (`locate_gh`, `choose_remote`, `classify`, `run_bounded`) lives in `micold-core`, whose whole suite CI already runs on Linux, macOS and Windows; the client reducer tests are platform-neutral. |
| M1 | code-review A #4 | `load_listing` does not de-duplicate across pages, and `complete` can read true while an issue updated mid-load was skipped | The picker holds a snapshot taken at form open (FR-004, data-model §2); the window is the ~1 s between two page requests, and reopening the form reloads. De-duplicating would hide the drift without closing it; a consistent read needs a GitHub API the plan did not choose. |
| M1 | code-review A #9 | `GitCli::remote_list` repeats `run_git`'s spawn and error formatting | It differs in exit semantics (1 means "no remotes", not a failure); widening `run_git` for one caller changes every other caller's contract. Duplication is eight lines. |
| M1 | conformance B F2 | `parse_list_page`'s `errors[]` branch is reachable only when `gh` exits 0 | Follows contract §3 (a non-zero exit goes to `classify`); `parse_list_page` stays total over any GraphQL answer. Revisit with M3's search rule. |
| M2 | code-review A #3 (part) | Remotes are not asked again after the daemon reconnects while the form is open | The disconnect arm ends `Checking` with "not connected"; closing and reopening the form asks again. A reconnect hook for one open dialog is new behaviour the plan did not name. The no-project half was fixed (U106). |
| M2 | code-review A #8 | `IssuesLoaded` carries `resolved_env`, shell data, through the reducer's `Msg` | T027 and contract §4 specify it: the snapshot resolved on the blocking thread reaches the shell's cache through the same message; the reducer ignores the field. |
| M2 | code-review A #9 | A row pick round-trips through the shell (`IssueRowPicked` then `IssuePicked`) | T024/T027 specify it: the Typeahead reports a row, the shell resolves it with `issue_number_at`, the reducer only sees issue numbers. U105 covers the path. |
| M2 | conformance B F3 / visual pass D1, D2 | Highlight after retyping, and the Issue field keeping the typed query after a pick | Parity with 021's branch picker, which the spec says to follow (FR-005): the same `rematch` re-seats the highlight and the same field keeps the query. A change belongs to both pickers, not to this one. |
| M3 | code-review A #3 | Typed text is not escaped, so `repo:other/x` could pull another repository's issues | Every searched issue is ranked against the whole typed text before it is shown (invariant 5), and a row's text never contains the typed qualifier, so such a hit is dropped (A10 and U70 pin the ranking). Escaping would also change what GitHub matches for ordinary words. |
| M3 | code-review A #5 | A 300 ms debounce still allows more than 30 searches a minute for slow typists | R9 fixes 300 ms; a rate-limit answer is `RateLimited` with Retry and the loaded matches stay. Superseded answers are discarded by seq (FR-007a). |
| M3 | code-review A #8 (MINOR) | "No open issue matches." shows while the search is pending | Contract §2 fixes the empty text; "Searching GitHub…" shows under the field once the search runs. MINOR, not fixed. |
| M3 | code-review A #9 | "Search beyond the loaded issues failed — Couldn't read issues: …" doubles the prefix | Contract §5 specifies exactly this: the same set of messages, prefixed. |
| M3 | code-review A r2 #5 | A whitespace-only keystroke cannot recover a skipped search | With r2 #1 fixed no search is left Pending; a failed one has Retry. |
| M3 | code-review A r2 #6 | Re-seat the highlight by issue inside `rematch_issues` for every caller | A keystroke's re-rank keeps 021's index rule (parity, as M2 declined for the branch picker); only the search answer, which re-ranks without the user acting, re-seats by issue. |
| M3 | code-review A r2 #10 | Cycle log records `build-lock.sh cargo test` instead of a mise task | `.specify/memory/tdd-profile.md` names these single-target commands; `mise run gate` runs the suite. |
| M3 | code-review A r3 #2 | A debounce ending while a create runs or a prompt is up still searches | Round 2 showed the guarded alternative leaves the search Pending for good. The keystroke is the named event (FR-003); the answer only re-ranks a closed list and never touches type, ticket or name, which is what the prompt resolves. Re-arming on return to editing is new wiring the plan did not name. |
| M3 | code-review A r3 #7 | Cycle log records `build-lock.sh cargo test` | As r2 #10. |
| M3 | visual pass D6 | The open result list covers the "Searching GitHub…" / search-failed lines under the field | The same Typeahead overlay behaviour M2 declined as D3; contract §2 places these lines under the picker. Visible once the list closes. Listed under Follow-ups. |
| M3 | M1 conformance B F2 (revisited) | `parse_list_page`'s `errors[]` branch is reached only at exit 0 | Revisited with M3's partial-response rule: a non-zero exit's stdout is used only when the parser accepts it, otherwise stderr is classified, because stderr names SAML and scope refusals that GraphQL types as FORBIDDEN (review A #2, U111). Round 2: an error the answer types exactly (NOT_FOUND → `NoAccess`, RATE_LIMITED) now stands at any exit status, so the `errors[]` branch is live for real `gh`; only an `Other` goes to stderr. F2 closed. |
| M4 | code-review A #1 | An entry dropped on read (unknown type) is gone from the file after the next write | Contract §3 and R10 prescribe the drop: the rest of the document must load, and `ConventionalType` is closed, so the entry has no meaning to keep. |
| M4 | code-review A #3 | `labels(first: 20)` hides a mapped label past the 20th | The query shape is contract github-issue-source.md §2 (research R2); issues with more than 20 labels are outside the spec's cases. |
| M4 | code-review A #4 | `store.load()` on the UI thread at every pick | Contract §4 and FR-014a: the mapping is read at the moment of the pick; `open_settings` reads the same small file the same way. |
| M4 | code-review A #5 | The Settings draft snapshots the mapping at open, so a hand edit made while Settings is open is overwritten on save | The same holds for every Settings field (the draft replaces the client's half whole); M5 makes the mapping editable in the view. Comment corrected. |
| M4 | code-review A #8 | `IssuePicked` carries the mapping rather than a resolved type | Data-model §5 and contract §4 fix the message shape `IssuePicked { number, mapping }`, so the reducer applies FR-013/FR-014 and is unit-testable. |
| M4 | code-review A r2 #1, #2 (MINOR) | A non-list mapping reads as the default and the next save writes the default over it; a list of all-bad entries reads as `[]` | Judged MINOR: the alternative (whole file to `.bak`) loses every setting to keep one malformed value. R10's rule — drop what cannot be read, keep the rest — applied to the field as a whole; `[]` is a list the user wrote. |
| M4 | code-review A r2 #3–#7, #9, #10 | Any parse error drops an entry; `Settings`' other fields not lenient; `Value` needs a self-describing format; `open_settings` hides a `.bak` move; three fallback spellings; cycle log uses `build-lock.sh cargo`; mid-milestone commits did not compile the client tests | MINOR or pre-existing: `Settings` is never sent over postcard; the `.bak`-on-open behaviour predates this feature; the cycle-log commands are the TDD profile's; the fixed commits land in one PR. #8 (doc comment) fixed. |
| M4 | visual pass §B4 note | With the type cleared, no message names the missing type; Create is disabled and the preview hides | That is the form's existing "type required" behaviour, which FR-014 and AS3 say to reuse. |
| M2 | visual pass D3–D5 | Open list covers the fields below; selected chip lacks an outline; Type menu scrolls after 8 rows | Existing component behaviour (Typeahead overlay, `ToggleChip` selected style, Select menu height) unchanged by this feature. |

## Review rounds

| Milestone | Review | Round | Snapshot | Verdict |
|---|---|---|---|---|
| M3 | A (code-review high) | 1 | 0b3a3df2:7c1231e2 | 10 findings: #1, #2, #4, #6, #7, #10 (doc, double parse) fixed (U107–U111); #3, #5, #8, #9 declined |
| M3 | A (code-review high), scoped | 2 | 14dfc916:feb7b70e | 10 findings: #1 (round 1 #7 left a search stuck Pending — reverted, U110 revised), #2/#3/#8 (a typed GraphQL error stands; only `Other` goes to stderr, ListNotFound case restored), #4 (highlight falls back to the clamp), #7 (doc), #9 (branch) fixed; #5, #6, #10 declined |
| M3 | A (code-review high), scoped | 3 (last) | a9a4807b:f7829cb2 | 7 findings: #1 (highlight cleared when its issue is dropped), #3 (GitHub's message kept over a generic `Other`), #4 (scope refusals → `NoAccess`, U112), #5/#6 (typed-error test split out, RATE_LIMITED search case) fixed; #2, #7 declined. No round 4 (limit); review B covers the fix diff. |
| M3 | B (conformance) | 1 | d4f988e7:c1c43e14 | CLEAN (1 MINOR: U70 core half test-after, covered by the reducer's red) |
| M4 | A (code-review high) | 1 | ecb65f49:d5ea6e7f | 8 findings: #2 (non-list mapping sent the file to `.bak`), #6 (`Settings` read strictly), #7 (fallback) fixed (cycle 23); #1, #3, #4, #5, #8 declined |
| M4 | A (code-review high), scoped | 2 | 010fa3f4:1ce11b0e | fixes hold; 10 MINOR/low: #8 (doc) fixed, rest declined. Done. |
| M4 | B (conformance) | 1 | 0c96ddb3:d721635b | CLEAN (1 MINOR: A14/U83 did not assert the label-set type before overriding — fixed); Verify all green |
| M4 | visual pass §B4 | 1 | d721635b | criteria 1–4 PASS (labels on rows, bug→fix, unmapped clears after chore, enhancement→feat) |
| M3 | visual pass §B6 | 1 | c1c43e14 | criteria 1, 3, 4 PASS; 2 partial — the issue beyond the cap appears once and is pickable, "Searching GitHub…" not caught on screen (load ~20); D6 declined |

## Open escalation

None.

## Follow-ups not done

- Visual pass D6 (M3): while the issue list is open, its overlay hides the search status line under the field. A status position that stays visible (above the field, or in the list's empty/footer row) would need a contract §2 change; raise it with M6's §B13 pass.
- SC-001 (form to created worktree under 20 s) not demonstrated in M2's visual pass: the 1,000-issue load alone took ~20 s on a loaded machine with a dev build, and `gh issue list -L 1000` alone took 8 s there. Accepted as a measurement-environment deviation for M2; re-time §B2–B5 on a release build in M6 (T063–T065).
- Quickstart §B5's second attempt with the same ticket and name shows the folder-collision error, not a reuse prompt, because the branch is held by the existing worktree. That is the existing FR-021 path; the reuse prompt itself is covered by A6.
