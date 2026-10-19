# Data model: Overview cards, readable layout and content (#1120)

Nothing is stored. Every entity is derived per render from the window's state (feature 813
data-model *CardModel*), except `HookActivity.prompt`, which the service holds in memory.

## CardModel (changed, `micold_core::overview`)

| Field | Change | Notes |
|---|---|---|
| `title: String` | replaces `summary` | `overview_text::card_title(issue_title, branch, display_name)`: known issue title, else branch, else display name (FR-006) |
| `display_name`, `branch`, `kind`, `sessions`, `state`, `reason`, `prs`, `progress`, `activity`, `activation` | unchanged | `sessions.1 <= sessions.0` is clamped for display only |

## CardText (new, `micold_core::overview_text`)

What one card says, `CardText::of(&CardModel)`. Validation rules are the contract
[contracts/card-text.md](contracts/card-text.md).

| Field | Type | Rule |
|---|---|---|
| `title` | `String` | `model.title` |
| `title_tooltip` | `String` | `title_tooltip(title, display_name)`: `"<title>\n<display name>"`, or the display name alone when the title is it (FR-006, FR-015) |
| `branch` | `Option<Line>` | `branch_line(title, branch)`: `None` without a branch or when the title is the branch; text = short branch or the branch; tooltip = full branch (FR-007) |
| `sessions` | `String` | `sessions_line(model.sessions)` (FR-008) |
| `progress` | `Option<ProgressRow>` | `progress_row(&model.progress)` (FR-009) |
| `prs` | `Option<(String, Option<String>)>` | `model.prs.footer()` (FR-010) |
| `activity` | `Option<Line>` | `activity_line(model.activity)`: sanitised text or slash sentence, ` · d/t` when a ratio; tooltip = the same text; `None` when nothing readable (FR-011–FR-013) |

`Line { text: String, tooltip: String }`. `ProgressRow { text: String, bar: Option<Ratio> }`.

## HookActivity.prompt (changed rule, `micold_core::hook_activity`)

`Option<String>`: `markup::readable(whole prompt)` clipped to `MAX_TEXT_CHARS`; `None` when the
latest prompt has nothing readable. Transition: every `UserPromptSubmit` replaces it (a readable
prompt → `Some`, an unreadable one → `None`); `apply` returns `true` only when the value changed.

## Palette (changed, client `overview_card`)

Per `CardState`, from `Roles`:

| State | `header` / `on_header` | `body` / `on_body` / `on_body_variant` |
|---|---|---|
| Needs action | `error_container` / `on_error_container` | `surface_container_low` / `on_surface` / `on_surface_variant` (every state) |
| Needs attention | `warning_container` / `on_warning_container` | ″ |
| Working | `primary_container` / `on_primary_container` | ″ |
| Done | `success_container` / `on_success_container` | ″ |
| Idle | `surface_variant` / `on_surface_variant` | ″ |

Invariant (tested): every `on_*` at ≥ 4.5:1 on its fill, both schemes; Idle `header` ≠ `body`.

## Type roles (changed)

| Role | Size / line / weight | Used for |
|---|---|---|
| `TypeRole::CardTitle` → `typography::OVERVIEW_CARD_TITLE` (new, in `typography::SCOPED`) | 17 / 24 / 500 | card title |
| `TypeRole::Body` → `BODY_MEDIUM` | 14 / 20 / 400 | branch, sessions, activity, progress, pull request lines |
| `TypeRole::Label` → `LABEL_MEDIUM` | 12 / 16 / 500 | header state and type words, header glyphs |

## Card height

`overview_card::MIN_HEIGHT = 172.0` (was `HEIGHT`). A grid row's height = `max(MIN_HEIGHT, tallest
card's natural height)`; every card in the row takes it (`cdk::EvenRow`).
