# Autopilot ledger — 034-github-issue-worktree

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: when creating new worktree add another option github issues. Application should fetch github issues for current project and allow to select one for working with it. The ticket should define an worktree name. The type should be resolved by mapping labels into issues types . The mapping should be global for application for now.
- **Kind**: feature
- **Worktree branch**: feat/allow-to-create-worktree-from-github-issue
- **Started**: 2026-09-29
- **Phase**: 4-milestones
- **Next step**: Run milestone M1 (T001–T009, T093, T010–T017).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #452 | Spec (PR 1) — reviewed: round 1 CHANGES (10 findings, all fixed), round 2 CLEAN (4 MINOR, fixed) | merged | d6c2f33e |
| #459 | Design (PR 2) — plan review r1 CHANGES (18, 17 fixed, F10 declined), r2 CHANGES (4 LOW, fixed); analyze 0 critical/high (fixed); tasks review r1 CHANGES (11, fixed), r2 CHANGES (2 LOW, fixed) | merged | 13967080 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T009, T093, T010–T017 | Issue source core (`gh` locate/load/classify, `name_from_title`) + `RemoteList` RPC, protocol 16; no UI yet | — | in-progress |
| M2 | T018–T031, T066–T075, T032 | 🎯 MVP: **GitHub issue** source in the form — list, search, pick fills ticket/name, create | — | pending |
| M3 | T033–T036, T094, T076, T037–T042, T077 | Search beyond the 1,000 loaded issues via GitHub | — | pending |
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

## Open escalation

None.

## Follow-ups not done

None.
