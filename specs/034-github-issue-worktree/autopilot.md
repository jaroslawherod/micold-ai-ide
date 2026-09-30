# Autopilot ledger — 034-github-issue-worktree

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: when creating new worktree add another option github issues. Application should fetch github issues for current project and allow to select one for working with it. The ticket should define an worktree name. The type should be resolved by mapping labels into issues types . The mapping should be global for application for now.
- **Kind**: feature
- **Worktree branch**: feat/allow-to-create-worktree-from-github-issue
- **Started**: 2026-09-29
- **Phase**: 4-milestones
- **Next step**: Run milestone M3 (T033–T036, T094, T076, T037–T042, T077).

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
| M3 | T033–T036, T094, T076, T037–T042, T077 | Search beyond the 1,000 loaded issues via GitHub | — | in-progress |
| M4 | T043–T052, T078–T085 | Issue labels choose the type (default mapping in settings.json) | — | pending |
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
| M3 | M1 conformance B F2 (revisited) | `parse_list_page`'s `errors[]` branch is reached only at exit 0 | Revisited with M3's partial-response rule: a non-zero exit's stdout is used only when the parser accepts it, otherwise stderr is classified, because stderr names SAML and scope refusals that GraphQL types as FORBIDDEN (review A #2, U111). The branch stays the exit-0 path; decline stands. |
| M2 | visual pass D3–D5 | Open list covers the fields below; selected chip lacks an outline; Type menu scrolls after 8 rows | Existing component behaviour (Typeahead overlay, `ToggleChip` selected style, Select menu height) unchanged by this feature. |

## Review rounds

| Milestone | Review | Round | Snapshot | Verdict |
|---|---|---|---|---|
| M3 | A (code-review high) | 1 | 0b3a3df2:7c1231e2 | 10 findings: #1, #2, #4, #6, #7, #10 (doc, double parse) fixed (U107–U111); #3, #5, #8, #9 declined |

## Open escalation

None.

## Follow-ups not done

- SC-001 (form to created worktree under 20 s) not demonstrated in M2's visual pass: the 1,000-issue load alone took ~20 s on a loaded machine with a dev build, and `gh issue list -L 1000` alone took 8 s there. Accepted as a measurement-environment deviation for M2; re-time §B2–B5 on a release build in M6 (T063–T065).
- Quickstart §B5's second attempt with the same ticket and name shows the folder-collision error, not a reuse prompt, because the branch is held by the existing worktree. That is the existing FR-021 path; the reuse prompt itself is covered by A6.
