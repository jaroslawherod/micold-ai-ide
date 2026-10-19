# Contract: card text rules (`micold_core::markup`, `micold_core::overview_text`)

Pure functions; no I/O, no logging (FR-020). Every row below is a core unit test.

## C1. `markup::readable(text: &str) -> Option<String>`

A *tag* is `<`, optional `/`, an ASCII letter, then characters other than `<` up to and including
the next `>`.

| Input | Output |
|---|---|
| `<agent-message from="orchestrator">Continue M2</agent-message>` | `Some("Continue M2")` |
| `<agent-message from="a"><agent-message from="b">Hi` | `Some("Hi")` |
| `<agent-message from="x">` + newline + `Continue M2` + newline + `</agent-message>` | `Some("Continue M2")` |
| `line one` + newline + `  line two` | `Some("line one line two")` |
| `a < b` / `x<3` / `if a<b` | unchanged |
| `<agent-message from="x"></agent-message>` / `   ` / `` | `None` |
| `<<b>a>` (a tag remains after removal) | `None` |
| `a<b and c>d` (`<b and c>` is a tag by the rule above) | `Some("ad")` |

## C2. `overview_text::activity_text(text: &str) -> Option<String>`

`readable`, then the slash-command sentence. Name = `[A-Za-z0-9_:.-]+` after a leading `/`, ended by
whitespace or the end; `speckit-` removed from its front; `-` → space.

| Input | Output |
|---|---|
| `/speckit-autopilot #1117` | `Running autopilot on #1117` |
| `/clear` | `Running clear` |
| `/compact keep the plan` | `Running compact on keep the plan` |
| `/speckit-autopilot <agent-message from="o">#1117</agent-message>` | `Running autopilot on #1117` |
| `/home/user/x` | `/home/user/x` (not a command) |
| `/` alone | `/` |
| `/speckit-` (nothing left after the prefix: the prefix is kept) | `Running speckit` |
| `/etc` (a one-segment path reads as a command, accepted) | `Running etc` |
| `Writing tests` | `Writing tests` |
| only markup | `None` |

## C3. `overview_text::activity_line(Option<&CurrentActivity>) -> Option<Line>`

`activity_text(line)` then ` · done/total` when `ratio` is `Some`; `None` when `activity_text` is
`None` (ratio not shown alone). Tooltip = the same text.

## C4. `overview_text::card_title(issue_title: Option<&str>, branch: Option<&str>, display_name: &str) -> String`

| issue_title | branch | display_name | title |
|---|---|---|---|
| `Some("Overview cards: readable layout")` | `Some("feat/1120_x")` | `feat-1120_x` | `Overview cards: readable layout` |
| `None` | `Some("fix/1115_container")` | `fix-1115_container` | `fix/1115_container` |
| `None` | `None` | `Default` | `Default` |
| `Some("   ")` | `Some("main")` | `d` | `main` (a blank title counts as unknown) |

## C5. `overview_text::short_branch(branch: &str) -> Option<String>`

| Branch | Result |
|---|---|
| `fix/1115_container-daemon-fails-to-start` | `Some("fix/1115")` |
| `fix/1115-foo` | `Some("fix/1115")` |
| `feat-813_worktree-overview` | `Some("feat/813")` |
| `chore/42` | `Some("chore/42")` |
| `main`, `claude/project-thread-7041o6`, `feat/make-branch-selector`, `scratch/023-docs-gate`, `fix/1115_` , `fix/12a` | `None` |

## C6. `overview_text::branch_line(title: &str, branch: Option<&str>) -> Option<Line>`

`None` when `branch` is `None` or equals `title`; else `Line { text: short_branch(b).unwrap_or(b),
tooltip: b }`.

## C7. `overview_text::sessions_line((total, running): (u32, u32)) -> String`

| (total, running) | Line |
|---|---|
| (0, 0) | `No sessions` |
| (1, 1) | `1 session · 1 running` |
| (1, 0) | `1 session` |
| (2, 0) | `2 sessions` |
| (3, 2) | `3 sessions · 2 running` |
| (120, 101) | `120 sessions · 101 running` |
| (2, 5) | `2 sessions · 2 running` (clamped) |

## C8. `overview_text::progress_row(&Progress) -> Option<ProgressRow>`

| phase | ratio | Row |
|---|---|---|
| `None` | `None` | `None` |
| `Some(Implement)` | `Some(6/10)` | `Implement · 6/10`, bar 6/10 |
| `None` | `Some(6/10)` | `6/10`, bar |
| `Some(Plan)` | `None` | `Plan`, no bar |

## C9. `overview::PrDisplay::footer(self) -> Option<(String, Option<String>)>`

| Display | Footer |
|---|---|
| `Hidden` | `None` |
| `Counts{0,0}` | `None` |
| `Counts{1,0}` | `("1 open", None)` |
| `Counts{0,2}` | `("2 merged", None)` |
| `Counts{1,2}` | `("1 open · 2 merged", None)` |
| `Counts{100 capped, 0}` | `("100+ open", None)` |
| `Unknown(r)` | `("—", Some(r.label()))` |

## C10. `HookActivity::apply` on `UserPromptSubmit`

| Prompt | `prompt` after | returns |
|---|---|---|
| multi-line `<agent-message …>` wrapper around `Continue M2` | `Some("Continue M2")` | `true` |
| same again | unchanged | `false` |
| only markup, after a readable one | `None` | `true` |
| 300 readable characters | first 200 characters | `true` |
